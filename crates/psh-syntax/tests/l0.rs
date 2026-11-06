//! L0 cases of the parser: `tests/l0/syntax.txt` holds snippets with bash's `bash -n`
//! verdict. The parser must agree on every verdict, and every case that parses must
//! survive print -> reparse with the same tree.

use std::path::Path;

struct Case {
    name: String,
    script: String,
    status: u32,
    message: Option<String>,
}

fn cases() -> Vec<Case> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/l0/syntax.txt");
    let text = std::fs::read_to_string(&path).expect("read tests/l0/syntax.txt");
    let mut cases = Vec::new();
    let mut current: Option<Case> = None;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("#### ") {
            cases.extend(current.take());
            current = Some(Case {
                name: name.to_string(),
                script: String::new(),
                status: u32::MAX,
                message: None,
            });
        } else if let Some(c) = current.as_mut() {
            if let Some(s) = line.strip_prefix("## status: ") {
                c.status = s.parse().expect("status");
            } else if let Some(m) = line.strip_prefix("## message: ") {
                c.message = Some(m.to_string());
            } else if c.status == u32::MAX {
                c.script.push_str(line);
                c.script.push('\n');
            }
        }
    }
    cases.extend(current);
    cases
}

#[test]
fn verdicts_agree_with_bash() {
    let cases = cases();
    assert!(cases.len() > 150);
    let mut failures = Vec::new();
    for case in &cases {
        let result = psh_syntax::parse(&case.script);
        let ok = result.is_ok();
        if ok != (case.status == 0) {
            failures.push(format!(
                "case {}: bash status {} ({}), psh {}\n{}",
                case.name,
                case.status,
                case.message.as_deref().unwrap_or(""),
                match &result {
                    Ok(_) => "parses".to_string(),
                    Err(e) => format!("error: {e}"),
                },
                case.script
            ));
        }
        if let Ok(ast) = result {
            let printed = psh_syntax::print(&ast);
            match psh_syntax::parse(&printed) {
                Ok(again) if psh_syntax::dump(&again) == psh_syntax::dump(&ast) => {}
                Ok(again) => failures.push(format!(
                    "case {}: round trip changed the tree\n{}\n{}\n{printed}",
                    case.name,
                    psh_syntax::dump(&ast),
                    psh_syntax::dump(&again)
                )),
                Err(e) => failures.push(format!(
                    "case {}: printed form does not parse: {e}\n{printed}",
                    case.name
                )),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
