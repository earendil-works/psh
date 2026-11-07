//! The utility API (D1080, D1081): in-process implementations of external programs
//! (`head`, `cat`, `timeout`, ... in psh-utils) run as async functions on the vproc that
//! runs the command, where shell builtins run. The embedder passes the registry in
//! [`crate::ShellConfig::utilities`]; the engine looks a name up after functions and
//! builtins and before `PATH`.
//!
//! Version 1 (`UTIL_API_VERSION`); a change to anything public here is a new decision.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::fd::{Fd, FdObj, bad_fd};
use crate::host::{DirEntry, Metadata, OpenOptions};
use crate::state::{R, Vp};

pub use crate::host::Access;

pub const UTIL_API_VERSION: u32 = 1;

/// A boxed future that stays on the engine's thread.
pub type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A utility: its arguments (argv[0] is the name it was called by) to its exit status.
pub type UtilFn = for<'a> fn(&'a mut UtilCtx<'_>, &'a [Vec<u8>]) -> LocalBoxFuture<'a, i32>;

/// When a utility runs instead of a program of the same name (D015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// S: always.
    Always,
    /// F: only if `PATH` has no such program (or the shell is a sandbox).
    PathFallback,
    /// W: always on Windows, as `PathFallback` elsewhere.
    WindowsShadow,
}

/// One utility of the registry.
#[derive(Clone, Copy)]
pub struct UtilEntry {
    /// The names it answers to (`timeout`, `gtimeout`).
    pub names: &'static [&'static str],
    pub mode: Mode,
    /// In the destructive group (`rm mv cp ln dd`), which can be turned off as one.
    pub destructive: bool,
    pub run: UtilFn,
}

/// The utilities of a shell and which are turned off.
#[derive(Clone, Default)]
pub struct UtilPolicy {
    pub entries: &'static [UtilEntry],
    /// No utility runs.
    pub off_all: bool,
    /// No utility of the destructive group runs.
    pub off_destructive: bool,
    /// These names run the real program (or are not found).
    pub off_names: Vec<Vec<u8>>,
    /// There are no real programs (wasm, a sandbox host): `PathFallback` and
    /// `WindowsShadow` utilities always run.
    pub sandbox: bool,
}

/// The utility that runs for `name`, if any: off switches and modes applied.
pub(crate) async fn lookup(vp: &Vp, name: &[u8]) -> Option<&'static UtilEntry> {
    // A path, or a name bound to a path with `hash -p` (or remembered from running the
    // program), names the program.
    if name.contains(&b'/') || crate::builtins::hash::hashed(vp, name).is_some() {
        return None;
    }
    let policy = &vp.core.utilities;
    if policy.off_all || policy.off_names.iter().any(|n| n == name) {
        return None;
    }
    let entries: &'static [UtilEntry] = policy.entries;
    let entry = entries
        .iter()
        .find(|e| e.names.iter().any(|n| n.as_bytes() == name))?;
    if entry.destructive && policy.off_destructive {
        return None;
    }
    let fallback = match entry.mode {
        Mode::Always => false,
        Mode::PathFallback => true,
        Mode::WindowsShadow => !cfg!(windows),
    };
    if fallback
        && !vp.core.utilities.sandbox
        && crate::builtins::hash::search(vp, name, None)
            .await
            .is_some()
    {
        return None;
    }
    Some(entry)
}

/// Runs a utility with the prefix assignments of its command line in its environment.
pub(crate) async fn run(
    vp: &mut Vp,
    entry: &'static UtilEntry,
    argv: Vec<Vec<u8>>,
    assigns: Vec<(String, Vec<u8>)>,
) -> R {
    let env = vp.env(&assigns);
    let name = String::from_utf8_lossy(&argv[0]).into_owned();
    let mut ctx = UtilCtx {
        vp,
        name,
        env,
        out: Vec::new(),
        write_error: None,
    };
    let status = (entry.run)(&mut ctx, &argv).await;
    // What is still buffered goes out now; a utility that cares about a failing final
    // write flushes itself before returning.
    let _ = ctx.flush().await;
    Ok(status)
}

/// stdout is flushed when the buffer holds this much (D1081).
const BUFFER: usize = 64 * 1024;

