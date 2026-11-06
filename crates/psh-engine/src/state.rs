//! The state of a virtual process: variables, positional parameters, cwd, descriptors,
//! `$?`. A subshell (pipeline stage, `( )`, `$(…)`) starts as a copy of its parent.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use psh_syntax::{Ast, LineIndex};

use crate::fd::{Fd, FdTable};
use crate::host::Host;
use crate::sched::Sched;

/// What the whole engine shares: the host, the scheduler, settings.
pub(crate) struct Core {
    pub host: Rc<dyn Host>,
    pub sched: Rc<Sched>,
    /// The name in front of messages: `$0` of the script.
    pub msg_name: RefCell<String>,
    /// The script runs as `-c` (bash words a few things differently then).
    pub dash_c: Cell<bool>,
    /// The nesting limit (D1050).
    pub max_depth: u32,
    /// Ids handed out for `$!` until real jobs exist.
    pub jobs: Cell<u32>,
}

impl Core {
    pub(crate) fn next_job_id(&self) -> u32 {
        self.jobs.set(self.jobs.get() + 1);
        // Looks like a pid, cannot be one of ours.
        100_000 + self.jobs.get()
    }
}

/// A parsed chunk plus the line table of its source, for `line N:` in messages.
pub(crate) struct Code {
    pub ast: Arc<Ast>,
    pub lines: Rc<LineIndex>,
    /// Added to line numbers: a backquote body parsed on its own counts from the
    /// line of its command.
    pub line_base: u32,
}

impl Code {
    pub(crate) fn line(&self, offset: u32) -> u32 {
        self.lines.line(offset) + self.line_base
    }
}

/// How control leaves a command other than by finishing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unwind {
    /// The (sub)shell ends with this status: `exit`, a fatal error, a signal.
    Exit(i32),
    // Functions and loops (later language work) unwind with these.
    #[allow(dead_code)]
    Return(i32),
    #[allow(dead_code)]
    Break(u32),
    #[allow(dead_code)]
    Continue(u32),
}

impl Unwind {
    /// The status when this reaches the top of a (sub)shell.
    pub(crate) fn status(self, last: i32) -> i32 {
        match self {
            Unwind::Exit(s) | Unwind::Return(s) => s,
            Unwind::Break(_) | Unwind::Continue(_) => last,
        }
    }
}

pub(crate) type R = Result<i32, Unwind>;

#[derive(Clone, Debug, Default)]
pub(crate) struct Var {
    /// `None`: declared (`export x`) but without a value.
    pub value: Option<Vec<u8>>,
    pub exported: bool,
    pub readonly: bool,
}

#[derive(Clone, Default)]
pub(crate) struct Vars {
    map: HashMap<String, Var>,
}

impl Vars {
    pub(crate) fn get(&self, name: &str) -> Option<&Var> {
        self.map.get(name)
    }

    pub(crate) fn value(&self, name: &str) -> Option<&[u8]> {
        self.map.get(name).and_then(|v| v.value.as_deref())
    }

    pub(crate) fn entry(&mut self, name: &str) -> &mut Var {
        self.map.entry(name.to_string()).or_default()
    }

    pub(crate) fn remove(&mut self, name: &str) -> Option<Var> {
        self.map.remove(name)
    }

    pub(crate) fn insert(&mut self, name: &str, var: Var) {
        self.map.insert(name.to_string(), var);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &Var)> {
        self.map.iter()
    }
}

/// Whether `s` is a valid variable name.
pub(crate) fn is_name(s: &[u8]) -> bool {
    !s.is_empty()
        && (s[0].is_ascii_alphabetic() || s[0] == b'_')
        && s.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// A virtual process: the shell or one of its subshells.
#[derive(Clone)]
pub(crate) struct Vp {
    pub core: Rc<Core>,
    pub vars: Vars,
    /// Environment entries inherited with names that are not shell variables; passed
    /// on to externals unchanged.
    pub foreign_env: Rc<Vec<(Vec<u8>, Vec<u8>)>>,
    pub arg0: Vec<u8>,
    pub args: Vec<Vec<u8>>,
    /// The logical working directory, absolute.
    pub cwd: Vec<u8>,
    pub fds: FdTable,
    /// `$?`
    pub status: i32,
    /// The line of the command running, for messages.
    pub line: u32,
    /// Nesting of subshells, substitutions and (later) function calls.
    pub depth: u32,
    /// `$!`
    pub last_bg: Option<u32>,
}

impl Vp {
    pub(crate) fn host(&self) -> &dyn Host {
        &*self.core.host
    }

    /// A copy for a subshell, one level deeper; `None` past the nesting limit.
    pub(crate) fn subshell(&self) -> Vp {
        let mut sub = self.clone();
        sub.depth += 1;
        sub
    }

    pub(crate) fn fd(&self, n: u32) -> Option<Fd> {
        self.fds.get(n).cloned()
    }

    pub(crate) fn var(&self, name: &str) -> Option<&[u8]> {
        self.vars.value(name)
    }

    pub(crate) fn set_var(&mut self, name: &str, value: Vec<u8>) {
        self.vars.entry(name).value = Some(value);
    }

    /// Whether the locale is UTF-8 (`LC_ALL`, `LC_CTYPE`, `LANG`, first set wins).
    pub(crate) fn utf8(&self) -> bool {
        for name in ["LC_ALL", "LC_CTYPE", "LANG"] {
            if let Some(v) = self.var(name)
                && !v.is_empty()
            {
                let v = v.to_ascii_lowercase();
                return v.windows(5).any(|w| w == b"utf-8") || v.windows(4).any(|w| w == b"utf8");
            }
        }
        // No locale at all: bash runs in the C locale.
        false
    }

    /// The environment for an external: exported variables with values, plus
    /// `extra` (assignments in front of the command), which win.
    pub(crate) fn env(&self, extra: &[(String, Vec<u8>)]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut env: Vec<(Vec<u8>, Vec<u8>)> = self.foreign_env.as_ref().clone();
        for (name, var) in self.vars.iter() {
            if var.exported
                && let Some(v) = &var.value
                && !extra.iter().any(|(n, _)| n == name)
            {
                env.push((name.as_bytes().to_vec(), v.clone()));
            }
        }
        for (n, v) in extra {
            env.push((n.as_bytes().to_vec(), v.clone()));
        }
        env.sort();
        env
    }

    /// Writes a message to fd 2, as `NAME: line N: MSG`. Write errors are ignored, as
    /// bash does.
    pub(crate) async fn error(&self, msg: &str) {
        let text = format!(
            "{}: line {}: {}\n",
            self.core.msg_name.borrow(),
            self.line,
            msg
        );
        self.error_raw(text.as_bytes()).await;
    }

    pub(crate) async fn error_raw(&self, text: &[u8]) {
        if let Some(fd) = self.fd(2) {
            let _ = fd.write_all(text).await;
        }
    }

    /// An absolute path for `p`, relative to the cwd.
    pub(crate) fn abs_path(&self, p: &[u8]) -> Vec<u8> {
        if p.first() == Some(&b'/') || is_windows_abs(p) {
            p.to_vec()
        } else {
            let mut out = self.cwd.clone();
            if out.last() != Some(&b'/') {
                out.push(b'/');
            }
            out.extend_from_slice(p);
            out
        }
    }
}

/// `C:/x` or `C:\x`.
fn is_windows_abs(p: &[u8]) -> bool {
    cfg!(windows)
        && p.len() >= 3
        && p[0].is_ascii_alphabetic()
        && p[1] == b':'
        && (p[2] == b'/' || p[2] == b'\\')
}
