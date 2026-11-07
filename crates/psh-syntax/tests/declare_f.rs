//! `tests/l0/declare-f.txt`: function definitions with bash's `declare -f` output; the
//! bash-format printer must reproduce each byte for byte.

use std::path::Path;

#[test]
fn declare_f_matches_bash() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/l0/declare-f.txt");
    let text = std::fs::read_to_string(&path).expect("read tests/l0/declare-f.txt");
    let mut failures = Vec::new();
    let mut count = 0;
    for case in text.split("\n#### ").skip(1) {
        let (name, rest) = case.split_once('\n').unwrap();
        let (source, rest) = rest.split_once("## declare -f:\n").unwrap();
        let (expected, _) = rest.split_once("## end\n").unwrap();
        count += 1;
        let ast = match psh_syntax::parse(source) {
            Ok(ast) => ast,
            Err(e) => {
                failures.push(format!("case {name}: does not parse: {e}"));
                continue;
            }
        };
        let id = ast.list(ast.root()).items[0].first.stages[0].cmd;
        let printed = psh_syntax::print_function_bash(&ast, id) + "\n";
        if printed != expected {
            failures.push(format!(
                "case {name}:\n--- source\n{source}--- bash\n{expected}--- psh\n{printed}"
            ));
        }
    }
    assert!(count > 100, "{count} cases");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
