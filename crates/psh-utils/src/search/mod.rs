//! The search layer shared by `rg`, `grep`, `fd` (and later `find`, `sed`) on the
//! ripgrep crates (D020, ARCH "Search"): the POSIX pattern translator, and the walker
//! seam with its native (`ignore::Walk`) and virtual (`SyncCtx` file system) halves.

pub mod posix;
