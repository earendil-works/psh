//! The labelled cases in tests/classify/cases.txt (format in its header).

use psh_classify::{Category, classify};

#[test]
fn labelled_cases() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/classify/cases.txt"
    );
    let text = std::fs::read_to_string(path).expect("tests/classify/cases.txt");
    let mut failures = Vec::new();
    let mut count = 0;
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let primary = fields.next().expect("category");
        let cmd = fields
            .next()
            .unwrap_or_else(|| panic!("line {}: no command", n + 1));
        let tokens = fields.next().unwrap_or("").split_whitespace();
        let cmd = cmd.replace('␤', "\n");
        count += 1;
        let c = classify(&cmd);
        let mut problems = Vec::new();
        if primary == "error" {
            if c.error.is_none() {
                problems.push("expected a parse error".to_string());
            }
        } else {
            let want = Category::from_name(primary)
                .unwrap_or_else(|| panic!("line {}: category {primary}", n + 1));
            if c.primary() != want {
                problems.push(format!("primary {} != {primary}", c.primary().name()));
            }
        }
        for t in tokens {
            let ok = if let Some(cat) = t.strip_prefix('+') {
                c.categories
                    .contains(Category::from_name(cat).expect("category"))
            } else if let Some(cat) = t.strip_prefix('-') {
                !c.categories
                    .contains(Category::from_name(cat).expect("category"))
            } else if let Some(p) = t.strip_prefix("w:") {
                c.writes.iter().any(|w| w.path.as_deref() == Some(p))
            } else if let Some(p) = t.strip_prefix("r:") {
                c.reads.iter().any(|w| w.path.as_deref() == Some(p))
            } else if t == "nowrite" {
                c.writes.is_empty()
            } else if t == "trunc" {
                c.truncated
            } else if t == "notrunc" {
                !c.truncated
            } else if let Some(name) = t.strip_prefix("cmd:") {
                c.commands.iter().any(|x| x.name.as_deref() == Some(name))
            } else {
                panic!("line {}: unknown assertion {t}", n + 1)
            };
            if !ok {
                problems.push(format!("{t} does not hold"));
            }
        }
        if !problems.is_empty() {
            let cats: Vec<&str> = c.categories.iter().map(|c| c.name()).collect();
            failures.push(format!(
                "line {}: {cmd:?}: {} (got {cats:?}, writes {:?})",
                n + 1,
                problems.join(", "),
                c.writes.iter().map(|w| w.path.clone()).collect::<Vec<_>>()
            ));
        }
    }
    assert!(count >= 300, "only {count} cases");
    assert!(
        failures.is_empty(),
        "{} of {count} cases fail:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
