//! The interpreter: lists, pipelines, commands, redirections, externals.
//!
//! Written as async functions over the AST (D1050); every `.await` is a point where the
//! vproc can wait without blocking the thread. Recursion goes through boxed futures.

use std::future::poll_fn;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Poll;

use psh_syntax::{
    AndOr, AndOrOp, AssignValue, CmdId, Command, CommandKind, ListId, Pipeline, RedirFd, RedirOp,
    RedirTarget, Redirect, SimpleCommand,
};

use crate::builtins;
use crate::expand::{self, Ctx};
use crate::fd::{Fd, FdObj, Pump};
use crate::host::{ChildFd, OpenOptions, SpawnSpec, error_text};
use crate::pipe;
use crate::sched::{Join, LocalBoxFuture, safepoint};
use crate::state::{Code, R, Unwind, Vp};

const DEFAULT_PATH: &[u8] = b"/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";

pub(crate) fn run_list<'a>(
    vp: &'a mut Vp,
    code: &'a Rc<Code>,
    id: ListId,
) -> LocalBoxFuture<'a, R> {
    Box::pin(async move {
        let list = code.ast.list(id);
        let mut status = vp.status;
        for (i, item) in list.items.iter().enumerate() {
            status = if item.background {
                run_background(vp, code, id, i).await?
            } else {
                run_andor(vp, code, item).await?
            };
            vp.status = status;
        }
        Ok(status)
    })
}

/// `cmd &`: a detached subshell task. `$!` gets a made-up id until jobs exist (0006).
async fn run_background(vp: &mut Vp, code: &Rc<Code>, list: ListId, item: usize) -> R {
    let mut sub = enter(vp).await?;
    let code = code.clone();
    vp.last_bg = Some(vp.core.next_job_id());
    vp.core.sched.spawn_task(Box::pin(async move {
        let ao = &code.ast.list(list).items[item];
        let _ = run_andor(&mut sub, &code, ao).await;
    }));
    Ok(0)
}

async fn run_andor(vp: &mut Vp, code: &Rc<Code>, ao: &AndOr) -> R {
    let mut status = run_pipeline(vp, code, &ao.first).await?;
    for (op, p) in &ao.rest {
        vp.status = status;
        let run = match op {
            AndOrOp::And => status == 0,
            AndOrOp::Or => status != 0,
        };
        if run {
            status = run_pipeline(vp, code, p).await?;
        }
    }
    Ok(status)
}

async fn run_pipeline(vp: &mut Vp, code: &Rc<Code>, p: &Pipeline) -> R {
    let status = if p.stages.is_empty() {
        0
    } else if p.stages.len() == 1 {
        run_command(vp, code, p.stages[0].cmd).await?
    } else {
        let n = p.stages.len();
        let mut joins: Vec<Join<i32>> = Vec::with_capacity(n);
        let mut next_in: Option<Fd> = None;
        for (i, stage) in p.stages.iter().enumerate() {
            let mut sub = enter(vp).await?;
            if let Some(r) = next_in.take() {
                sub.fds.set(0, r);
            }
            if i + 1 < n {
                let (r, w) = pipe::pipe();
                let w: Fd = Rc::new(FdObj::PipeWrite(w));
                sub.fds.set(1, w.clone());
                if stage.pipe_stderr {
                    sub.fds.set(2, w);
                }
                next_in = Some(Rc::new(FdObj::PipeRead(r)));
            }
            joins.push(spawn_sub(sub, code.clone(), Body::Cmd(stage.cmd)));
        }
        let mut last = 0;
        for j in joins {
            last = j.await;
        }
        last
    };
    Ok(if p.negated() {
        i32::from(status == 0)
    } else {
        status
    })
}

/// Clones the state for a subshell, or fails past the nesting limit (D1050).
pub(crate) async fn enter(vp: &Vp) -> Result<Vp, Unwind> {
    if vp.depth >= vp.core.max_depth {
        vp.error(&format!(
            "maximum nesting level exceeded ({})",
            vp.core.max_depth
        ))
        .await;
        return Err(Unwind::Exit(2));
    }
    Ok(vp.subshell())
}

pub(crate) enum Body {
    List(ListId),
    Cmd(CmdId),
}

/// Runs a subshell as its own task; the handle yields its exit status.
pub(crate) fn spawn_sub(sub: Vp, code: Rc<Code>, body: Body) -> Join<i32> {
    let sched = sub.core.sched.clone();
    sched.spawn(async move {
        let mut sub = sub;
        let r = match body {
            Body::List(l) => run_list(&mut sub, &code, l).await,
            Body::Cmd(c) => run_command(&mut sub, &code, c).await,
        };
        let last = sub.status;
        r.unwrap_or_else(|u| u.status(last))
    })
}

