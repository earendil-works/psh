//! `awk`: POSIX awk with the gawk extensions agents use (`-v`, `-f`, regex and
//! multi-character `-F`, `length(array)`, `tolower`/`toupper`, `gensub`, `match` with an
//! array, `strftime`/`systime`/`mktime`, `printf %*d`, getline from files and
//! commands, `system()` and pipes through psh's shell, `/dev/stderr`).
//!
//! A port of GoAWK (MIT, see NOTICE): lexer and parser to a syntax tree (`lexer`,
//! `parser`, `ast`), scopes and scalar/array types (`resolve`), instructions for a stack
//! machine (`compile`) and the machine (`interp`) with its values, I/O and builtins.
//! Behavior follows POSIX awk; where gawk and onetrueawk differ, gawk (D016).

mod array;
mod ast;
mod builtins;
mod compile;
mod interp;
mod io;
mod lexer;
mod parser;
mod printf;
mod regex;
mod resolve;
mod value;

#[cfg(test)]
mod tests;

use std::rc::Rc;

use psh_engine::host::{OpenOptions, error_text};
use psh_engine::util::UtilCtx;

use crate::common;
use crate::opts::{Spec, both};

use interp::Interp;
use value::Value;

pub const SPEC: Spec = Spec {
    name: "awk",
    opts: &[
        both('F', "field-separator").value(),
        both('f', "file").value(),
        both('v', "assign").value(),
    ],
    posix_order: true,
};

/// A program and where its lines come from, for messages.
struct Source {
    text: Vec<u8>,
    /// (first line in `text`, name) per `-f` file; empty for a command-line program.
    files: Vec<(u32, String)>,
}

impl Source {
    /// `cmd. line:N` or `FILE:N` for a line of the program.
    fn place(&self, line: u32) -> String {
        match self.files.iter().rev().find(|(first, _)| *first <= line) {
            Some((first, name)) => format!("{name}:{}", line - first + 1),
            None => format!("cmd. line:{line}"),
        }
    }

    fn line_text(&self, line: u32) -> &[u8] {
        self.text
            .split(|&b| b == b'\n')
            .nth(line.saturating_sub(1) as usize)
            .unwrap_or(b"")
    }
}

pub async fn run(ctx: &mut UtilCtx<'_>, argv: &[Vec<u8>]) -> i32 {
    let p = match common::parse(ctx, &SPEC, argv).await {
        Ok(p) => p,
        Err(st) => return st,
    };
    let mut operands = p.operands.clone().into_iter();
    let mut fs = None;
    let mut assigns = Vec::new();
    let mut files = Vec::new();
    for (key, value) in p.iter(&SPEC) {
        let value = value.unwrap_or_default().to_vec();
        match key.as_str() {
            "field-separator" => fs = Some(value),
            "assign" => assigns.push(value),
            _ => files.push(value),
        }
    }

    // The program: the -f files in order, else the first operand.
    let source = if files.is_empty() {
        let Some(text) = operands.next() else {
            ctx.error_raw(b"usage: awk [-F fs][-v var=value][prog | -f progfile][file ...]\n")
                .await;
            return 2;
        };
        Source {
            text,
            files: Vec::new(),
        }
    } else {
        let mut text = Vec::new();
        let mut names = Vec::new();
        for f in &files {
            let line = text.iter().filter(|&&b| b == b'\n').count() as u32 + 1;
            let name = String::from_utf8_lossy(f).into_owned();
            let data = if f.as_slice() == b"-" {
                let mut d = Vec::new();
                let _ = ctx.read_to_end(&mut d).await;
                Ok(d)
            } else {
                match ctx.open(f, OpenOptions::read()).await {
                    Ok(file) => {
                        let mut d = Vec::new();
                        file.read_to_end(&mut d).await.map(|()| d)
                    }
                    Err(e) => Err(e),
                }
            };
            match data {
                Ok(d) => {
                    text.extend_from_slice(&d);
                    if !d.ends_with(b"\n") {
                        text.push(b'\n');
                    }
                }
                Err(e) => {
                    ctx.error(&format!(
                        "fatal: cannot open source file `{name}' for reading: {}",
                        error_text(&e)
                    ))
                    .await;
                    return 2;
                }
            }
            names.push((line, name));
        }
        Source { text, files: names }
    };

    let utf8 = ctx.utf8();
    let prog = match build(&source.text, utf8) {
        Ok(p) => Rc::new(p),
        Err(e) => {
            let place = source.place(e.pos.line.max(1));
            let line = source.line_text(e.pos.line.max(1)).to_vec();
            let mut msg = format!("awk: {place}: ").into_bytes();
            msg.extend_from_slice(&line);
            msg.push(b'\n');
            let caret = " ".repeat(e.pos.col.max(1) as usize);
            msg.extend_from_slice(format!("awk: {place}:{caret}^ {}\n", e.msg).as_bytes());
            let _ = ctx.flush().await;
            ctx.error_raw(&msg).await;
            return 1;
        }
    };

    let env: Vec<(Vec<u8>, Vec<u8>)> = ctx.env().to_vec();
    let mut m = Interp::new(ctx, prog);
    if let Some(a) = m.global_array(b"ARGV") {
        let mut a = a.borrow_mut();
        a.set(Rc::from(&b"0"[..]), Value::str(b"awk"));
        let mut n = 1;
        for op in operands {
            a.set(Rc::from(n.to_string().as_bytes()), Value::strnum(&op));
            n += 1;
        }
        m.argc = n as f64;
    }
    if let Some(e) = m.global_array(b"ENVIRON") {
        let mut e = e.borrow_mut();
        for (k, v) in env {
            e.set(Rc::from(k.as_slice()), Value::strnum(&v));
        }
    }
    if let Some(fs) = fs {
        // `-F t` is a tab (POSIX); escapes as in a string.
        let fs = if fs == b"t" {
            b"\t".to_vec()
        } else {
            lexer::unescape(&fs)
        };
        if let Err(interp::Flow::Fatal(msg)) = m.set_special(resolve::Special::Fs, Value::str(&fs))
        {
            m.ctx.error(&format!("fatal: {msg}")).await;
            return 2;
        }
    }
    for a in assigns {
        let Some((name, value)) = interp::split_assignment(&a) else {
            m.ctx
                .error(&format!(
                    "fatal: `{}' argument to `-v' not in `var=value' form",
                    String::from_utf8_lossy(&a)
                ))
                .await;
            return 2;
        };
        let value = lexer::unescape(value);
        if let Err(interp::Flow::Fatal(msg)) = m.assign_var(name, &value) {
            m.ctx.error(&format!("fatal: {msg}")).await;
            return 2;
        }
    }
    m.run_program().await
}

/// Source to a runnable program.
fn build(src: &[u8], utf8: bool) -> Result<compile::Program, parser::ParseError> {
    let ast = parser::parse(src)?;
    let res = resolve::resolve(&ast)?;
    let re = |s: &[u8]| regex::compile(s, utf8);
    compile::compile(&ast, &res, &re)
}
