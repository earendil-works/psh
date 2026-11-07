//! The psh interpreter: a synchronous, sans-IO state machine stepped by its embedder
//! (ARCH.md "psh-engine", D005, D1050).
//!
//! ```
//! use std::rc::Rc;
//! use psh_engine::{Script, Shell, ShellConfig, Step, testhost::TestHost};
//!
//! let host = Rc::new(TestHost::new());
//! let mut shell = Shell::new(host.config());
//! shell.start(Script::command("echo hi | cat"));
//! let status = loop {
//!     match shell.step(1000) {
//!         Step::Done(status) => break status,
//!         Step::Yield | Step::Blocked => continue,
//!         Step::Idle => unreachable!(),
//!     }
//! };
//! assert_eq!(status, 0);
//! assert_eq!(host.stdout(), b"hi\n");
//! ```
//!
//! Every side effect goes through a [`Host`]; the engine waits only through
//! `std::task::Waker`s, never by blocking the thread, and runs no threads itself. A
//! [`Shell`] is `!Send`: create and step it on one thread.

mod arith;
mod arrays;
mod builtins;
mod escape;
mod exec;
mod expand;
mod fd;
pub mod host;
pub mod invocation;
mod pattern;
mod pipe;
mod sched;
mod state;
#[cfg(any(unix, windows))]
pub mod std_host;
pub mod testhost;
#[cfg(test)]
mod tests;
mod vars;

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::task::Waker;

use psh_syntax::{LineIndex, Options, Parser};

pub use host::{Host, HostFile};

use fd::{FdObj, FdTable};
use sched::{CANCEL, Join, PAUSE, RunEnd, Sched, Shared};
use state::{Code, Core, Unwind, Var, Vars, Vp};

/// What one call of [`Shell::step`] ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The script finished with this status. Reported once; then `Idle`.
    Done(i32),
    /// Everything waits on the host (or the shell is paused). The waker given to
    /// [`Shell::set_waker`] fires when there is work again.
    Blocked,
    /// The budget is spent; call `step` again (there is work).
    Yield,
    /// No script is running.
    Idle,
}

/// Settings for a new [`Shell`].
pub struct ShellConfig {
    pub host: Rc<dyn Host>,
    /// The environment the shell starts with.
    pub env: Vec<(Vec<u8>, Vec<u8>)>,
    /// The working directory, absolute.
    pub cwd: Vec<u8>,
    /// Descriptors 0, 1 and 2; `None` is closed.
    pub stdin: Option<Rc<dyn HostFile>>,
    pub stdout: Option<Rc<dyn HostFile>>,
    pub stderr: Option<Rc<dyn HostFile>>,
    /// The nesting limit for subshells and substitutions (D1050). Each level costs
    /// native stack while it is polled; see [`ShellConfig::STACK_PER_LEVEL`].
    pub max_depth: u32,
    /// What happens to background jobs (`cmd &`) still running when the script ends.
    /// `None`: they are released and outlive the shell, as in bash. `Some(grace)`: the
    /// shell waits up to `grace` for them, then abandons them (TERM, KILL): for
    /// embedders that must leave no process behind (D1051).
    pub exit_grace: Option<std::time::Duration>,
}

impl ShellConfig {
    /// Native stack an embedder should budget per nesting level: a function call
    /// measured about 3 KiB in release and 15 KiB in debug builds
    /// (`examples/stackprobe.rs`, D1050, D1059).
    pub const STACK_PER_LEVEL: usize = 16 * 1024;
    /// The default nesting limit: 1000 levels need up to 16 MiB of stack. On wasm32
    /// (1 MiB stack by default) use about 200, or link with a larger stack.
    pub const DEFAULT_MAX_DEPTH: u32 = 1000;

    /// A config with closed stdio, an empty environment and `/` as the working
    /// directory.
    pub fn new(host: Rc<dyn Host>) -> ShellConfig {
        ShellConfig {
            host,
            env: Vec::new(),
            cwd: b"/".to_vec(),
            stdin: None,
            stdout: None,
            stderr: None,
            max_depth: ShellConfig::DEFAULT_MAX_DEPTH,
            exit_grace: None,
        }
    }
}

/// A script to run.
pub struct Script {
    /// The script's bytes; need not be UTF-8 (parsed byte-exactly).
    pub source: Vec<u8>,
    /// `$0`, also the name in front of messages.
    pub arg0: Vec<u8>,
    /// `$1`…
    pub args: Vec<Vec<u8>>,
    /// Run as `-c`: syntax errors say `-c:`.
    pub dash_c: bool,
}

impl Script {
    /// `psh -c SOURCE`.
    pub fn command(source: &str) -> Script {
        Script {
            source: source.as_bytes().to_vec(),
            arg0: b"psh".to_vec(),
            args: Vec::new(),
            dash_c: true,
        }
    }
}

/// Lets another thread cancel or pause a running shell. Cheap to clone, `Send` and
/// `Sync`.
#[derive(Clone)]
pub struct ControlHandle {
    shared: Arc<Shared>,
}

