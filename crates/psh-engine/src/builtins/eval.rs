//! `eval` and `source`/`.`: run text as shell code in the current vproc.

use std::rc::Rc;
use std::sync::Arc;

use psh_syntax::{LineIndex, Options, Parser};

use crate::exec;
use crate::host::{FileKind, OpenOptions, error_text};
use crate::state::{Code, R, Unwind, Vp};

use super::s;

/// Where text to run comes from, for messages about its syntax errors.
pub(crate) enum Origin {
    /// `NAME: eval: line N: …`
    Eval,
    /// `FILE: line N: …` (the file being sourced is already the message name).
    File,
}

/// Parses and runs `text` one top-level command line at a time, as the script itself
/// is run: a syntax error stops at that line (status 2) after the lines before it ran.
/// `line_base` is added to line numbers. Returns the status of the last command run (0
/// for none).
pub(crate) async fn run_text(vp: &mut Vp, text: &[u8], line_base: u32, origin: Origin) -> R {
    let lines = Rc::new(LineIndex::from_bytes(text));
    let mut parser = Parser::from_bytes(text, Options::default());
    let mut status = 0;
    while let Some(chunk) = parser.next_chunk() {
        match chunk {
            Ok(ast) => {
                let root = ast.root();
                let code = Rc::new(Code {
                    ast: Arc::new(ast),
                    lines: lines.clone(),
                    line_base,
                });
                status = exec::run_list(vp, &code, root).await?;
                vp.status = status;
            }
            Err(e) => {
                let line = lines.line(e.span.start) + line_base;
                let prefix = match origin {
                    Origin::Eval => format!("{}: eval: line {line}: ", vp.msg_name()),
                    Origin::File => format!("{}: line {line}: ", vp.msg_name()),
                };
                let mut msg = format!("{prefix}{}\n", e.message).into_bytes();
                if e.message.starts_with("syntax error near") {
                    let n = (line - line_base) as usize;
                    let l = text.split(|&b| b == b'\n').nth(n - 1).unwrap_or(b"");
                    msg.extend_from_slice(prefix.as_bytes());
                    msg.push(b'`');
                    msg.extend_from_slice(l.strip_suffix(b"\r").unwrap_or(l));
                    msg.extend_from_slice(b"'\n");
                }
                vp.error_raw(&msg).await;
                return Ok(2);
            }
        }
    }
    Ok(status)
}

pub(super) async fn eval(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let mut rest = &args[1..];
    match rest.first() {
        Some(a) if a == b"--" => rest = &rest[1..],
        Some(a) if a.len() > 1 && a[0] == b'-' => {
            vp.error(&format!("eval: {}: invalid option", s(&a[..2])))
                .await;
            vp.error_raw(b"eval: usage: eval [arg ...]\n").await;
            return Ok(2);
        }
        _ => {}
    }
    // Ends with a newline, as bash's eval text does: an unexpected end of file is
    // reported on the line after the last.
    let mut text = rest.join(&b' ');
    text.push(b'\n');
    let line_base = vp.line.saturating_sub(1);
    run_text(vp, &text, line_base, Origin::Eval).await
}

/// `source FILE [ARGS]` and `. FILE [ARGS]`.
pub(super) async fn source(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let name = s(&args[0]).into_owned();
    let usage = format!("{name}: usage: {name} [-p path] filename [arguments]\n");
    let mut i = 1;
    let mut search: Option<Vec<u8>> = None;
    while let Some(a) = args.get(i) {
        if a == b"--" {
            i += 1;
            break;
        }
        if a.len() < 2 || a[0] != b'-' {
            break;
        }
        if a == b"-p" {
            let Some(p) = args.get(i + 1) else {
                vp.error(&format!("{name}: -p: option requires an argument"))
                    .await;
                vp.error_raw(usage.as_bytes()).await;
                return Ok(2);
            };
            search = Some(p.clone());
            i += 2;
            continue;
        }
        vp.error(&format!("{name}: {}: invalid option", s(&a[..2])))
            .await;
        vp.error_raw(usage.as_bytes()).await;
        return Ok(2);
    }
    let Some(file) = args.get(i) else {
        vp.error(&format!("{name}: filename argument required"))
            .await;
        vp.error_raw(usage.as_bytes()).await;
        return Ok(2);
    };
    let path = match find_file(vp, file, search.as_deref()).await {
        Some(p) => p,
        None => {
            vp.error(&format!("{}: No such file or directory", s(file)))
                .await;
            return Ok(1);
        }
    };
    let text = match read_file(vp, &path).await {
        Ok(t) => t,
        Err(None) => {
            vp.error(&format!("{name}: {}: is a directory", s(file)))
                .await;
            return Ok(1);
        }
        Err(Some(msg)) => {
            vp.error(&format!("{}: {msg}", s(file))).await;
            return Ok(1);
        }
    };

    let new_args = (args.len() > i + 1).then(|| args[i + 1..].to_vec());
    let saved_args = match new_args {
        Some(a) => Some(std::mem::replace(&mut vp.args, a)),
        None => None,
    };
    vp.sourced.push(s(file).into_owned());
    let r = Box::pin(run_text(vp, &text, 0, Origin::File)).await;
    vp.sourced.pop();
    if let Some(a) = saved_args {
        vp.args = a;
    }
    match r {
        Err(Unwind::Return(st)) => Ok(st),
        r => r,
    }
}

/// The file `source` reads: a name with a slash as it is; else the first regular file
/// of that name in `PATH` (or `-p`'s list), then the current directory.
async fn find_file(vp: &Vp, file: &[u8], search: Option<&[u8]>) -> Option<Vec<u8>> {
    if file.contains(&b'/') {
        return Some(vp.abs_path(file));
    }
    let path = match search {
        Some(p) => Some(p.to_vec()),
        None => vp.var("PATH").map(<[u8]>::to_vec),
    };
    if let Some(path) = path {
        for dir in path.split(|&b| b == b':') {
            let mut cand = if dir.is_empty() {
                vp.cwd.clone()
            } else {
                vp.abs_path(dir)
            };
            if cand.last() != Some(&b'/') {
                cand.push(b'/');
            }
            cand.extend_from_slice(file);
            if let Ok(m) = vp.host().stat(&cand, true).await
                && m.kind != FileKind::Dir
            {
                return Some(cand);
            }
        }
    }
    // `-p` replaces the search entirely; without it the cwd comes last.
    if search.is_some() {
        return None;
    }
    let cand = vp.abs_path(file);
    match vp.host().stat(&cand, true).await {
        Ok(_) => Some(cand),
        Err(_) => None,
    }
}

/// The whole file; `Err(None)` for a directory. `/dev/stdin` and `/dev/fd/N` are the
/// vproc's own descriptors (`source /dev/stdin <<EOF`).
async fn read_file(vp: &Vp, path: &[u8]) -> Result<Vec<u8>, Option<String>> {
    let fd = match crate::exec::dev_fd(path) {
        Some(n) => vp
            .fd(n)
            .ok_or_else(|| Some("Bad file descriptor".to_string()))?,
        None => {
            if let Ok(m) = vp.host().stat(path, true).await
                && m.kind == FileKind::Dir
            {
                return Err(None);
            }
            let f = vp
                .host()
                .open(path, OpenOptions::read())
                .await
                .map_err(|e| Some(error_text(&e)))?;
            Rc::new(crate::fd::FdObj::File(f))
        }
    };
    let mut out = Vec::new();
    fd.read_to_end(&mut out)
        .await
        .map_err(|e| Some(error_text(&e)))?;
    Ok(out)
}