/// What a utility sees of the shell (D1081): its descriptors, cwd, environment and the
/// Host. Created by the engine for each run.
pub struct UtilCtx<'v> {
    vp: &'v mut Vp,
    name: String,
    env: Vec<(Vec<u8>, Vec<u8>)>,
    out: Vec<u8>,
    /// A failed stdout write (a flush before a read, say): every later write fails with
    /// it, so a utility writing into a closed pipe stops (EPIPE).
    write_error: Option<io::ErrorKind>,
}

/// How [`UtilCtx::spawn`] changes the environment of the command it runs.
#[derive(Clone, Debug, Default)]
pub struct SpawnEnv {
    /// Start from an empty environment (`env -i`).
    pub clear: bool,
    /// Variables to remove (`env -u NAME`).
    pub unset: Vec<String>,
    /// Variables to set, in order (`env NAME=value`).
    pub set: Vec<(String, Vec<u8>)>,
}

/// Shell builtins that also exist as programs, so wrappers (`env`, `timeout`, `xargs`)
/// run them (D015). Other builtins are not found from a wrapper, as in bash.
pub const STANDALONE_BUILTINS: &[&str] = &[
    "printf", "echo", "test", "[", "true", "false", "pwd", "kill",
];

/// A command started by [`UtilCtx::spawn`]; dropping it stops the command (its externals
/// are abandoned: TERM, then KILL).
pub struct UtilChild {
    fut: LocalBoxFuture<'static, i32>,
    status: Option<i32>,
}

impl UtilChild {
    /// Its exit status, once it ended.
    pub fn poll_wait(&mut self, cx: &mut Context<'_>) -> Poll<i32> {
        if let Some(s) = self.status {
            return Poll::Ready(s);
        }
        match self.fut.as_mut().poll(cx) {
            Poll::Ready(s) => {
                self.status = Some(s);
                Poll::Ready(s)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    pub async fn wait(&mut self) -> i32 {
        std::future::poll_fn(|cx| self.poll_wait(cx)).await
    }
}

/// An open file: one of the Host, or one of the shell's descriptors (`/dev/fd/N`,
/// `/dev/stdin`, as the engine emulates them, D1055).
pub struct UtilFile {
    file: Fd,
}

impl UtilFile {
    pub async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf).await
    }

    pub async fn read_to_end(&self, out: &mut Vec<u8>) -> io::Result<()> {
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            let n = self.read(&mut buf).await?;
            if n == 0 {
                return Ok(());
            }
            out.extend_from_slice(&buf[..n]);
        }
    }

    pub async fn write_all(&self, data: &[u8]) -> io::Result<()> {
        self.file.write_all(data).await
    }
}

