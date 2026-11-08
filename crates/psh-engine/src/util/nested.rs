//! `bash` and `sh` named in a command run in psh (D1410): a new shell, started afresh
//! as a new bash process would be (D1300), with psh's own command line
//! (`invocation::parse_args`). It sits in the lookup after functions and builtins and
//! before the utilities, so wrappers (`env`, `xargs`, `find -exec`, `timeout`) reach it,
//! and reports like a utility (`type`, `command -v`, `which`).
//!
//! A path (`/bin/bash`, `./bash`), a hashed name and `command -p bash` run the program,
//! except the four Unix shell paths on Windows, where no such files exist. Options psh
//! does not implement run the real program from `PATH`, as utilities fall through
//! (D1082).

use std::io;

use super::{LocalBoxFuture, Mode, UtilCtx, UtilEntry};
use crate::host::{FileKind, OpenOptions, error_text};
use crate::invocation::{Invocation, parse_args};

/// Not in any registry: the engine looks it up itself (`super::lookup`).
pub(crate) static ENTRY: UtilEntry = UtilEntry {
    names: &["bash", "sh"],
    mode: Mode::Always,
    destructive: false,
    run,
};

/// On Windows these mean psh, as they do in shebangs there.
const WINDOWS_PATHS: [&[u8]; 4] = [b"/bin/sh", b"/bin/bash", b"/usr/bin/sh", b"/usr/bin/bash"];

/// Whether `name` runs as a shell in psh here.
pub(crate) fn is_shell(vp: &crate::state::Vp, name: &[u8]) -> bool {
    vp.core.nested_shells
        && (matches!(name, b"bash" | b"sh") || (cfg!(windows) && WINDOWS_PATHS.contains(&name)))
}

fn run<'a>(ctx: &'a mut UtilCtx<'_>, argv: &'a [Vec<u8>]) -> LocalBoxFuture<'a, i32> {
    Box::pin(shell(ctx, argv))
}

async fn shell(ctx: &mut UtilCtx<'_>, argv: &[Vec<u8>]) -> i32 {
    let (source, arg0, args, options, dash_c) = match parse_args(&argv[1..]) {
        Ok(Invocation::Command {
            source,
            arg0,
            args,
            options,
        }) => (
            source,
            arg0.unwrap_or_else(|| argv[0].clone()),
            args,
            options,
            true,
        ),
        Ok(Invocation::File {
            path,
            args,
            options,
        }) => match read_script(ctx, &path).await {
            Ok(source) => (source, path, args, options, false),
            Err(status) => return status,
        },
        Ok(Invocation::Version) => {
            let text = format!("psh {}\n", env!("CARGO_PKG_VERSION"));
            return i32::from(ctx.write(text.as_bytes()).await.is_err());
        }
        // Until psh reads a script from stdin one command at a time (0041 part 2).
        Ok(Invocation::Stdin { .. }) => {
            return fall_through(ctx, argv, "a script on stdin is not supported yet").await;
        }
        Ok(Invocation::Help) => return fall_through(ctx, argv, "--help: not supported").await,
        Err(msg) => return fall_through(ctx, argv, &msg).await,
    };
    let mut sub = match ctx.afresh().await {
        Ok(sub) => sub,
        Err(status) => return status,
    };
    if ctx.in_place {
        // `exec bash`: the same process goes on, so `$$`, `$PPID` and `SHLVL` stay
        // (bash lowers `SHLVL` before it execs; the new shell raises it again).
        sub.shell_pid = ctx.vp.shell_pid;
        sub.bashpid = ctx.vp.bashpid;
        for name in ["PPID", "SHLVL"] {
            let value = match name {
                "PPID" => ctx.vp.var(name).map(<[u8]>::to_vec),
                _ => ctx.var(name.as_bytes()).map(<[u8]>::to_vec),
            };
            if let (Some(value), Some(var)) = (value, sub.vars.get_mut(name)) {
                var.value = crate::vars::Value::Str(value);
            }
        }
    }
    sub.arg0 = arg0;
    sub.script_args = args.clone();
    sub.args = args;
    sub.opts.dash_c = dash_c;
    crate::builtins::apply_invocation_options(&mut sub, &options);
    let (status, _) = Box::pin(crate::run_script(sub, source)).await;
    status
}

/// An option psh lacks (or a script on stdin, for now): the real program from `PATH`
/// runs, else the shell fails with status 2 naming psh. A `bash` on `PATH` that is psh
/// itself runs as a standalone psh and fails the same way.
async fn fall_through(ctx: &mut UtilCtx<'_>, argv: &[Vec<u8>], why: &str) -> i32 {
    if let Some(status) = ctx.run_program(argv.to_vec()).await {
        return status;
    }
    let name = ctx.name().to_string();
    ctx.error(&format!("{why} (psh built-in {name})")).await;
    2
}

/// The script `bash FILE` runs: `FILE` from the cwd, else (no slash) from a directory
/// of `PATH`, as bash looks for it (it need not be executable). Errors as bash's: 127
/// for a missing file, 126 for one that cannot be read, a directory or a binary file.
async fn read_script(ctx: &mut UtilCtx<'_>, path: &[u8]) -> Result<Vec<u8>, i32> {
    let shown = String::from_utf8_lossy(path).into_owned();
    let mut target = path.to_vec();
    if !path.contains(&b'/')
        && ctx.stat(path, true).await.is_err()
        && let Some(found) = search_path(ctx, path).await
    {
        target = found;
    }
    let read = match ctx.open(&target, OpenOptions::read()).await {
        Ok(f) => {
            let mut source = Vec::new();
            f.read_to_end(&mut source).await.map(|()| source)
        }
        Err(e) => Err(e),
    };
    let source = match read {
        Ok(source) => source,
        Err(e) => {
            if matches!(ctx.stat(&target, true).await, Ok(m) if m.kind == FileKind::Dir) {
                return Err(refused(ctx, &shown, "Is a directory").await);
            }
            ctx.error(&format!("{shown}: {}", error_text(&e))).await;
            return Err(if e.kind() == io::ErrorKind::NotFound {
                127
            } else {
                126
            });
        }
    };
    // bash's test, as for a file it runs itself (D1300): a NUL in the first line of the
    // first 80 bytes.
    let head = &source[..source.len().min(80)];
    let first_line = head.split(|&b| b == b'\n').next().unwrap_or(head);
    if first_line.contains(&0) {
        return Err(refused(ctx, &shown, "cannot execute binary file").await);
    }
    Ok(source)
}

/// bash names the script twice for these: its `$0` is the file already. Status 126.
async fn refused(ctx: &mut UtilCtx<'_>, shown: &str, what: &str) -> i32 {
    ctx.error_raw(format!("{shown}: {shown}: {what}\n").as_bytes())
        .await;
    126
}

/// The first regular file named `name` in a directory of the command's `PATH`.
async fn search_path(ctx: &UtilCtx<'_>, name: &[u8]) -> Option<Vec<u8>> {
    let path = ctx
        .var(b"PATH")
        .map(<[u8]>::to_vec)
        .unwrap_or_else(|| crate::exec::DEFAULT_PATH.to_vec());
    for dir in ctx.vp.host().path_entries(&path) {
        let mut p = dir;
        if !p.is_empty() {
            p.push(b'/');
        }
        p.extend_from_slice(name);
        if matches!(ctx.stat(&p, true).await, Ok(m) if m.kind == FileKind::File) {
            return Some(p);
        }
    }
    None
}
