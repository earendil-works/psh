//! Classifies NUL-separated commands from stdin and prints one tab-separated line each:
//! primary category, all categories, read-only, parse error, the first unknown reason,
//! and the names of unknown commands. Used by tests/classify/corpus_accuracy.py.

use std::io::{BufWriter, Read, Write};

use psh_classify::{Category, classify};

fn main() {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).expect("stdin");
    let mut out = BufWriter::new(std::io::stdout().lock());
    for chunk in input.split(|&b| b == 0) {
        let c = classify(&String::from_utf8_lossy(chunk));
        let cats: Vec<&str> = c.categories.iter().map(|c| c.name()).collect();
        let reason = c.commands.iter().find_map(|x| x.reason).unwrap_or("");
        let unknown: Vec<&str> = c
            .commands
            .iter()
            .filter(|x| x.categories.contains(Category::Unknown))
            .map(|x| x.name.as_deref().unwrap_or("<dynamic>"))
            .collect();
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{reason}\t{}",
            c.primary().name(),
            cats.join(","),
            c.read_only(),
            c.error.is_some(),
            unknown.join(",")
        )
        .expect("stdout");
    }
}
