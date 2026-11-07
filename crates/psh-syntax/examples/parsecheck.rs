//! Development tool: parses files and reports the verdict of each.
//!
//! `cargo run -p psh-syntax --example parsecheck -- [--dump|--print|--roundtrip] FILE...`
//! prints `PATH\tok` or `PATH\terr\tMESSAGE` per file; `-c SCRIPT` parses a string.

use std::io::Write;

fn main() {
    let mut mode = "";
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--dump" | "--print" | "--roundtrip" => {
                mode = if a == "--dump" {
                    "dump"
                } else if a == "--print" {
                    "print"
                } else {
                    "roundtrip"
                }
            }
            "-c" => inputs.push(("-c".into(), args.next().expect("-c SCRIPT").into_bytes())),
            "--bench" => mode = "bench",
            _ => {
                let bytes = std::fs::read(&a).expect("read");
                inputs.push((a, bytes));
            }
        }
    }
    if mode == "bench" {
        // Parse every input 20 times; report the time per full pass.
        let bytes: usize = inputs.iter().map(|(_, s)| s.len()).sum();
        let start = std::time::Instant::now();
        let mut n = 0usize;
        for _ in 0..20 {
            for (_, src) in &inputs {
                n += psh_syntax::parse_bytes(src, Default::default()).map_or(0, |a| a.cmd_count());
            }
        }
        let per_pass = start.elapsed() / 20;
        println!(
            "{} inputs, {bytes} bytes: {per_pass:?} per pass ({n} commands)",
            inputs.len()
        );
        return;
    }
    let out = std::io::stdout();
    let mut out = out.lock();
    for (name, src) in inputs {
        match psh_syntax::parse_bytes(&src, Default::default()) {
            Ok(ast) => match mode {
                "dump" => writeln!(out, "{}", psh_syntax::dump(&ast)).unwrap(),
                "print" => write!(out, "{}", psh_syntax::print(&ast)).unwrap(),
                "roundtrip" => {
                    let printed = psh_syntax::print(&ast);
                    match psh_syntax::parse_bytes(&ast.restore_bytes(&printed), Default::default())
                    {
                        Ok(again) if psh_syntax::dump(&again) == psh_syntax::dump(&ast) => {
                            writeln!(out, "{name}\tok").unwrap()
                        }
                        Ok(again) => writeln!(
                            out,
                            "{name}\tmismatch\n  {}\n  {}\n---\n{printed}---",
                            psh_syntax::dump(&ast),
                            psh_syntax::dump(&again)
                        )
                        .unwrap(),
                        Err(e) => writeln!(
                            out,
                            "{name}\treparse-error\t{}\n---\n{printed}---",
                            e.message
                        )
                        .unwrap(),
                    }
                }
                _ => writeln!(out, "{name}\tok").unwrap(),
            },
            Err(e) => writeln!(out, "{name}\terr\t{}", e.message.replace('\n', " / ")).unwrap(),
        }
    }
}
