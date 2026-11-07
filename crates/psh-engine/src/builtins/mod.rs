//! Shell builtins (Tier 0, ARCH.md): part of the engine, run on the vproc that calls
//! them.

mod printf;
mod test;

use crate::escape::{Dialect, decode};
use crate::host::{FileKind, error_text};
use crate::sched::LocalBoxFuture;
use crate::state::{R, Unwind, Vp, is_name};

pub(crate) type BuiltinFn = for<'a> fn(&'a mut Vp, &'a [Vec<u8>]) -> LocalBoxFuture<'a, R>;

#[derive(Clone, Copy)]
pub(crate) struct Builtin {
    pub run: BuiltinFn,
    /// A POSIX special builtin: assignments in front of it stay.
    pub special: bool,
}

macro_rules! builtin {
    ($f:path) => {{
        fn wrap<'a>(vp: &'a mut Vp, args: &'a [Vec<u8>]) -> LocalBoxFuture<'a, R> {
            Box::pin($f(vp, args))
        }
        wrap as BuiltinFn
    }};
}

pub(crate) fn find(name: &[u8]) -> Option<Builtin> {
    let (run, special) = match name {
        b":" => (builtin!(colon), true),
        b"true" => (builtin!(colon), false),
        b"false" => (builtin!(false_), false),
        b"echo" => (builtin!(echo), false),
        b"printf" => (builtin!(printf::printf), false),
        b"cd" => (builtin!(cd), false),
        b"pwd" => (builtin!(pwd), false),
        b"export" => (builtin!(export), true),
        b"readonly" => (builtin!(readonly), true),
        b"unset" => (builtin!(unset), true),
        b"exit" => (builtin!(exit), true),
        b"set" => (builtin!(set), true),
        b"shift" => (builtin!(shift), true),
        b"return" => (builtin!(return_), true),
        b"break" => (builtin!(break_), true),
        b"continue" => (builtin!(continue_), true),
        b"local" => (builtin!(local), false),
        b"let" => (builtin!(let_), false),
        b"getopts" => (builtin!(getopts), false),
        b"test" => (builtin!(test::test), false),
        b"[" => (builtin!(test::bracket), false),
        _ => return None,
    };
    Some(Builtin { run, special })
}

/// Whether `name` is a declaration builtin, whose `name=value` arguments are expanded
/// like assignments (no word splitting).
pub(crate) fn is_declaration(name: &[u8]) -> bool {
    matches!(
        name,
        b"export" | b"readonly" | b"declare" | b"typeset" | b"local"
    )
}

/// Writes to a descriptor of the vproc. A closed pipe ends the vproc as SIGPIPE would
/// (status 141); other errors print `NAME: write error: …` and return false.
pub(crate) async fn write_fd(vp: &Vp, fd: u32, name: &str, data: &[u8]) -> Result<bool, Unwind> {
    let Some(f) = vp.fd(fd) else {
        vp.error(&format!("{name}: write error: Bad file descriptor"))
            .await;
        return Ok(false);
    };
    match f.write_all(data).await {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
            Err(Unwind::Exit(128 + crate::host::SIGPIPE))
        }
        Err(e) => {
            vp.error(&format!("{name}: write error: {}", error_text(&e)))
                .await;
            Ok(false)
        }
    }
}

pub(crate) async fn out(vp: &Vp, name: &str, data: &[u8]) -> R {
    Ok(if write_fd(vp, 1, name, data).await? {
        0
    } else {
        1
    })
}

fn s(b: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(b)
}

async fn colon(_vp: &mut Vp, _args: &[Vec<u8>]) -> R {
    Ok(0)
}

async fn false_(_vp: &mut Vp, _args: &[Vec<u8>]) -> R {
    Ok(1)
}

