//! Development tool: reads arithmetic texts, one per line. Prints for each either `ok`
//! or bash's error text; with `--eval`, evaluates `_r=(TEXT)` with x=3, y=-2,
//! a=(5 6 7) and prints `v=… x=… y=…` (or the error).

use std::collections::BTreeMap;
use std::io::BufRead;

use psh_syntax::{ArithEnv, ArithIndex, EvalError};

struct Env(BTreeMap<String, String>);

fn key(name: &[u8], index: Option<ArithIndex<'_>>) -> String {
    let name = String::from_utf8_lossy(name);
    match index {
        Some(ArithIndex::Num(n)) => format!("{name}[{}]", if n < 0 { n + 3 } else { n }),
        Some(ArithIndex::Key(k)) => format!("{name}[{}]", String::from_utf8_lossy(k)),
        None if name == "a" => "a[0]".into(),
        None => name.into_owned(),
    }
}

impl ArithEnv for Env {
    type Error = String;
    fn is_assoc(&mut self, _: &[u8]) -> bool {
        false
    }
    fn get(&mut self, n: &[u8], i: Option<ArithIndex<'_>>) -> Result<Option<Vec<u8>>, String> {
        Ok(self.0.get(&key(n, i)).map(|v| v.clone().into_bytes()))
    }
    fn set(&mut self, n: &[u8], i: Option<ArithIndex<'_>>, v: i64) -> Result<(), String> {
        self.0.insert(key(n, i), v.to_string());
        Ok(())
    }
}

fn main() {
    let eval = std::env::args().any(|a| a == "--eval");
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        if eval {
            let mut env = Env(BTreeMap::new());
            for (k, v) in [
                ("x", "3"),
                ("y", "-2"),
                ("a[0]", "5"),
                ("a[1]", "6"),
                ("a[2]", "7"),
            ] {
                env.0.insert(k.into(), v.into());
            }
            let text = format!("_r=({line})");
            match psh_syntax::eval_arith(text.as_bytes(), &mut env) {
                Ok(_) => println!(
                    "v={} x={} y={}",
                    env.0.get("_r").map_or("", |s| s),
                    env.0["x"],
                    env.0["y"]
                ),
                Err(EvalError::Arith { error, text }) => println!("{}", error.bash_text(&text)),
                Err(EvalError::Env(e)) => println!("{e}"),
            }
            continue;
        }
        match psh_syntax::parse_arith(line.as_bytes()) {
            Ok(_) => println!("ok"),
            Err(e) => println!("{}", e.bash_text(line.as_bytes())),
        }
    }
}
