//! Development tool for differential tests: reads lines `FLAGS HEXPATTERN HEXTEXT`
//! (FLAGS: E extended, I icase, N newline, B not-bol, U UTF-8, `-` none) and prints what
//! `regcomp`/`regexec` would: `ERR n`, `NOMATCH`, or `so,eo so,eo …` (unset: `-1,-1`).

use std::io::BufRead;

fn unhex(h: &str) -> Vec<u8> {
    (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn main() {
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut f = line.split(' ');
        let flags = f.next().unwrap_or("");
        let pat = unhex(f.next().unwrap_or(""));
        let text = unhex(f.next().unwrap_or(""));
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
        match psh_regex::Regex::new(&pat, opts) {
            Err(e) => println!("ERR {}", e.kind as i32),
            Ok(re) => match re.exec(&text, eopts) {
                None => println!("NOMATCH"),
                Some(caps) => {
                    let parts: Vec<String> = caps
                        .iter()
                        .map(|c| match c {
                            Some((a, b)) => format!("{a},{b}"),
                            None => "-1,-1".into(),
                        })
                        .collect();
                    println!("{}", parts.join(" "));
                }
            },
        }
    }
}