impl ControlHandle {
    /// Stops the run: at the next step every vproc gets a virtual TERM (no traps yet:
    /// the default action, termination) and the run ends with status 143. Externals
    /// are abandoned (TERM, then KILL after the host's grace period).
    pub fn cancel(&self) {
        self.shared.control.fetch_or(CANCEL, Ordering::SeqCst);
        self.shared.wake_outer();
    }

    /// Stops at the next safepoint; `step` returns `Blocked` until `resume`.
    pub fn pause(&self) {
        self.shared.control.fetch_or(PAUSE, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.shared.control.fetch_and(!PAUSE, Ordering::SeqCst);
        self.shared.wake_outer();
    }
}

/// A shell: its state, the scheduler of its vprocs, and the script running.
pub struct Shell {
    core: Rc<Core>,
    shared: Arc<Shared>,
    /// The main vproc's state between runs.
    vp: Option<Vp>,
    /// The state before the current run, restored if it is cancelled.
    before: Option<Vp>,
    main: Option<Join<(i32, Vp)>>,
    /// The script ended with this status; its background jobs get until the timer
    /// (`exit_grace`) to finish.
    draining: Option<(i32, Join<()>)>,
    exit_grace: Option<std::time::Duration>,
    /// `Rc` inside: a shell stays on the thread that made it.
    _not_send: PhantomData<*const ()>,
}

impl Shell {
    pub fn new(cfg: ShellConfig) -> Shell {
        let shared = Arc::new(Shared::new());
        let core = Rc::new(Core {
            host: cfg.host.clone(),
            sched: Rc::new(Sched::new(shared.clone())),
            msg_name: RefCell::new("psh".into()),
            dash_c: Cell::new(false),
            max_depth: cfg.max_depth,
            jobs: Cell::new(0),
            release_on_drop: Cell::new(false),
        });
        let mut vars = Vars::default();
        let mut foreign = Vec::new();
        for (k, v) in &cfg.env {
            match std::str::from_utf8(k) {
                Ok(name) if state::is_name(k) => vars.insert(
                    name,
                    Var {
                        exported: true,
                        ..Var::scalar(v.clone())
                    },
                ),
                _ => foreign.push((k.clone(), v.clone())),
            }
        }
        vars.insert("IFS", Var::scalar(b" \t\n".to_vec()));
        let pwd = vars.entry("PWD");
        pwd.value = vars::Value::Str(cfg.cwd.clone());
        pwd.exported = true;
        let shlvl = vars
            .value("SHLVL")
            .and_then(|v| std::str::from_utf8(v).ok())
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(0);
        let lvl = vars.entry("SHLVL");
        lvl.value = vars::Value::Str((shlvl + 1).to_string().into_bytes());
        lvl.exported = true;
        let old = vars.entry("OLDPWD");
        old.exported = true;
        let mut fds = FdTable::default();
        for (n, f) in [(0, cfg.stdin), (1, cfg.stdout), (2, cfg.stderr)] {
            if let Some(f) = f {
                fds.set(n, Rc::new(FdObj::File(f)));
            }
        }
        let vp = Vp {
            core: core.clone(),
            vars,
            foreign_env: Rc::new(foreign),
            arg0: b"psh".to_vec(),
            args: Vec::new(),
            cwd: cfg.cwd,
            fds,
            status: 0,
            line: 0,
            depth: 0,
            last_bg: None,
            funcs: Rc::new(Default::default()),
            frames: Vec::new(),
            loops: 0,
            getopts: (0, 0),
            aliases: psh_syntax::Aliases::new(),
            expand_aliases: false,
            sourced: Vec::new(),
            hash: Default::default(),
            disabled: Default::default(),
        };
        Shell {
            core,
            shared,
            vp: Some(vp),
            before: None,
            main: None,
            draining: None,
            exit_grace: cfg.exit_grace,
            _not_send: PhantomData,
        }
    }

    /// The handle to cancel or pause this shell from any thread.
    pub fn control(&self) -> ControlHandle {
        ControlHandle {
            shared: self.shared.clone(),
        }
    }

    /// The waker to wake when a `Blocked` shell can make progress.
    pub fn set_waker(&self, waker: Waker) {
        self.shared.set_waker(waker);
    }

    /// Starts a script. Clears earlier cancel and pause requests, so a late cancel
    /// never reaches the next run. Panics if a script is still running.
    pub fn start(&mut self, script: Script) {
        assert!(self.main.is_none(), "a script is already running");
        self.shared.control.store(0, Ordering::SeqCst);
        let mut vp = self.vp.take().expect("shell state");
        self.before = Some(vp.clone());
        *self.core.msg_name.borrow_mut() = String::from_utf8_lossy(&script.arg0).into_owned();
        self.core.dash_c.set(script.dash_c);
        vp.arg0 = script.arg0;
        vp.args = script.args;
        let fut = run_script(vp, script.source);
        self.main = Some(self.core.sched.spawn(fut));
    }

