//! `env [-i] [-u NAME]... [NAME=VALUE]... [COMMAND [ARG]...]`: prints the environment,
//! or runs a command with a changed one through the shell's dispatch (D1083: standalone
//! builtins, utilities, programs; not functions). `-0` ends printed entries with NUL.
//! GNU's `-C`, `-S` and the signal options fall through to the real program.

use psh_engine::util::{SpawnEnv, UtilCtx};

use crate::common;
use crate::opts::{Spec, both};

pub const SPEC: Spec = Spec {
    name: "env",
    opts: &[
        both('i', "ignore-environment"),
        both('u', "unset").value(),
        both('0', "null"),
    ],
    posix_order: true,
};

pub async fn run(ctx: &mut UtilCtx<'_>, argv: &[Vec<u8>]) -> i32 {
    // A lone `-` is `-i` (POSIX).
    let mut args: Vec<Vec<u8>> = argv.to_vec();
    let mut dash_i = false;
    if args.get(1).is_some_and(|a| a == b"-") {
        args.remove(1);
        dash_i = true;
    }
    let p = match SPEC.parse(&args[1..]) {
        Ok(p) => p,
        Err(e) => {
            let st = common::option_error(ctx, &SPEC, argv, &e).await;
            // GNU env's own errors are 125.
            return if st == 1 { 125 } else { st };
        }
    };
    let mut env = SpawnEnv {
        clear: dash_i,
        ..SpawnEnv::default()
    };
    let mut nul = false;
    for (key, value) in p.iter(&SPEC) {
        match key.as_str() {
            "ignore-environment" => env.clear = true,
            "unset" => env
                .unset
                .push(String::from_utf8_lossy(value.unwrap_or_default()).into_owned()),
            "null" => nul = true,
            _ => {}
        }
    }
    let mut rest = p.operands.as_slice();
    while let Some(a) = rest.first() {
        let Some(eq) = a.iter().position(|&b| b == b'=') else {
            break;
        };
        if eq == 0 {
            break;
        }
        env.set.push((
            String::from_utf8_lossy(&a[..eq]).into_owned(),
            a[eq + 1..].to_vec(),
        ));
        rest = &rest[1..];
    }
    if rest.is_empty() {
        // Print the resulting environment.
        let mut vars: Vec<(Vec<u8>, Vec<u8>)> = if env.clear {
            Vec::new()
        } else {
            ctx.env()
                .iter()
                .filter(|(k, _)| !env.unset.iter().any(|u| u.as_bytes() == k.as_slice()))
                .cloned()
                .collect()
        };
        for (k, v) in &env.set {
            vars.retain(|(n, _)| n != k.as_bytes());
            vars.push((k.as_bytes().to_vec(), v.clone()));
        }
        let mut out = Vec::new();
        for (k, v) in vars {
            out.extend_from_slice(&k);
            out.push(b'=');
            out.extend_from_slice(&v);
            out.push(if nul { 0 } else { b'\n' });
        }
        if let Err(e) = ctx.write(&out).await {
            return common::write_failed(ctx, &e).await;
        }
        return 0;
    }
    let mut child = ctx.spawn(rest.to_vec(), env).await;
    child.wait().await
}