impl UtilCtx<'_> {
    /// The name the utility was called by (`argv[0]`), for messages.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The shell's working directory (logical, absolute).
    pub fn cwd(&self) -> &[u8] {
        &self.vp.cwd
    }

    /// The environment the command sees, sorted by name.
    pub fn env(&self) -> &[(Vec<u8>, Vec<u8>)] {
        &self.env
    }

    /// One environment variable.
    pub fn var(&self, name: &[u8]) -> Option<&[u8]> {
        self.env
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_slice())
    }

    /// The locale is UTF-8 (`LC_ALL`, `LC_CTYPE`, `LANG`).
    pub fn utf8(&self) -> bool {
        let get = |n: &[u8]| self.var(n).filter(|v| !v.is_empty());
        let v = get(b"LC_ALL")
            .or_else(|| get(b"LC_CTYPE"))
            .or_else(|| get(b"LANG"));
        v.is_some_and(|v| {
            let v = v.to_ascii_lowercase();
            v.windows(5).any(|w| w == b"utf-8") || v.windows(4).any(|w| w == b"utf8")
        })
    }

    /// A path made absolute against the cwd.
    pub fn path(&self, p: &[u8]) -> Vec<u8> {
        self.vp.abs_path(p)
    }

    fn fd(&self, n: u32) -> Option<Fd> {
        self.vp.fd(n)
    }

    /// Whether descriptor `n` is a terminal.
    pub fn is_terminal(&self, n: u32) -> bool {
        matches!(self.fd(n).as_deref(), Some(FdObj::File(f)) if f.is_terminal())
    }

    /// Reads stdin (stdout is flushed first). `Ok(0)` is end of file.
    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let _ = self.flush().await;
        match self.fd(0) {
            Some(f) => f.read(buf).await,
            None => Err(bad_fd()),
        }
    }

    /// Reads stdin to its end.
    pub async fn read_to_end(&mut self, out: &mut Vec<u8>) -> io::Result<()> {
        let _ = self.flush().await;
        match self.fd(0) {
            Some(f) => f.read_to_end(out).await,
            None => Err(bad_fd()),
        }
    }

    /// Writes to stdout through the buffer (flushed when full, and at line ends when
    /// stdout is a terminal).
    pub async fn write(&mut self, data: &[u8]) -> io::Result<()> {
        if let Some(kind) = self.write_error {
            return Err(kind.into());
        }
        self.out.extend_from_slice(data);
        if self.out.len() >= BUFFER || (data.contains(&b'\n') && self.is_terminal(1)) {
            self.flush().await?;
        }
        Ok(())
    }

    /// Writes the buffer to stdout. The buffer is emptied even when the write fails.
    pub async fn flush(&mut self) -> io::Result<()> {
        if self.out.is_empty() {
            return Ok(());
        }
        let data = std::mem::take(&mut self.out);
        let r = match self.fd(1) {
            Some(f) => f.write_all(&data).await,
            None => Err(bad_fd()),
        };
        if let Err(e) = &r {
            self.write_error = Some(e.kind());
        }
        r
    }

    /// Writes `NAME: msg` and a newline to stderr (stdout is flushed first).
    pub async fn error(&mut self, msg: &str) {
        let text = format!("{}: {msg}\n", self.name);
        self.error_raw(text.as_bytes()).await;
    }

    /// Writes bytes to stderr as they are (stdout is flushed first).
    pub async fn error_raw(&mut self, data: &[u8]) {
        let _ = self.flush().await;
        if let Some(f) = self.fd(2) {
            let _ = f.write_all(data).await;
        }
    }

    /// Opens a file (relative paths against the cwd).
    pub async fn open(&self, path: &[u8], opts: OpenOptions) -> io::Result<UtilFile> {
        // The shell's own descriptors, as for redirections (D1055): process
        // substitutions are engine pipes no host knows.
        if let Some(n) = crate::exec::dev_fd(path) {
            return self.fd(n).map(|file| UtilFile { file }).ok_or_else(bad_fd);
        }
        let file = self.vp.host().open(&self.path(path), opts).await?;
        Ok(UtilFile {
            file: Rc::new(FdObj::File(file)),
        })
    }

    /// `stat` (`follow`) or `lstat`.
    pub async fn stat(&self, path: &[u8], follow: bool) -> io::Result<Metadata> {
        self.vp.host().stat(&self.path(path), follow).await
    }

    pub async fn read_dir(&self, path: &[u8]) -> io::Result<Vec<DirEntry>> {
        self.vp.host().read_dir(&self.path(path)).await
    }

    pub async fn access(&self, path: &[u8], mode: Access) -> io::Result<bool> {
        self.vp.host().access(&self.path(path), mode).await
    }

    pub async fn canonicalize(&self, path: &[u8]) -> io::Result<Vec<u8>> {
        self.vp.host().canonicalize(&self.path(path)).await
    }

    /// Completes after `duration` (cancellable: dropping it stops the wait).
    pub async fn sleep(&self, duration: Duration) {
        self.vp.host().sleep(duration).await;
    }

    /// A safepoint for long loops: yields when the step's budget is spent.
    pub async fn check(&self) {
        crate::sched::safepoint(&self.vp.core.sched).await;
    }

    /// Runs the real program of this name from `PATH` with these arguments, for an
    /// option the utility does not implement (D1082): its status, or `None` when there
    /// is no such program (or the shell is a sandbox, where none exist).
    pub async fn run_program(&mut self, argv: Vec<Vec<u8>>) -> Option<i32> {
        if self.vp.core.utilities.sandbox || argv.is_empty() {
            return None;
        }
        crate::builtins::hash::search(self.vp, &argv[0], None).await?;
        let _ = self.flush().await;
        let mut sub = self.vp.subshell();
        for (k, v) in &self.env {
            if let Ok(name) = std::str::from_utf8(k)
                && crate::state::is_name(k)
            {
                let var = sub.vars.entry(name);
                var.set(v.clone());
                var.exported = true;
            }
        }
        let r = crate::exec::run_external(&mut sub, argv, Vec::new()).await;
        let last = sub.status;
        Some(r.unwrap_or_else(|u| u.status(last)))
    }

    /// Starts a command for a wrapper (`env`, `timeout`, `xargs`, `find -exec`): a
    /// standalone builtin ([`STANDALONE_BUILTINS`]), a utility or an external, with this
    /// context's descriptors and cwd and the environment changed by `env`. stdout is
    /// flushed first. The command runs when the returned child is awaited.
    pub async fn spawn(&mut self, argv: Vec<Vec<u8>>, env: SpawnEnv) -> UtilChild {
        let _ = self.flush().await;
        let mut sub = self.vp.subshell();
        // The command sees the environment of the utility (prefix assignments
        // included), changed by `env`.
        if env.clear {
            let names: Vec<String> = sub.vars.iter().map(|(n, _)| n.clone()).collect();
            for n in names {
                if let Some(var) = sub.vars.get_mut(&n) {
                    var.exported = false;
                }
            }
            sub.foreign_env = Rc::new(Vec::new());
        } else {
            for (k, v) in &self.env {
                if let Ok(name) = std::str::from_utf8(k)
                    && crate::state::is_name(k)
                {
                    let var = sub.vars.entry(name);
                    var.set(v.clone());
                    var.exported = true;
                }
            }
        }
        for name in &env.unset {
            sub.vars.remove(name);
        }
        // Set as variables of the child, not as prefix assignments: the command is
        // looked up with them (`env PATH=/x cmd` searches /x).
        for (name, value) in env.set {
            let var = sub.vars.entry(&name);
            var.set(value);
            var.exported = true;
        }
        UtilChild {
            fut: Box::pin(async move {
                let mut sub = sub;
                let r = run_wrapped(&mut sub, argv, Vec::new()).await;
                let last = sub.status;
                r.unwrap_or_else(|u| u.status(last))
            }),
            status: None,
        }
    }
}

