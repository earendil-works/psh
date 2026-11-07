//! The OS hosts of psh: the real processes, pipes and files behind the engine's `Host`
//! trait (ARCH.md "psh-os", research/execution-core.md §5).
//!
//! - [`unix`]: macOS, Linux and Android.
//! - Windows: 0007.
//!
//! The platform-neutral parts ([`lifecycle`], the timer thread) live outside the
//! platform modules, for the Windows host to share.

pub mod lifecycle;
mod timer;
#[cfg(unix)]
pub mod unix;

pub use timer::sleep;