pub(crate) fn run_command<'a>(
    vp: &'a mut Vp,
    code: &'a Rc<Code>,
    id: CmdId,
) -> LocalBoxFuture<'a, R> {
    Box::pin(async move {
        safepoint(&vp.core.sched).await;
        let cmd = code.ast.cmd(id);
        match &cmd.kind {
            CommandKind::Simple(s) => run_simple(vp, code, cmd, s).await,
            CommandKind::Brace(_) | CommandKind::Subshell(_) => with_redirects(vp, code, cmd).await,
            _ => {
                vp.line = code.line(cmd.span.start);
                expand::unsupported(vp, compound_name(&cmd.kind)).await
            }
        }
    })
}

fn compound_name(kind: &CommandKind) -> &'static str {
    match kind {
        CommandKind::If(_) => "if",
        CommandKind::While(w) if w.until => "until",
        CommandKind::While(_) => "while",
        CommandKind::For(_) => "for",
        CommandKind::Select(_) => "select",
        CommandKind::ArithFor(_) => "for ((…))",
        CommandKind::Case(_) => "case",
        CommandKind::Arith(_) => "((…))",
        CommandKind::Cond(_) => "[[ … ]]",
        CommandKind::Function(_) => "function definitions",
        CommandKind::Coproc(_) => "coproc",
        _ => "this command",
    }
}

/// Runs a compound command with its redirections applied, then restores the
/// descriptors. A failed redirection is status 1 and the body does not run.
async fn with_redirects(vp: &mut Vp, code: &Rc<Code>, cmd: &Command) -> R {
    let saved = if cmd.redirects.is_empty() {
        None
    } else {
        let saved = vp.fds.clone();
        match apply_redirects(vp, code, &cmd.redirects).await {
            Ok(true) => {}
            Ok(false) => {
                vp.fds = saved;
                return Ok(1);
            }
            Err(u) => {
                vp.fds = saved;
                return Err(u);
            }
        }
        Some(saved)
    };
    let r = match &cmd.kind {
        CommandKind::Brace(list) => run_list(vp, code, *list).await,
        CommandKind::Subshell(list) => match enter(vp).await {
            Ok(sub) => Ok(spawn_sub(sub, code.clone(), Body::List(*list)).await),
            Err(u) => Err(u),
        },
        _ => Ok(0),
    };
    if let Some(saved) = saved {
        vp.fds = saved;
    }
    r
}

async fn run_simple(vp: &mut Vp, code: &Rc<Code>, cmd: &Command, s: &SimpleCommand) -> R {
    vp.line = code.line(cmd.span.start);
    let decl = s
        .words
        .first()
        .and_then(|w| psh_syntax::literal_text(&code.ast, w))
        .is_some_and(|t| builtins::is_declaration(t.as_bytes()));
    let fields = if decl {
        expand::expand_decl_words(vp, code, &s.words).await?
    } else {
        expand::expand_words(vp, code, &s.words).await?
    };
    let mut subst_status = fields.subst_status;
    let argv: Vec<Vec<u8>> = fields.out.into_iter().map(|f| f.text).collect();

    // Assignments, expanded after the words.
    let mut assigns: Vec<(String, Vec<u8>)> = Vec::with_capacity(s.assigns.len());
    for a in &s.assigns {
        let name = code.ast.text(a.name).to_string();
        if a.index.is_some() {
            return expand::unsupported(vp, "array assignment").await;
        }
        let value = match &a.value {
            AssignValue::Scalar(w) => {
                let (v, st) = expand::assign_value(vp, code, w).await?;
                if st.is_some() {
                    subst_status = st;
                }
                v
            }
            AssignValue::Array(_) => return expand::unsupported(vp, "array assignment").await,
        };
        let value = if a.append {
            let mut old = vp.var(&name).map(<[u8]>::to_vec).unwrap_or_default();
            old.extend_from_slice(&value);
            old
        } else {
            value
        };
        if argv.is_empty() {
            // Assigned at once: later assignments see earlier ones (`a=1 b=$a`).
            if vp.vars.get(&name).is_some_and(|v| v.readonly) {
                vp.error(&format!("{name}: readonly variable")).await;
                return Ok(1);
            }
            vp.set_var(&name, value);
        } else {
            assigns.push((name, value));
        }
    }

    if argv.is_empty() {
        let status = subst_status.unwrap_or(0);
        if cmd.redirects.is_empty() {
            return Ok(status);
        }
        let saved = vp.fds.clone();
        let ok = apply_redirects(vp, code, &cmd.redirects).await;
        vp.fds = saved;
        return Ok(if ok? { status } else { 1 });
    }

    let name = String::from_utf8_lossy(&argv[0]).into_owned();
    // `exec` without a command keeps its redirections.
    if name == "exec" && argv.len() == 1 {
        return Ok(if apply_redirects(vp, code, &cmd.redirects).await? {
            0
        } else {
            1
        });
    }

    let saved = vp.fds.clone();
    match apply_redirects(vp, code, &cmd.redirects).await {
        Ok(true) => {}
        Ok(false) => {
            vp.fds = saved;
            return Ok(1);
        }
        Err(u) => {
            vp.fds = saved;
            return Err(u);
        }
    }
    let r = run_argv(vp, argv, assigns).await;
    vp.fds = saved;
    r
}

