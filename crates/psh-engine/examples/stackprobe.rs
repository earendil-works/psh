//! Measures the native stack the engine needs per nesting level (D1050):
//! `cargo run --release --example stackprobe -p psh-engine -- DEPTH STACK_BYTES` runs
//! DEPTH nested brace groups on a thread with that stack; it aborts with a stack
//! overflow if the stack is too small. Measured 2026-11: about 5 KiB per level in
//! release builds, 15 KiB in debug builds.

fn main() {
    let mut args = std::env::args().skip(1);
    let depth: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(95);
    let stack: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(1 << 20);
    let src = if std::env::args().nth(3).as_deref() == Some("func") {
        format!("f() {{ if [ $1 -lt {depth} ]; then f $(( $1 + 1 )); else echo deep; fi; }}; f 0")
    } else {
        format!("{}echo deep{}", "{ ".repeat(depth), "; }".repeat(depth))
    };
    let t = std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || psh_engine::run_on_test_host(&src))
        .expect("thread");
    let (status, out, _) = t.join().expect("join");
    println!("{status} {}", String::from_utf8_lossy(&out).trim());
}
