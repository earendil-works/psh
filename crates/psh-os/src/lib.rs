//! The OS hosts of psh: the real processes, pipes and files behind the engine's `Host`
//! trait (ARCH.md "psh-os", research/execution-core.md §5).
//!
//! - [`unix`]: macOS, Linux and Android.
//! - [`windows`]: Windows (x64 and arm64).
//!
//! The platform-neutral parts ([`lifecycle`], the timer thread, [`Leftovers`]) live
//! outside the platform modules and are shared by both hosts.

pub mod lifecycle;
mod timer;
#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

pub use timer::sleep;

/// The fate of a run's leftovers when its host is dropped (D1067).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leftovers {
    /// TERM to every group of the run, KILL after the grace period. Only detached
    /// (`nohup`, `setsid`) and disowned externals survive. The default.
    Kill,
    /// Leave them running, as bash does.
    Keep,
}