    /// Runs ready work for at most `budget` safepoints (about one command each).
    pub fn step(&mut self, budget: u32) -> Step {
        loop {
            if let Some((status, timer)) = &self.draining {
                let status = *status;
                let ctl = self.shared.control.load(Ordering::SeqCst);
                let sched = &self.core.sched;
                // Done when only the timer is left, or it fired.
                let end = if ctl & CANCEL != 0 {
                    RunEnd::Done
                } else {
                    sched.run(budget, &|| timer.is_done() || sched.task_count() <= 1)
                };
                match end {
                    RunEnd::Done => {
                        self.draining = None;
                        self.core.sched.drop_all();
                        self.shared.control.store(0, Ordering::SeqCst);
                        return Step::Done(status);
                    }
                    RunEnd::Yield => return Step::Yield,
                    RunEnd::Blocked => return Step::Blocked,
                    RunEnd::Control => continue,
                }
            }
            let Some(main) = &self.main else {
                return Step::Idle;
            };
            let ctl = self.shared.control.load(Ordering::SeqCst);
            if ctl & CANCEL != 0 {
                return self.finish_cancelled();
            }
            if ctl & PAUSE != 0 {
                return Step::Blocked;
            }
            let end = self.core.sched.run(budget, &|| main.is_done());
            match end {
                RunEnd::Done => {
                    let (status, vp) = self
                        .main
                        .take()
                        .and_then(|m| m.try_take())
                        .expect("main result");
                    self.vp = Some(vp);
                    // Give detached vprocs (`cmd &`) what is ready.
                    self.core.sched.run(budget, &|| false);
                    if let Some(grace) = self.exit_grace
                        && self.core.sched.task_count() > 0
                    {
                        let timer = self.core.host.sleep(grace);
                        let join = self.core.sched.spawn(timer);
                        self.draining = Some((status, join));
                        continue;
                    }
                    // Release what still runs (bash leaves background jobs running).
                    self.core.release_on_drop.set(true);
                    self.core.sched.drop_all();
                    self.core.release_on_drop.set(false);
                    return Step::Done(status);
                }
                RunEnd::Yield => return Step::Yield,
                RunEnd::Blocked => return Step::Blocked,
                RunEnd::Control => continue,
            }
        }
    }

    fn finish_cancelled(&mut self) -> Step {
        self.main = None;
        self.core.sched.drop_all();
        self.shared.control.store(0, Ordering::SeqCst);
        // The main vproc's state went with its task: the next run starts from the
        // state before this one.
        self.vp = self.before.take();
        Step::Done(128 + host::SIGTERM)
    }

    /// The number of live tasks (vprocs and helpers), for leak tests.
    pub fn task_count(&self) -> usize {
        self.core.sched.task_count()
    }
}

/// The main vproc: parses and runs the script one top-level line at a time.
async fn run_script(mut vp: Vp, source: Vec<u8>) -> (i32, Vp) {
    let lines = Rc::new(LineIndex::from_bytes(&source));
    let mut parser = Parser::from_bytes(&source, Options::default());
    loop {
        // Bash expands aliases while it reads each line, with the table it has then.
        parser.set_aliases(vp.expand_aliases.then_some(&vp.aliases));
        let Some(chunk) = parser.next_chunk() else {
            break;
        };
        match chunk {
            Ok(ast) => {
                let root = ast.root();
                let code = Rc::new(Code {
                    ast: Arc::new(ast),
                    lines: lines.clone(),
                    line_base: 0,
                });
                match exec::run_list(&mut vp, &code, root).await {
                    Ok(s) => vp.status = s,
                    Err(u) => {
                        let st = u.status(vp.status);
                        vp.status = st;
                        if let Unwind::Exit(_) = u {
                            return (st, vp);
                        }
                    }
                }
            }
            Err(e) => {
                let line = lines.line(e.span.start);
                vp.line = line;
                let dash_c = if vp.core.dash_c.get() { "-c: " } else { "" };
                let name = vp.core.msg_name.borrow().clone();
                let mut msg = format!("{name}: {dash_c}line {line}: {}\n", e.message).into_bytes();
                if e.message.starts_with("syntax error near") {
                    let text = source_line(&source, line);
                    msg.extend_from_slice(format!("{name}: {dash_c}line {line}: `").as_bytes());
                    msg.extend_from_slice(text);
                    msg.extend_from_slice(b"'\n");
                }
                vp.error_raw(&msg).await;
                vp.status = 2;
                return (2, vp);
            }
        }
    }
    let st = vp.status;
    (st, vp)
}

fn source_line(source: &[u8], line: u32) -> &[u8] {
    let l = source
        .split(|&b| b == b'\n')
        .nth(line as usize - 1)
        .unwrap_or(b"");
    l.strip_suffix(b"\r").unwrap_or(l)
}

/// Runs a command string on the test host's empty fs: for quick checks and doc tests.
#[doc(hidden)]
pub fn run_on_test_host(source: &str) -> (i32, Vec<u8>, Vec<u8>) {
    let host = Rc::new(testhost::TestHost::new());
    let mut shell = Shell::new(host.config());
    shell.start(Script::command(source));
    let status = loop {
        match shell.step(1000) {
            Step::Done(s) => break s,
            Step::Yield | Step::Blocked => {}
            Step::Idle => break 0,
        }
    };
    (status, host.stdout(), host.stderr())
}