async fn echo(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let mut newline = true;
    let mut escapes = false;
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if a.len() < 2 || a[0] != b'-' || !a[1..].iter().all(|c| b"neE".contains(c)) {
            break;
        }
        for c in &a[1..] {
            match c {
                b'n' => newline = false,
                b'e' => escapes = true,
                _ => escapes = false,
            }
        }
        i += 1;
    }
    let mut buf = Vec::new();
    for (k, a) in args[i..].iter().enumerate() {
        if k > 0 {
            buf.push(b' ');
        }
        if escapes {
            let d = decode(a, Dialect::Echo);
            buf.extend_from_slice(&d.bytes);
            if d.stop {
                return out(vp, "echo", &buf).await;
            }
        } else {
            buf.extend_from_slice(a);
        }
    }
    if newline {
        buf.push(b'\n');
    }
    out(vp, "echo", &buf).await
}

/// Normalizes `.` and `..` in an absolute path, lexically (`cd -L`).
pub(crate) fn normalize(path: &[u8]) -> Vec<u8> {
    let mut parts: Vec<&[u8]> = Vec::new();
    for c in path.split(|&b| b == b'/') {
        match c {
            b"" | b"." => {}
            b".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    let mut out = Vec::new();
    for p in parts {
        out.push(b'/');
        out.extend_from_slice(p);
    }
    if out.is_empty() {
        out.push(b'/');
    }
    out
}

async fn cd(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let mut physical = false;
    let mut i = 1;
    while i < args.len() && args[i].len() > 1 && args[i][0] == b'-' {
        match &args[i][..] {
            b"--" => {
                i += 1;
                break;
            }
            b"-L" => physical = false,
            b"-P" => physical = true,
            b"-e" | b"-@" => {}
            _ => break,
        }
        i += 1;
    }
    let rest = &args[i..];
    if rest.len() > 1 {
        vp.error("cd: too many arguments").await;
        return Ok(1);
    }
    let mut print = false;
    let dir = match rest.first() {
        None => match vp.var("HOME") {
            Some(h) => h.to_vec(),
            None => {
                vp.error("cd: HOME not set").await;
                return Ok(1);
            }
        },
        Some(d) if d == b"-" => match vp.var("OLDPWD") {
            Some(o) => {
                print = true;
                o.to_vec()
            }
            None => {
                vp.error("cd: OLDPWD not set").await;
                return Ok(1);
            }
        },
        Some(d) => d.clone(),
    };
    if dir.is_empty() {
        return Ok(0);
    }
    let mut target = normalize(&vp.abs_path(&dir));
    match vp.host().stat(&target, true).await {
        Ok(m) if m.kind == FileKind::Dir => {}
        Ok(_) => {
            vp.error(&format!("cd: {}: Not a directory", s(&dir))).await;
            return Ok(1);
        }
        Err(e) => {
            vp.error(&format!("cd: {}: {}", s(&dir), error_text(&e)))
                .await;
            return Ok(1);
        }
    }
    if physical && let Ok(p) = vp.host().canonicalize(&target).await {
        target = p;
    }
    let old = vp.cwd.clone();
    vp.set_var("OLDPWD", old);
    vp.set_var("PWD", target.clone());
    vp.cwd = target;
    if print {
        let mut line = vp.cwd.clone();
        line.push(b'\n');
        return out(vp, "cd", &line).await;
    }
    Ok(0)
}

async fn pwd(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let physical = args[1..].iter().any(|a| a == b"-P");
    let mut dir = vp.cwd.clone();
    if physical && let Ok(p) = vp.host().canonicalize(&dir).await {
        dir = p;
    }
    dir.push(b'\n');
    out(vp, "pwd", &dir).await
}

/// `"value"` with `"`, `\`, `$` and `` ` `` escaped, as `declare -p` prints it.
fn dquote(v: &[u8]) -> Vec<u8> {
    let mut out = vec![b'"'];
    for &b in v {
        if matches!(b, b'"' | b'\\' | b'$' | b'`') {
            out.push(b'\\');
        }
        out.push(b);
    }
    out.push(b'"');
    out
}

async fn print_vars(
    vp: &Vp,
    name: &str,
    flag: &str,
    filter: impl Fn(&crate::state::Var) -> bool,
) -> R {
    let mut vars: Vec<_> = vp.vars.iter().filter(|(_, v)| filter(v)).collect();
    vars.sort_by(|a, b| a.0.cmp(b.0));
    let mut buf = Vec::new();
    for (n, v) in vars {
        buf.extend_from_slice(format!("declare -{flag} {n}").as_bytes());
        if let Some(val) = &v.value {
            buf.push(b'=');
            buf.extend_from_slice(&dquote(val));
        }
        buf.push(b'\n');
    }
    out(vp, name, &buf).await
}

/// `export` and `readonly`: set an attribute, with optional values.
async fn declare_attr(vp: &mut Vp, args: &[Vec<u8>], name: &str, readonly: bool) -> R {
    let mut remove = false;
    let mut i = 1;
    let mut print = false;
    while i < args.len() && args[i].first() == Some(&b'-') && args[i].len() > 1 {
        if args[i] == b"--" {
            i += 1;
            break;
        }
        for c in &args[i][1..] {
            match c {
                b'n' if !readonly => remove = true,
                b'p' => print = true,
                b'f' | b'a' | b'A' => {}
                _ => {
                    vp.error(&format!("{name}: -{}: invalid option", *c as char))
                        .await;
                    return Ok(2);
                }
            }
        }
        i += 1;
    }
    if i == args.len() {
        let _ = print;
        let flag = if readonly { "r" } else { "x" };
        return print_vars(vp, name, flag, |v| {
            if readonly { v.readonly } else { v.exported }
        })
        .await;
    }
    let mut status = 0;
    for a in &args[i..] {
        let (n, value) = match a.iter().position(|&b| b == b'=') {
            Some(eq) => {
                let (n, append) = match a[..eq].strip_suffix(b"+") {
                    Some(n) => (n, true),
                    None => (&a[..eq], false),
                };
                let mut v = a[eq + 1..].to_vec();
                if append
                    && let Ok(nm) = std::str::from_utf8(n)
                    && let Some(old) = vp.var(nm)
                {
                    let mut o = old.to_vec();
                    o.extend_from_slice(&v);
                    v = o;
                }
                (n, Some(v))
            }
            None => (&a[..], None),
        };
        if !is_name(n) {
            vp.error(&format!("{name}: `{}': not a valid identifier", s(a)))
                .await;
            status = 1;
            continue;
        }
        let n = std::str::from_utf8(n).unwrap_or_default();
        let var = vp.vars.entry(n);
        if value.is_some() && var.readonly {
            vp.error(&format!("{n}: readonly variable")).await;
            status = 1;
            continue;
        }
        if let Some(v) = value {
            var.value = Some(v);
        }
        if readonly {
            var.readonly = true;
        } else {
            var.exported = !remove;
        }
    }
    Ok(status)
}

async fn export(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    declare_attr(vp, args, "export", false).await
}

async fn readonly(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    declare_attr(vp, args, "readonly", true).await
}

async fn unset(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let mut i = 1;
    let (mut funcs, mut vars_only) = (false, false);
    while i < args.len() && args[i].first() == Some(&b'-') && args[i].len() > 1 {
        if args[i] == b"--" {
            i += 1;
            break;
        }
        if !args[i][1..].iter().all(|c| b"fvn".contains(c)) {
            vp.error(&format!("unset: {}: invalid option", s(&args[i][..2])))
                .await;
            return Ok(2);
        }
        funcs |= args[i].contains(&b'f');
        vars_only |= args[i].contains(&b'v');
        i += 1;
    }
    let mut status = 0;
    for a in &args[i..] {
        let n = std::str::from_utf8(a).unwrap_or_default();
        // `-f`, or no option and no such variable: the function.
        if (funcs && !vars_only)
            || (!vars_only && vp.vars.get(n).is_none() && vp.funcs.contains_key(n))
        {
            std::rc::Rc::make_mut(&mut vp.funcs).remove(n);
            continue;
        }
        if !is_name(a) {
            vp.error(&format!("unset: `{}': not a valid identifier", s(a)))
                .await;
            status = 1;
            continue;
        }
        if vp.vars.get(n).is_some_and(|v| v.readonly) {
            vp.error(&format!("unset: {n}: cannot unset: readonly variable"))
                .await;
            status = 1;
            continue;
        }
        vp.vars.remove(n);
    }
    Ok(status)
}

/// Parses a decimal status argument the way `exit`, `return` and `shift` do.
fn parse_int(a: &[u8]) -> Option<i64> {
    let t = std::str::from_utf8(a).ok()?.trim();
    t.parse::<i64>().ok()
}

async fn exit(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let mut rest = &args[1..];
    if rest.first().is_some_and(|a| a == b"--") {
        rest = &rest[1..];
    }
    match rest {
        [] => Err(Unwind::Exit(vp.status)),
        [a] => match parse_int(a) {
            Some(n) => Err(Unwind::Exit((n & 0xff) as i32)),
            None => {
                vp.error(&format!("exit: {}: numeric argument required", s(a)))
                    .await;
                Err(Unwind::Exit(2))
            }
        },
        _ => {
            vp.error("exit: too many arguments").await;
            Ok(1)
        }
    }
}

async fn set(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let i = 1;
    if i < args.len() {
        let a = &args[i];
        if a == b"--" {
            vp.args = args[i + 1..].to_vec();
            return Ok(0);
        }
        if a == b"-" {
            vp.args = args[i + 1..].to_vec();
            return Ok(0);
        }
        if (a.first() == Some(&b'-') || a.first() == Some(&b'+')) && a.len() > 1 {
            return crate::expand::unsupported(vp, &format!("set {}", s(a))).await;
        }
        vp.args = args[i..].to_vec();
        return Ok(0);
    }
    if args.len() == 1 {
        let mut vars: Vec<_> = vp.vars.iter().filter(|(_, v)| v.value.is_some()).collect();
        vars.sort_by(|a, b| a.0.cmp(b.0));
        let mut buf = Vec::new();
        for (n, v) in vars {
            buf.extend_from_slice(n.as_bytes());
            buf.push(b'=');
            buf.extend_from_slice(&shell_quote(v.value.as_deref().unwrap_or_default()));
            buf.push(b'\n');
        }
        return out(vp, "set", &buf).await;
    }
    Ok(0)
}

async fn return_(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    if vp.frames.is_empty() {
        vp.error("return: can only `return' from a function or sourced script")
            .await;
        return Ok(2);
    }
    let mut rest = &args[1..];
    if rest.first().is_some_and(|a| a == b"--") {
        rest = &rest[1..];
    }
    match rest.first() {
        None => Err(Unwind::Return(vp.status)),
        Some(a) => match parse_int(a) {
            Some(n) => Err(Unwind::Return((n & 0xff) as i32)),
            None => {
                vp.error(&format!("return: {}: numeric argument required", s(a)))
                    .await;
                Err(Unwind::Return(2))
            }
        },
    }
}

async fn break_(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    loop_control(vp, args, "break").await
}

async fn continue_(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    loop_control(vp, args, "continue").await
}

async fn loop_control(vp: &mut Vp, args: &[Vec<u8>], name: &str) -> R {
    let n = match args.get(1) {
        None => 1,
        Some(a) => match parse_int(a) {
            Some(n) if n >= 1 => n,
            Some(_) => {
                vp.error(&format!("{name}: {}: loop count out of range", s(a)))
                    .await;
                return Ok(1);
            }
            None => {
                // A usage error of a special builtin ends the shell.
                vp.error(&format!("{name}: {}: numeric argument required", s(a)))
                    .await;
                return Err(Unwind::Exit(2));
            }
        },
    };
    if vp.loops == 0 {
        vp.error(&format!(
            "{name}: only meaningful in a `for', `while', or `until' loop"
        ))
        .await;
        return Ok(0);
    }
    let n = n.min(i64::from(vp.loops)) as u32;
    Err(if name == "break" {
        Unwind::Break(n)
    } else {
        Unwind::Continue(n)
    })
}

async fn local(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    if vp.frames.is_empty() {
        vp.error("local: can only be used in a function").await;
        return Ok(1);
    }
    let mut i = 1;
    let (mut readonly, mut export) = (false, false);
    while i < args.len() && args[i].len() > 1 && (args[i][0] == b'-' || args[i][0] == b'+') {
        if args[i] == b"--" {
            i += 1;
            break;
        }
        for c in &args[i][1..] {
            match c {
                b'r' => readonly = args[i][0] == b'-',
                b'x' => export = args[i][0] == b'-',
                _ => {
                    return crate::expand::unsupported(vp, &format!("local -{}", *c as char)).await;
                }
            }
        }
        i += 1;
    }
    let mut status = 0;
    for a in &args[i..] {
        let (n, value) = match a.iter().position(|&b| b == b'=') {
            Some(eq) => (&a[..eq], Some(a[eq + 1..].to_vec())),
            None => (&a[..], None),
        };
        let (n, append) = match n.strip_suffix(b"+") {
            Some(n) if value.is_some() => (n, true),
            _ => (n, false),
        };
        if !is_name(n) {
            vp.error(&format!("local: `{}': not a valid identifier", s(a)))
                .await;
            status = 1;
            continue;
        }
        let n = std::str::from_utf8(n).unwrap_or_default().to_string();
        if vp.vars.get(&n).is_some_and(|v| v.readonly) {
            vp.error(&format!("local: {n}: readonly variable")).await;
            status = 1;
            continue;
        }
        let fresh = vp
            .frames
            .last()
            .is_some_and(|f| !f.saved.iter().any(|(s, _)| *s == n));
        vp.make_local(&n);
        let old = vp.var(&n).map(<[u8]>::to_vec);
        let var = vp.vars.entry(&n);
        if fresh {
            *var = crate::state::Var::default();
        }
        match value {
            Some(v) if append => {
                let mut o = old.unwrap_or_default();
                o.extend_from_slice(&v);
                var.value = Some(o);
            }
            Some(v) => var.value = Some(v),
            None => {}
        }
        var.readonly |= readonly;
        var.exported |= export;
    }
    Ok(status)
}

async fn let_(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    if args.len() < 2 {
        vp.error("let: expression expected").await;
        return Ok(1);
    }
    let mut last = 0;
    for a in &args[1..] {
        match crate::arith::eval(vp, a) {
            Ok(v) => last = v,
            Err(msg) => {
                vp.error(&format!("let: {msg}")).await;
                return Ok(1);
            }
        }
    }
    Ok(i32::from(last == 0))
}

async fn getopts(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let args = match args.get(1) {
        Some(a) if a == b"--" => {
            let mut v = args.to_vec();
            v.remove(1);
            v
        }
        Some(a) if a.len() > 1 && a[0] == b'-' => {
            vp.error(&format!("getopts: {}: invalid option", s(&a[..2])))
                .await;
            vp.error_raw(b"getopts: usage: getopts optstring name [arg ...]\n")
                .await;
            return Ok(2);
        }
        _ => args.to_vec(),
    };
    if args.len() < 3 {
        vp.error("getopts: usage: getopts optstring name [arg ...]")
            .await;
        return Ok(2);
    }
    let spec = args[1].clone();
    let Some(name) = std::str::from_utf8(&args[2])
        .ok()
        .filter(|n| is_name(n.as_bytes()))
        .map(str::to_string)
    else {
        vp.error(&format!(
            "getopts: `{}': not a valid identifier",
            s(&args[2])
        ))
        .await;
        return Ok(1);
    };
    let (silent, spec) = match spec.strip_prefix(b":") {
        Some(rest) => (true, rest.to_vec()),
        None => (false, spec),
    };
    let params: Vec<Vec<u8>> = if args.len() > 3 {
        args[3..].to_vec()
    } else {
        vp.args.clone()
    };
    let mut optind = vp.var("OPTIND").and_then(parse_int).unwrap_or(1).max(1);
    let mut pos = if vp.getopts.0 == optind {
        vp.getopts.1
    } else {
        0
    };
    let end = |vp: &mut Vp, optind: i64| {
        vp.set_var(&name, b"?".to_vec());
        vp.vars.remove("OPTARG");
        vp.set_var("OPTIND", optind.to_string().into_bytes());
        vp.getopts = (optind, 0);
        Ok(1)
    };
    let Some(arg) = params.get(optind as usize - 1).cloned() else {
        return end(vp, optind);
    };
    if pos >= arg.len() {
        pos = 0;
    }
    if pos == 0 {
        if arg == b"--" {
            return end(vp, optind + 1);
        }
        if arg.len() < 2 || arg[0] != b'-' {
            return end(vp, optind);
        }
        pos = 1;
    }
    let c = arg[pos];
    pos += 1;
    let at_end = pos >= arg.len();
    let found = spec.iter().position(|&x| x == c && x != b':');
    let mut value = vec![c];
    let mut optarg: Option<Vec<u8>> = None;
    let next = |optind: &mut i64, pos: &mut usize| {
        if *pos >= arg.len() {
            *optind += 1;
            *pos = 0;
        }
    };
    match found {
        Some(i) if spec.get(i + 1) == Some(&b':') => {
            if !at_end {
                optarg = Some(arg[pos..].to_vec());
                optind += 1;
                pos = 0;
            } else if let Some(v) = params.get(optind as usize) {
                optarg = Some(v.clone());
                optind += 2;
                pos = 0;
            } else {
                optind += 1;
                pos = 0;
                if silent {
                    value = b":".to_vec();
                    optarg = Some(vec![c]);
                } else {
                    let msg = format!(
                        "{}: option requires an argument -- {}\n",
                        vp.core.msg_name.borrow(),
                        c as char
                    );
                    vp.error_raw(msg.as_bytes()).await;
                    value = b"?".to_vec();
                }
            }
        }
        Some(_) => next(&mut optind, &mut pos),
        None => {
            next(&mut optind, &mut pos);
            if silent {
                optarg = Some(vec![c]);
            } else {
                let msg = format!(
                    "{}: illegal option -- {}\n",
                    vp.core.msg_name.borrow(),
                    c as char
                );
                vp.error_raw(msg.as_bytes()).await;
            }
            value = b"?".to_vec();
        }
    }
    vp.set_var(&name, value);
    match optarg {
        Some(v) => vp.set_var("OPTARG", v),
        None => {
            vp.vars.remove("OPTARG");
        }
    }
    vp.set_var("OPTIND", optind.to_string().into_bytes());
    vp.getopts = (optind, pos);
    Ok(0)
}

async fn shift(vp: &mut Vp, args: &[Vec<u8>]) -> R {
    let n = match args.get(1) {
        None => 1,
        Some(a) => match parse_int(a) {
            Some(n) if n >= 0 => n as usize,
            Some(_) => {
                vp.error(&format!("shift: {}: shift count out of range", s(a)))
                    .await;
                return Ok(1);
            }
            None => {
                vp.error(&format!("shift: {}: numeric argument required", s(a)))
                    .await;
                return Ok(2);
            }
        },
    };
    if n > vp.args.len() {
        return Ok(1);
    }
    vp.args.drain(..n);
    Ok(0)
}

/// Quoting for `set` output and `printf %q`: backslashes, or `$'…'` for control chars,
/// `''` for the empty string.
pub(crate) fn shell_quote(v: &[u8]) -> Vec<u8> {
    if v.is_empty() {
        return b"''".to_vec();
    }
    if v.iter().any(|b| b.is_ascii_control()) {
        let mut out = b"$'".to_vec();
        for &b in v {
            match b {
                b'\n' => out.extend_from_slice(b"\\n"),
                b'\t' => out.extend_from_slice(b"\\t"),
                b'\r' => out.extend_from_slice(b"\\r"),
                0x1b => out.extend_from_slice(b"\\E"),
                b'\'' => out.extend_from_slice(b"\\'"),
                b'\\' => out.extend_from_slice(b"\\\\"),
                b if b.is_ascii_control() => out.extend_from_slice(format!("\\{b:03o}").as_bytes()),
                b => out.push(b),
            }
        }
        out.push(b'\'');
        return out;
    }
    let mut out = Vec::with_capacity(v.len());
    for (i, &b) in v.iter().enumerate() {
        let special = matches!(
            b,
            b' ' | b'\''
                | b'"'
                | b'\\'
                | b'$'
                | b'`'
                | b'!'
                | b'&'
                | b';'
                | b'|'
                | b'<'
                | b'>'
                | b'('
                | b')'
                | b'*'
                | b'?'
                | b'['
                | b']'
                | b'{'
                | b'}'
                | b'#'
                | b'^'
                | b','
        ) || (b == b'~' && i == 0)
            || (b == b'=' && i == 0);
        if special {
            out.push(b'\\');
        }
        out.push(b);
    }
    out
}