/// Runs an expanded command: a builtin or an external.
async fn run_argv(vp: &mut Vp, argv: Vec<Vec<u8>>, assigns: Vec<(String, Vec<u8>)>) -> R {
    let name = argv[0].clone();
    if let Some(b) = builtins::find(&name) {
        if b.special {
            for (n, v) in assigns {
                vp.set_var(&n, v);
            }
            return (b.run)(vp, &argv).await;
        }
        // Assignments in front of a regular builtin last for the builtin only.
        let saved: Vec<(String, Option<crate::state::Var>)> = assigns
            .iter()
            .map(|(n, _)| (n.clone(), vp.vars.get(n).cloned()))
            .collect();
        for (n, v) in assigns {
            vp.set_var(&n, v);
        }
        let r = (b.run)(vp, &argv).await;
        for (n, old) in saved {
            match old {
                Some(var) => vp.vars.insert(&n, var),
                None => {
                    vp.vars.remove(&n);
                }
            }
        }
        return r;
    }
    if name == b"exec" {
        let r = run_external(vp, argv[1..].to_vec(), assigns).await?;
        return Err(Unwind::Exit(r));
    }
    run_external(vp, argv, assigns).await
}

/// Starts an external command and waits for it, pumping its pipes to engine objects.
async fn run_external(vp: &mut Vp, argv: Vec<Vec<u8>>, assigns: Vec<(String, Vec<u8>)>) -> R {
    if argv.is_empty() {
        return Ok(0);
    }
    let name = argv[0].clone();
    let shown = String::from_utf8_lossy(&name).into_owned();
    let program = if name.contains(&b'/') {
        vp.abs_path(&name)
    } else {
        let path = vp
            .var("PATH")
            .map(<[u8]>::to_vec)
            .unwrap_or_else(|| DEFAULT_PATH.to_vec());
        match vp.host().resolve(&name, &path, &vp.cwd).await {
            Some(p) => p,
            None => {
                vp.error(&format!("{shown}: command not found")).await;
                return Ok(127);
            }
        }
    };
    let mut fds = Vec::new();
    let mut has = [false; 3];
    for (n, fd) in vp.fds.iter() {
        let child = match &**fd {
            FdObj::File(f) => ChildFd::File(f.clone()),
            FdObj::PipeRead(_) => ChildFd::Pipe { input: true },
            FdObj::PipeWrite(_) => ChildFd::Pipe { input: false },
        };
        if (n as usize) < 3 {
            has[n as usize] = true;
        }
        fds.push((n, child));
    }
    for (n, h) in has.iter().enumerate() {
        if !h {
            fds.push((n as u32, ChildFd::Null));
        }
    }
    let spec = SpawnSpec {
        program,
        argv,
        env: vp.env(&assigns),
        cwd: vp.cwd.clone(),
        fds,
    };
    let spawned = match vp.host().spawn(spec) {
        Ok(s) => s,
        Err(e) => {
            let status = if e.kind() == std::io::ErrorKind::NotFound {
                127
            } else {
                126
            };
            vp.error(&format!("{shown}: {}", error_text(&e))).await;
            return Ok(status);
        }
    };
    let mut proc = spawned.proc;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for (n, theirs) in spawned.pipes {
        let Some(ours) = vp.fd(n) else { continue };
        let theirs: Fd = Rc::new(FdObj::File(theirs));
        match &*ours {
            FdObj::PipeRead(_) => inputs.push(Pump::new(ours.clone(), theirs)),
            _ => outputs.push(Pump::new(theirs, ours.clone())),
        }
    }
    let mut status = None;
    let st = poll_fn(|cx| {
        if status.is_none() {
            for p in &mut inputs {
                let _ = p.poll(cx);
            }
            inputs.retain(|p| !p.is_done());
        }
        for p in &mut outputs {
            let _ = p.poll(cx);
        }
        outputs.retain(|p| !p.is_done());
        if status.is_none()
            && let Poll::Ready(s) = proc.poll_wait(cx)
        {
            status = Some(s);
            // The process is gone: its input pipes close, and a vproc writing to
            // them gets EPIPE instead of waiting forever.
            inputs.clear();
        }
        match status {
            // Output is complete only at EOF of its pipes (pitfall 7).
            Some(s) if outputs.is_empty() => Poll::Ready(s),
            _ => Poll::Pending,
        }
    })
    .await;
    Ok(st.code())
}

