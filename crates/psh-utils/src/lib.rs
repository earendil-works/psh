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
pub mod search;
mod sum_hash;
mod textio;
mod timefmt;

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

mod chmod;
// Formatted in ripgrep's style (rg/rustfmt.toml), not the workspace's, so the copied code stays a
// small diff from upstream.
#[rustfmt::skip]
mod rg;

mod comm;
mod cp;
mod cut;
mod ln;
mod mv;
mod nl;
mod paste;
mod rm;
mod seq;
mod sort;
mod tee;
mod tr;
mod uniq;

mod find;
mod xargs;

mod base64;
mod cmp;
mod sum;
mod which;

use psh_engine::util::{Mode, UtilEntry, UtilPolicy};

use common::util_fn;

/// Every utility, with its names, mode (D015) and group.
pub static REGISTRY: &[UtilEntry] = &[
    UtilEntry {
        names: &["find"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(find::run),
    },
    UtilEntry {
        names: &["xargs"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(xargs::run),
    },
    UtilEntry {
        names: &["timeout", "gtimeout"],
        mode: Mode::Always,
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
    UtilEntry {
        names: &["rm"],
        mode: Mode::Always,
        destructive: true,
        run: util_fn!(rm::run),
    },
    UtilEntry {
        names: &["mv"],
        mode: Mode::Always,
        destructive: true,
        run: util_fn!(mv::run),
    },
    UtilEntry {
        names: &["cp"],
        mode: Mode::Always,
        destructive: true,
        run: util_fn!(cp::run),
    },
    UtilEntry {
        names: &["ln"],
        mode: Mode::Always,
        destructive: true,
        run: util_fn!(ln::run),
    },
    UtilEntry {
        names: &["chmod"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(chmod::run),
    },
    UtilEntry {
        names: &["rg"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(rg::run),
    },
    UtilEntry {
        names: &["base64"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(base64::run),
    },
    UtilEntry {
        names: &["cmp"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(cmp::run),
    },
    UtilEntry {
        names: &[
            "md5sum",
            "sha1sum",
            "sha224sum",
            "sha256sum",
            "sha384sum",
            "sha512sum",
        ],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(sum::run),
    },
    UtilEntry {
        names: &["md5"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(sum::run_md5),
    },
    UtilEntry {
        names: &["shasum"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(sum::run_shasum),
    },
    UtilEntry {
        names: &["which"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(which::run),
    },
    UtilEntry {
        names: &["printenv"],
        mode: Mode::Always,
        destructive: false,
        run: util_fn!(env::run_printenv),
    },
];

/// The option specs, for the classifier check (D1082).
pub static SPECS: &[opts::Spec] = &[
    timeout::SPEC,
    xargs::SPEC,
    find::SPEC,
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
    rm::SPEC,
    mv::SPEC,
    cp::SPEC,
    ln::SPEC,
    chmod::SPEC,
    base64::SPEC,
    cmp::SPEC,
    sum::SPEC,
    sum::MD5_SPEC,
    sum::SHASUM_SPEC,
    which::SPEC,
    env::PRINTENV_SPEC,
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
