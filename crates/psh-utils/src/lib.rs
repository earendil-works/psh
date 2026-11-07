//! psh's in-process utilities (ARCH.md "psh-utils", D1080–D1082): implementations of
//! external programs (`head`, `cat`, `timeout`, ...) that run inside the shell, on the
//! engine's scheduler, natively and in wasm.

pub mod opts;
