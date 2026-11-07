//! psh's in-process utilities (ARCH.md "psh-utils", D1080–D1083): implementations of
//! external programs (`head`, `cat`, `timeout`, ...) that run inside the shell, on the
//! engine's scheduler, natively and in wasm.
//!
//! Adding a utility: a module with `SPEC` (its options, D1082) and `async fn run(ctx,
//! argv) -> i32`, plus one entry in [`REGISTRY`] and one in [`SPECS`].

mod common;
mod fsutil;
mod modes;
pub mod opts;
mod timefmt;
mod textio;

mod cat;
mod env;
mod head;
mod sleep;
mod tail;
mod timeout;
mod wc;

mod basename;
mod dirname;
mod mkdir;
mod readlink;
mod realpath;
mod rmdir;
mod touch;

mod comm;
mod cut;
mod nl;
mod paste;
mod seq;
mod sort;
mod tee;
mod tr;
mod uniq;

use psh_engine::util::{Mode, UtilEntry, UtilPolicy};

use common::util_fn;

/// Every utility, with its names, mode (D015) and group.
pub static REGISTRY: &[UtilEntry] = &[
    UtilEntry {
        names: &["timeout", "gtimeout"],
        mode: Mode::WindowsShadow,
        destructive: false,
        run: util_fn!(timeout::run),
    },
    UtilEntry {
        names: &["head"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(head::run),
    },
    UtilEntry {
        names: &["tail"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(tail::run),
    },
    UtilEntry {
        names: &["cat"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(cat::run),
    },
    UtilEntry {
        names: &["wc"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(wc::run),
    },
    UtilEntry {
        names: &["env"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(env::run),
    },
    UtilEntry {
        names: &["sleep"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(sleep::run),
    },
    UtilEntry {
        names: &["basename"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(basename::run),
    },
    UtilEntry {
        names: &["dirname"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(dirname::run),
    },
    UtilEntry {
        names: &["realpath"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(realpath::run),
    },
    UtilEntry {
        names: &["readlink"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(readlink::run),
    },
    UtilEntry {
        names: &["mkdir"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(mkdir::run),
    },
    UtilEntry {
        names: &["rmdir"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(rmdir::run),
    },
    UtilEntry {
        names: &["touch"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(touch::run),
    },
    UtilEntry {
        names: &["cut"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(cut::run),
    },
    UtilEntry {
        names: &["paste"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(paste::run),
    },
    UtilEntry {
        names: &["seq"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(seq::run),
    },
    UtilEntry {
        names: &["tee"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(tee::run),
    },
    UtilEntry {
        names: &["tr"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(tr::run),
    },
    UtilEntry {
        names: &["uniq"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(uniq::run),
    },
    UtilEntry {
        names: &["comm"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(comm::run),
    },
    UtilEntry {
        names: &["nl"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(nl::run),
    },
    UtilEntry {
        names: &["sort"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(sort::run),
    },
];

/// The option specs, for the classifier check (D1082).
pub static SPECS: &[opts::Spec] = &[
    timeout::SPEC,
    head::SPEC,
    tail::SPEC,
    cat::SPEC,
    wc::SPEC,
    sleep::SPEC,
    env::SPEC,
    basename::SPEC,
    dirname::SPEC,
    realpath::SPEC,
    readlink::SPEC,
    mkdir::SPEC,
    rmdir::SPEC,
    touch::SPEC,
    cut::SPEC,
    paste::SPEC,
    seq::SPEC,
    tee::SPEC,
    tr::SPEC,
    uniq::SPEC,
    comm::SPEC,
    nl::SPEC,
    sort::SPEC,
];

/// The policy for a shell: every utility on. `sandbox`: no real programs exist (wasm),
/// so `F` and `W` utilities always run.
pub fn policy(sandbox: bool) -> UtilPolicy {
    UtilPolicy {
        entries: REGISTRY,
        sandbox,
        ..UtilPolicy::default()
    }
}

#[cfg(test)]
mod tests;