/// What a wrapper runs: a standalone builtin, a utility, or an external.
async fn run_wrapped(vp: &mut Vp, argv: Vec<Vec<u8>>, assigns: Vec<(String, Vec<u8>)>) -> R {
    if argv.is_empty() {
        return Ok(0);
    }
    let name = argv[0].clone();
    if STANDALONE_BUILTINS.iter().any(|b| b.as_bytes() == name) {
        return crate::exec::run_builtin(vp, argv, assigns).await;
    }
    if let Some(e) = lookup(vp, &name).await {
        return Box::pin(run(vp, e, argv, assigns)).await;
    }
    crate::exec::run_external(vp, argv, assigns).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testhost::TestHost;
    use crate::{Script, Shell, Step};

    /// `upper`: stdin to stdout, upper-cased. `fake-cat` (F mode): the test host also
    /// has `cat`, so only the off-PATH name runs in-process. `wrap CMD...`: runs its
    /// arguments through `spawn` with `W=1` set. `rmish`: destructive.
    fn upper<'a>(ctx: &'a mut UtilCtx<'_>, _argv: &'a [Vec<u8>]) -> LocalBoxFuture<'a, i32> {
        Box::pin(async move {
            let mut buf = [0u8; 3];
            loop {
                match ctx.read(&mut buf).await {
                    Ok(0) => return 0,
                    Ok(n) => {
                        if ctx.write(&buf[..n].to_ascii_uppercase()).await.is_err() {
                            return 141;
                        }
                    }
                    Err(_) => return 1,
                }
            }
        })
    }

    fn says<'a>(ctx: &'a mut UtilCtx<'_>, argv: &'a [Vec<u8>]) -> LocalBoxFuture<'a, i32> {
        Box::pin(async move {
            let name = ctx.name().to_string();
            let v = ctx.var(b"W").map(<[u8]>::to_vec).unwrap_or_default();
            let _ = ctx
                .write(
                    format!("{name} {} W={}\n", argv.len(), String::from_utf8_lossy(&v)).as_bytes(),
                )
                .await;
            ctx.error("to stderr").await;
            0
        })
    }

    fn wrap<'a>(ctx: &'a mut UtilCtx<'_>, argv: &'a [Vec<u8>]) -> LocalBoxFuture<'a, i32> {
        Box::pin(async move {
            let env = SpawnEnv {
                set: vec![("W".into(), b"1".to_vec())],
                ..SpawnEnv::default()
            };
            let mut child = ctx.spawn(argv[1..].to_vec(), env).await;
            child.wait().await
        })
    }

    static ENTRIES: &[UtilEntry] = &[
        UtilEntry {
            names: &["upper"],
            mode: Mode::Always,
            destructive: false,
            run: upper,
        },
        UtilEntry {
            names: &["says", "cat"],
            mode: Mode::PathFallback,
            destructive: false,
            run: says,
        },
        UtilEntry {
            names: &["wrap"],
            mode: Mode::Always,
            destructive: false,
            run: wrap,
        },
        UtilEntry {
            names: &["rmish"],
            mode: Mode::Always,
            destructive: true,
            run: says,
        },
    ];

    fn run_with(src: &str, policy: UtilPolicy) -> (i32, String, String) {
        let host = Rc::new(TestHost::new());
        host.set_stdin(b"abc\ndef\n");
        let mut cfg = host.config();
        cfg.utilities = policy;
        let mut shell = Shell::new(cfg);
        shell.start(Script::command(src));
        let status = loop {
            match shell.step(7) {
                Step::Done(s) => break s,
                Step::Yield => {}
                other => panic!("{other:?} running {src:?}"),
            }
        };
        assert_eq!(shell.task_count(), 0);
        (
            status,
            String::from_utf8_lossy(&host.stdout()).into_owned(),
            String::from_utf8_lossy(&host.stderr()).into_owned(),
        )
    }

    fn on() -> UtilPolicy {
        UtilPolicy {
            entries: ENTRIES,
            ..UtilPolicy::default()
        }
    }

    #[test]
    fn dispatch_io_and_modes() {
        assert_eq!(run_with("upper", on()).1, "ABC\nDEF\n");
        assert_eq!(run_with("echo x | upper | upper", on()).1, "X\n");
        // F mode: `cat` is on PATH, so the program runs; `says` is not.
        assert_eq!(run_with("echo y | cat", on()).1, "y\n");
        assert_eq!(
            run_with("says a b", on()),
            (0, "says 3 W=\n".into(), "says: to stderr\n".into())
        );
        // A sandbox treats F as S.
        let sandbox = UtilPolicy {
            sandbox: true,
            ..on()
        };
        assert_eq!(run_with("cat", sandbox).1, "cat 1 W=\n");
        // A path always runs the program; functions win over utilities.
        assert_eq!(run_with("/bin/cat", on()).1, "abc\ndef\n");
        assert_eq!(run_with("upper() { echo fn; }; upper", on()).1, "fn\n");
        // Prefix assignments reach the utility's environment.
        assert_eq!(run_with("W=7 says", on()).1, "says 1 W=7\n");
    }

    #[test]
    fn off_switches() {
        let st = |src: &str, p: UtilPolicy| run_with(src, p).0;
        assert_eq!(
            st(
                "upper",
                UtilPolicy {
                    off_all: true,
                    ..on()
                }
            ),
            127
        );
        assert_eq!(
            st(
                "rmish",
                UtilPolicy {
                    off_destructive: true,
                    ..on()
                }
            ),
            127
        );
        assert_eq!(
            st(
                "upper",
                UtilPolicy {
                    off_destructive: true,
                    ..on()
                }
            ),
            0
        );
        assert_eq!(
            st(
                "upper",
                UtilPolicy {
                    off_names: vec![b"upper".to_vec()],
                    ..on()
                }
            ),
            127
        );
        assert_eq!(st("upper", UtilPolicy::default()), 127);
    }

    #[test]
    fn type_command_and_wrappers() {
        assert_eq!(
            run_with("type upper; command -v upper; type -t upper", on()).1,
            "upper is a psh utility\nupper\nbuiltin\n"
        );
        assert_eq!(run_with("command upper", on()).1, "ABC\nDEF\n");
        // `exec` into a utility: it runs, then the shell ends with its status.
        assert_eq!(
            run_with("exec upper; echo not reached", on()),
            (0, "ABC\nDEF\n".into(), String::new())
        );
        assert_eq!(run_with("(exec says); echo $?", on()).1, "says 1 W=\n0\n");
        // A wrapper runs utilities, standalone builtins and programs, not functions.
        assert_eq!(run_with("wrap says", on()).1, "says 1 W=1\n");
        assert_eq!(
            run_with("wrap echo hi; wrap true; echo $?", on()).1,
            "hi\n0\n"
        );
        assert_eq!(run_with("f() { echo fn; }; wrap f; echo $?", on()).0, 0);
        assert!(
            run_with("f() { echo fn; }; wrap f", on())
                .2
                .contains("f: command not found")
        );
        assert_eq!(run_with("echo q | wrap cat", on()).1, "q\n");
    }
}
