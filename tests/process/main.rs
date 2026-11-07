//! L5 process tests (TESTING.md): the engine over the real OS host, in-process. Every
//! test runs under a deadline and ends with a leak check: nothing the host started may
//! still be running (no live process group on Unix, no live job on Windows).
//!
//! - `common`: the cases for every platform, with `psh-test-helper` (`$H`) as the
//!   external program, since Windows has no `sleep`, `cat` or `yes`.
//! - `unix`: cases with the system's programs.
//! - `windows`: batch files, paths, `PATH` lookup of System32 programs.

mod common;
mod harness;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