/// The default descriptor of a redirection operator.
fn default_fd(op: RedirOp) -> u32 {
    match op {
        RedirOp::Read
        | RedirOp::ReadWrite
        | RedirOp::DupRead
        | RedirOp::HereString
        | RedirOp::HereDoc
        | RedirOp::HereDocStrip => 0,
        _ => 1,
    }
}

/// Applies redirections in order. `Ok(false)`: one failed (message printed).
pub(crate) async fn apply_redirects(
    vp: &mut Vp,
    code: &Rc<Code>,
    redirects: &[Redirect],
) -> Result<bool, Unwind> {
    for r in redirects {
        if !apply_redirect(vp, code, r).await? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `/dev/stdin`, `/dev/stdout`, `/dev/stderr` and `/dev/fd/N` refer to the vproc's own
/// descriptors (the process's are not the shell's when it runs embedded).
fn dev_fd(path: &[u8]) -> Option<u32> {
    match path {
        b"/dev/stdin" => Some(0),
        b"/dev/stdout" => Some(1),
        b"/dev/stderr" => Some(2),
        _ => path
            .strip_prefix(b"/dev/fd/")
            .and_then(|n| std::str::from_utf8(n).ok())
            .and_then(|n| n.parse().ok()),
    }
}

async fn apply_redirect(vp: &mut Vp, code: &Rc<Code>, r: &Redirect) -> Result<bool, Unwind> {
    let fd = match r.fd {
        Some(RedirFd::Number(n)) => n,
        Some(RedirFd::Var(_)) => return expand::unsupported(vp, "{var}> redirection").await,
        None => default_fd(r.op),
    };
    match r.op {
        RedirOp::HereDoc | RedirOp::HereDocStrip => {
            let RedirTarget::HereDoc(id) = &r.target else {
                return Ok(true);
            };
            let doc = code.ast.heredoc(*id);
            let body = if doc.quoted {
                literal_body(code, &doc.body)
            } else {
                if let Some(e) = &doc.error {
                    vp.error(&e.message).await;
                    return Ok(false);
                }
                expand::expand_heredoc(vp, code, &doc.body).await?
            };
            vp.fds
                .set(fd, Rc::new(FdObj::PipeRead(pipe::preloaded(&body))));
            return Ok(true);
        }
        RedirOp::HereString => {
            let RedirTarget::Word(w) = &r.target else {
                return Ok(true);
            };
            let (mut body, _) = expand::expand_single(vp, code, w, Ctx::Word).await?;
            body.push(b'\n');
            vp.fds
                .set(fd, Rc::new(FdObj::PipeRead(pipe::preloaded(&body))));
            return Ok(true);
        }
        _ => {}
    }
    let RedirTarget::Word(word) = &r.target else {
        return Ok(true);
    };
    let Some(target) = expand::expand_target(vp, code, word).await? else {
        vp.error(&format!("{}: ambiguous redirect", code.ast.text(word.span)))
            .await;
        return Ok(false);
    };
    match r.op {
        RedirOp::DupRead | RedirOp::DupWrite => {
            if target == b"-" {
                vp.fds.close(fd);
                return Ok(true);
            }
            let (num, close) = match target.strip_suffix(b"-") {
                Some(n) => (n, true),
                None => (&target[..], false),
            };
            if !num.is_empty() && num.iter().all(u8::is_ascii_digit) {
                let src: u32 = match std::str::from_utf8(num).ok().and_then(|s| s.parse().ok()) {
                    Some(n) => n,
                    None => {
                        vp.error(&format!(
                            "{}: Bad file descriptor",
                            String::from_utf8_lossy(num)
                        ))
                        .await;
                        return Ok(false);
                    }
                };
                match vp.fd(src) {
                    Some(obj) => {
                        vp.fds.set(fd, obj);
                        if close && src != fd {
                            vp.fds.close(src);
                        }
                    }
                    None => {
                        vp.error(&format!("{src}: Bad file descriptor")).await;
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
            if r.op == RedirOp::DupWrite && r.fd.is_none() {
                // `>& file` is `&> file`.
                return open_to(vp, &target, OpenOptions::write(), &[1, 2]).await;
            }
            vp.error(&format!(
                "{}: ambiguous redirect",
                String::from_utf8_lossy(&target)
            ))
            .await;
            Ok(false)
        }
        RedirOp::Read => open_to(vp, &target, OpenOptions::read(), &[fd]).await,
        RedirOp::Write | RedirOp::Clobber => {
            open_to(vp, &target, OpenOptions::write(), &[fd]).await
        }
        RedirOp::Append => open_to(vp, &target, OpenOptions::append(), &[fd]).await,
        RedirOp::ReadWrite => open_to(vp, &target, OpenOptions::read_write(), &[fd]).await,
        RedirOp::WriteBoth => open_to(vp, &target, OpenOptions::write(), &[1, 2]).await,
        RedirOp::AppendBoth => open_to(vp, &target, OpenOptions::append(), &[1, 2]).await,
        RedirOp::HereString | RedirOp::HereDoc | RedirOp::HereDocStrip => Ok(true),
    }
}

async fn open_to(
    vp: &mut Vp,
    target: &[u8],
    opts: OpenOptions,
    fds: &[u32],
) -> Result<bool, Unwind> {
    if let Some(src) = dev_fd(target) {
        return match vp.fd(src) {
            Some(obj) => {
                for &n in fds {
                    vp.fds.set(n, obj.clone());
                }
                Ok(true)
            }
            None => {
                vp.error(&format!("{src}: Bad file descriptor")).await;
                Ok(false)
            }
        };
    }
    if target.is_empty() {
        vp.error(": No such file or directory").await;
        return Ok(false);
    }
    let path = vp.abs_path(target);
    match vp.host().open(&path, opts).await {
        Ok(f) => {
            let obj: Fd = Rc::new(FdObj::File(f));
            for &n in fds {
                vp.fds.set(n, obj.clone());
            }
            Ok(true)
        }
        Err(e) => {
            vp.error(&format!(
                "{}: {}",
                String::from_utf8_lossy(target),
                error_text(&e)
            ))
            .await;
            Ok(false)
        }
    }
}

/// The body of a quoted here-document: its text, without the stripped tabs.
fn literal_body(code: &Code, body: &psh_syntax::Word) -> Vec<u8> {
    let mut out = Vec::new();
    for p in &body.parts {
        out.extend_from_slice(code.ast.text(p.span).as_bytes());
    }
    out
}

/// `$(list)`: runs the list in a subshell with stdout into a pipe, reads it to the
/// end, strips trailing newlines. Sets `$?`.
pub(crate) async fn command_subst(
    vp: &mut Vp,
    code: &Rc<Code>,
    list: ListId,
) -> Result<(Vec<u8>, i32), Unwind> {
    subst(vp, code.clone(), list).await
}

async fn subst(vp: &mut Vp, code: Rc<Code>, list: ListId) -> Result<(Vec<u8>, i32), Unwind> {
    let mut sub = enter(vp).await?;
    let (r, w) = pipe::pipe();
    sub.fds.set(1, Rc::new(FdObj::PipeWrite(w)));
    let join = spawn_sub(sub, code, Body::List(list));
    let reader = FdObj::PipeRead(r);
    let mut out = Vec::new();
    let _ = reader.read_to_end(&mut out).await;
    drop(reader);
    let status = join.await;
    vp.status = status;
    if out.contains(&0) {
        vp.error("warning: command substitution: ignored null byte in input")
            .await;
        out.retain(|&b| b != 0);
    }
    while out.last() == Some(&b'\n') {
        out.pop();
    }
    Ok((out, status))
}

/// `` `text` ``: parsed when it runs, as bash does.
pub(crate) async fn command_subst_text(
    vp: &mut Vp,
    code: &Rc<Code>,
    text: &str,
    offset: u32,
) -> Result<(Vec<u8>, i32), Unwind> {
    match psh_syntax::parse(text) {
        Ok(ast) => {
            let root = ast.root();
            let inner = Rc::new(Code {
                lines: Rc::new(psh_syntax::LineIndex::new(ast.source())),
                ast: Arc::new(ast),
                line_base: code.line(offset) - 1,
            });
            subst(vp, inner, root).await
        }
        Err(e) => {
            vp.error(&format!("command substitution: {}", e.message))
                .await;
            vp.status = 2;
            Ok((Vec::new(), 2))
        }
    }
}
