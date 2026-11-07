use alloc::vec::Vec;

use crate::*;

fn m(pat: &str, text: &str, ere: bool) -> Option<Vec<Option<(usize, usize)>>> {
    let opts = Options {
        extended: ere,
        utf8: true,
        ..Options::default()
    };
    Regex::new(pat.as_bytes(), opts)
        .unwrap()
        .exec(text.as_bytes(), ExecOptions::default())
}

#[test]
fn basics() {
    assert_eq!(m("b+", "abbbc", true), Some(alloc::vec![Some((1, 4))]));
    assert_eq!(
        m("(a|ab)(c|bcd)(d*)", "abcd", true),
        Some(alloc::vec![Some((0, 4)), Some((0, 1)), Some((1, 4)), Some((4, 4))])
    );
    assert_eq!(m("x", "abc", true), None);
}
