//! Development tool for the bash-format printer.
//!
//! `fncheck extract OUTDIR FILE...` writes every function definition found in the files
//! to `OUTDIR/N.sh` (its source, with here-doc bodies) and `OUTDIR/N.name`.
//! `fncheck print FILE...` prints, for each file, its first command in `declare -f`
//! format followed by a line `=====`.

use psh_syntax::{Ast, CommandKind, RedirTarget};

fn snippet(ast: &Ast, id: psh_syntax::CmdId) -> String {
    let c = ast.cmd(id);
    let mut end = c.span.end as usize;
    // Here-doc bodies of commands inside the function may follow its last line.
    for cid in ast.cmd_ids() {
        let cmd = ast.cmd(cid);
        for r in &cmd.redirects {
            if let RedirTarget::HereDoc(h) = r.target {
                let h = ast.heredoc(h);
                if c.span.contains(h.delimiter.span) {
                    // The body and the delimiter line after it.
                    let body_end = h.body.span.end as usize;
                    let rest = &ast.source()[body_end..];
                    end = end.max(body_end + rest.find('\n').map_or(rest.len(), |n| n + 1));
                }
            }
        }
    }
    ast.source()[c.span.start as usize..end].to_string()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args[0].as_str() {
        "extract" => {
            let out = std::path::Path::new(&args[1]);
            std::fs::create_dir_all(out).unwrap();
            let mut n = 0;
            for f in &args[2..] {
                let Ok(src) = std::fs::read_to_string(f) else {
                    continue;
                };
                let Ok(ast) = psh_syntax::parse(&src) else {
                    continue;
                };
                for id in ast.cmd_ids() {
                    if let CommandKind::Function(func) = &ast.cmd(id).kind {
                        let Some(name) = psh_syntax::literal_text(&ast, &func.name) else {
                            continue;
                        };
                        if !name.bytes().all(|c| {
                            c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'.'
                        }) {
                            continue;
                        }
                        std::fs::write(out.join(format!("{n:05}.sh")), snippet(&ast, id) + "\n")
                            .unwrap();
                        std::fs::write(out.join(format!("{n:05}.name")), &name).unwrap();
                        n += 1;
                    }
                }
            }
            eprintln!("{n} functions");
        }
        "print" => {
            for f in &args[1..] {
                let src = std::fs::read_to_string(f).unwrap();
                match psh_syntax::parse(&src) {
                    Ok(ast) => {
                        let item = &ast.list(ast.root()).items[0];
                        let id = item.first.stages[0].cmd;
                        println!("{}", psh_syntax::print_function_bash(&ast, id));
                    }
                    Err(e) => println!("PARSE ERROR {e}"),
                }
                println!("=====");
            }
        }
        _ => panic!("usage"),
    }
}
