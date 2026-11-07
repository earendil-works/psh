//! Differential cases recorded from GNU libc's `regcomp`/`regexec` (in the recording
//! container): psh-regex must give the same result for every one, except the few marked
//! as known differences, where it must give its recorded result.

use std::path::Path;

fn unhex(h: &str) -> Vec<u8> {
    if h == "-" {
        return Vec::new();
    }
    (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn run(flags: &str, pat: &[u8], text: &[u8]) -> String {
    let opts = psh_regex::Options {
        extended: flags.contains('E'),
        icase: flags.contains('I'),
        newline: flags.contains('N'),
        utf8: flags.contains('U'),
    };
    let eopts = psh_regex::ExecOptions {
        not_bol: flags.contains('B'),
        not_eol: false,
    };
    match psh_regex::Regex::new(pat, opts) {
        Err(e) => format!("ERR {}", e.kind as i32),
        Ok(re) => match re.exec(text, eopts) {
            None => "NOMATCH".into(),
            Some(caps) => caps
                .iter()
                .map(|c| match c {
                    Some((a, b)) => format!("{a},{b}"),
                    None => "-1,-1".into(),
                })
                .collect::<Vec<_>>()
                .join(" "),
        },
    }
}

#[test]
fn agrees_with_gnu() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/gnu.txt");
    let text = std::fs::read_to_string(path).unwrap();
    let mut failures = Vec::new();
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let (known, line) = match line.strip_prefix("! ") {
            Some(rest) => (true, rest),
            None => (false, line),
        };
        let mut f = line.splitn(4, ' ');
        let (flags, pat, txt, rest) = (
            f.next().unwrap(),
            unhex(f.next().unwrap()),
            unhex(f.next().unwrap()),
            f.next().unwrap(),
        );
        let want = if known {
            rest.split(" | ").nth(1).unwrap()
        } else {
            rest
        };
        n += 1;
        let got = run(flags, &pat, &txt);
        if got != want {
            failures.push(format!(
                "{flags} {:?} {:?}: want {want}, got {got}",
                String::from_utf8_lossy(&pat),
                String::from_utf8_lossy(&txt)
            ));
        }
    }
    assert!(n > 4000, "{n} cases");
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
