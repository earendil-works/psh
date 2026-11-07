//! Running cases: the run directory (helpers, the Oils tree copy), each case's
//! environment and sandbox, parallel execution, and normalization of what came out.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::case::{Case, Layer};
use crate::engine::{Engine, RunOutput, RunRequest, Status};
use crate::helpers;
use crate::normalize::Paths;
use crate::sandbox::{Entry, FileEntry, Sandbox};

/// The per-run temporary directory: `bin/` (helpers, first on `PATH`), `repo/` (a copy
/// of the vendored Oils tree, `$REPO_ROOT`), `cases/` (one sandbox per case).
pub struct RunDir {
    pub root: PathBuf,
    pub bin: PathBuf,
    pub repo: PathBuf,
    pub cases: PathBuf,
    pub keep: bool,
    /// The host's name, full and up to the first dot, for normalization.
    pub hostnames: Vec<String>,
}

impl RunDir {
    pub fn create(repo_root: &Path, keep: bool) -> Result<RunDir, String> {
        let base = std::env::temp_dir().join(format!("psh-spec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).map_err(|e| format!("{}: {e}", base.display()))?;
        // Canonical, so the paths the shell prints (`pwd -P`) match ours.
        let root = std::fs::canonicalize(&base).map_err(|e| e.to_string())?;
        let bin = root.join("bin");
        let repo = root.join("repo");
        let cases = root.join("cases");
        for d in [&bin, &repo, &cases] {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        for name in helpers::NAMES {
            link_exe(&exe, &bin, name).map_err(|e| format!("helper {name}: {e}"))?;
        }
        // The Oils tree: spec/ with its non-Python helpers also in bin/.
        let oils = repo_root.join("tests/oils");
        if oils.join("spec").is_dir() {
            copy_tree(&oils.join("spec"), &repo.join("spec")).map_err(|e| e.to_string())?;
            // Cases also call the helpers by path (`spec/bin/argv.py`).
            for name in helpers::NAMES {
                let target = repo.join("spec/bin").join(name);
                if target.exists() {
                    std::fs::remove_file(&target).map_err(|e| e.to_string())?;
                    link_exe(&exe, &repo.join("spec/bin"), name)
                        .map_err(|e| format!("helper {name}: {e}"))?;
                }
            }
            if let Ok(entries) = std::fs::read_dir(oils.join("spec/bin")) {
                for e in entries.flatten() {
                    let name = e.file_name();
                    if !name.to_string_lossy().ends_with(".py") {
                        std::fs::copy(e.path(), bin.join(&name)).map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        let full = std::process::Command::new("hostname")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let short = full.split('.').next().unwrap_or("").to_string();
        // Too short a name would replace ordinary text.
        let hostnames = [full, short].into_iter().filter(|h| h.len() >= 4).collect();
        Ok(RunDir {
            root,
            bin,
            repo,
            cases,
            keep,
            hostnames,
        })
    }
}

impl Drop for RunDir {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

#[cfg(unix)]
fn link_exe(exe: &Path, bin: &Path, name: &str) -> std::io::Result<()> {
    std::os::unix::fs::symlink(exe, bin.join(name))
}

#[cfg(not(unix))]
fn link_exe(exe: &Path, bin: &Path, name: &str) -> std::io::Result<()> {
    std::fs::copy(exe, bin.join(format!("{name}.exe"))).map(|_| ())
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let target = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_tree(&e.path(), &target)?;
        } else {
            std::fs::copy(e.path(), &target)?;
        }
    }
    Ok(())
}

/// What one run of a case produced, normalized.
#[derive(Debug, Clone)]
pub struct Observed {
    pub output: RunOutput,
    pub files: Vec<FileEntry>,
    /// `bash -n`'s status and stderr, when asked for (recording).
    pub parse: Option<(i32, Vec<u8>)>,
}

pub struct Runner<'a> {
    pub engine: &'a dyn Engine,
    pub dir: &'a RunDir,
    pub timeout: Option<Duration>,
    /// Also run `SHELL -n` on the script (for the goldens' L0 verdict).
    pub parse_check: bool,
    /// Recording the BSD goldens (macOS): only the system's own directories on PATH,
    /// so no GNU tool from /usr/local or Homebrew answers.
    pub bsd: bool,
}

const SYSTEM_PATH: &str = "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";
const BSD_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";

impl Runner<'_> {
    fn env(&self, case: &Case, sb: &Sandbox) -> Vec<(String, String)> {
        // Paths as the shell writes them: on Windows `C:/x`, without the `\\?\` of
        // canonical paths, which no path joined to with `/` would survive (D1914).
        let s = |p: &Path| {
            let t = p.to_string_lossy().into_owned();
            if cfg!(windows) {
                t.strip_prefix(r"\\?\").unwrap_or(&t).replace('\\', "/")
            } else {
                t
            }
        };
        let home = match &case.home_dir {
            Some(rel) => sb.cwd.join(rel),
            None => sb.home.clone(),
        };
        let sep = if cfg!(windows) { ";" } else { ":" };
        let mut env = vec![
            (
                "PATH".to_string(),
                format!(
                    "{}{sep}{}",
                    s(&self.dir.bin),
                    if self.bsd { BSD_PATH } else { SYSTEM_PATH }
                ),
            ),
            ("HOME".to_string(), s(&home)),
            ("USER".to_string(), "spec".to_string()),
            ("LOGNAME".to_string(), "spec".to_string()),
            ("TZ".to_string(), "UTC".to_string()),
        ];
        match case.layer {
            Layer::Oils => {
                env.push(("LC_ALL".into(), "C.UTF-8".into()));
                env.push(("TMP".into(), s(&sb.cwd)));
                env.push(("SH".into(), s(self.engine.binary())));
                env.push(("REPO_ROOT".into(), s(&self.dir.repo)));
            }
            Layer::Brush => {
                env.push(("LC_ALL".into(), "C".into()));
                env.push(("PS1".into(), "test$ ".into()));
            }
            Layer::Agent => {
                env.push(("LC_ALL".into(), "C.UTF-8".into()));
            }
            Layer::Utils => {
                env.push(("LC_ALL".into(), "C.UTF-8".into()));
                // The cases test psh's utilities, also those that defer to a program on
                // PATH (D1192). Only psh reads (and drops) it; a recording shell would
                // pass it on.
                if self.engine.name() == "psh" {
                    env.push((psh_utils::FORCE_VAR.into(), "all".into()));
                }
            }
        }
        for (k, v) in &case.env {
            env.retain(|(name, _)| name != k);
            env.push((k.clone(), v.clone()));
        }
        env
    }

    pub fn run(&self, case: &Case, n: usize) -> Result<Observed, String> {
        // Serial cases hold a machine-wide lock: they share fixed paths with each other,
        // across workers and across concurrent psh-spec runs (gates in other worktrees).
        let _serial = if case.serial {
            Some(serial_lock()?)
        } else {
            None
        };
        let sb = Sandbox::create(&self.dir.cases, n, case).map_err(|e| format!("sandbox: {e}"))?;
        let env = self.env(case, &sb);
        let timeout = self.timeout.unwrap_or(case.timeout);
        let result = self.engine.run(&RunRequest {
            args: &case.args,
            stdin: case.stdin.as_deref(),
            env: &env,
            cwd: &sb.cwd,
            timeout,
        });
        let output = match result {
            Ok(o) => o,
            Err(e) => {
                if !self.dir.keep {
                    sb.remove();
                }
                return Err(format!("running {}: {e}", self.engine.binary().display()));
            }
        };
        let files = sb.snapshot(case);
        let parse = if self.parse_check {
            let args = ["-n".to_string()];
            let out = self
                .engine
                .run(&RunRequest {
                    args: &args,
                    stdin: Some(case.script.as_bytes()),
                    env: &env,
                    cwd: &sb.cwd,
                    timeout,
                })
                .map_err(|e| format!("{} -n: {e}", self.engine.binary().display()))?;
            let status = match out.status {
                Status::Exit(n) => n,
                _ => -1,
            };
            Some((status, out.stderr))
        } else {
            None
        };
        let home = match &case.home_dir {
            Some(rel) => sb.cwd.join(rel),
            None => sb.home.clone(),
        };
        let hosts: Vec<(String, &'static str)> = self
            .dir
            .hostnames
            .iter()
            .map(|h| (h.clone(), "<host>"))
            .collect();
        let paths = Paths::new(
            &[
                (&sb.cwd, "<case>"),
                (&home, "<home>"),
                (&self.dir.repo, "<repo>"),
                (&self.dir.bin, "<bin>"),
                (&self.dir.root, "<run>"),
                (self.engine.binary(), "<sh>"),
            ],
            &hosts,
        );
        if !self.dir.keep {
            sb.remove();
        }
        Ok(Observed {
            output: RunOutput {
                stdout: paths.apply(&output.stdout),
                stderr: paths.apply(&output.stderr),
                status: output.status,
                notes: output.notes,
                leaked: output.leaked,
            },
            files: files
                .into_iter()
                .map(|f| FileEntry {
                    path: f.path,
                    entry: match f.entry {
                        Entry::File(d) => Entry::File(paths.apply(&d)),
                        Entry::Link(t) => Entry::Link(paths.apply(&t)),
                        other => other,
                    },
                })
                .collect(),
            parse: parse.map(|(s, e)| (s, paths.apply(&e))),
        })
    }
}

/// The machine-wide lock of serial cases: an exclusive lock on a file in the temporary
/// directory, released when the returned file is dropped.
fn serial_lock() -> Result<std::fs::File, String> {
    let path = std::env::temp_dir().join("psh-spec-serial.lock");
    // Read and write access: Windows refuses to lock a handle opened for appending.
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.lock()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(file)
}

/// Maps `f` over `items` on `jobs` threads, keeping the order.
pub fn parallel_map<T: Sync, R: Send>(
    items: &[T],
    jobs: usize,
    f: impl Fn(usize, &T) -> R + Sync,
) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let mut results: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    let slots: Vec<std::sync::Mutex<&mut Option<R>>> =
        results.iter_mut().map(std::sync::Mutex::new).collect();
    std::thread::scope(|s| {
        for _ in 0..jobs.max(1) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= items.len() {
                        break;
                    }
                    let r = f(i, &items[i]);
                    **slots[i].lock().unwrap() = Some(r);
                }
            });
        }
    });
    drop(slots);
    results.into_iter().map(Option::unwrap).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_lock_excludes_other_holders() {
        let inside = AtomicUsize::new(0);
        let max = AtomicUsize::new(0);
        parallel_map(&[(); 8], 8, |_, _| {
            let _lock = serial_lock().unwrap();
            let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
            max.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(20));
            inside.fetch_sub(1, Ordering::SeqCst);
        });
        assert_eq!(max.load(Ordering::SeqCst), 1);
    }
}
