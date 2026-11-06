//! Golden files: what bash 5.3 did for each case, one text file per source file.
//!
//! ```text
//! # header comment lines
//!
//! === 3: case name
//! status: 0
//! bash-n: 0
//! --- stdout
//! |a line of output
//! \an escaped line: \x1b[0m\r
//! ~ no newline at end
//! --- stderr
//! |bash: line 1: foo: command not found
//! --- file out.txt
//! |contents
//! --- dir sub
//! --- link l -> target
//! --- bigfile big.bin 100000 0123456789abcdef
//! --- other fifo
//! --- bash-n stderr
//! |...
//! ```
//!
//! Byte blocks are written line by line: `|` + the line when it is valid UTF-8 without
//! control characters other than tab, else `\` + the line with `\\` and `\xNN` escapes.
//! A block that does not end in a newline is followed by `~ no newline at end`. Empty
//! blocks are left out.

use crate::engine::Status;
use crate::sandbox::{Entry, FileEntry};

#[derive(Debug, Clone, PartialEq)]
pub struct Golden {
    pub index: usize,
    pub name: String,
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub files: Vec<FileEntry>,
    /// The status of `bash -n` on the case's script, and its stderr.
    pub parse_status: i32,
    pub parse_stderr: Vec<u8>,
}

const NO_EOL: &str = "~ no newline at end";

fn plain_line(line: &[u8]) -> bool {
    match std::str::from_utf8(line) {
        Ok(s) => !s
            .chars()
            .any(|c| (c.is_control() && c != '\t') || c == '\u{7f}'),
        Err(_) => false,
    }
}

pub fn write_block(out: &mut String, data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let ends_nl = data.ends_with(b"\n");
    let body = if ends_nl {
        &data[..data.len() - 1]
    } else {
        data
    };
    for line in body.split(|&b| b == b'\n') {
        if plain_line(line) {
            out.push('|');
            out.push_str(std::str::from_utf8(line).unwrap());
        } else {
            out.push('\\');
            for &b in line {
                match b {
                    b'\\' => out.push_str("\\\\"),
                    b'\t' => out.push('\t'),
                    0x20..=0x7e => out.push(b as char),
                    _ => out.push_str(&format!("\\x{b:02x}")),
                }
            }
        }
        out.push('\n');
    }
    if !ends_nl {
        out.push_str(NO_EOL);
        out.push('\n');
    }
}

fn unescape(line: &str) -> Result<Vec<u8>, String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            match bytes.get(i + 1) {
                Some(b'\\') => {
                    out.push(b'\\');
                    i += 2;
                }
                Some(b'x') => {
                    let hex = line.get(i + 2..i + 4).ok_or("short \\x escape")?;
                    out.push(u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?);
                    i += 4;
                }
                _ => return Err(format!("bad escape in {line:?}")),
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Ok(out)
}

pub fn write_file(header: &str, goldens: &[Golden]) -> String {
    let mut out = String::new();
    for line in header.lines() {
        out.push_str("# ");
        out.push_str(line);
        out.push('\n');
    }
    for g in goldens {
        out.push('\n');
        out.push_str(&format!("=== {}: {}\n", g.index, g.name.replace('\n', " ")));
        out.push_str(&format!("status: {}\n", g.status));
        out.push_str(&format!("bash-n: {}\n", g.parse_status));
        if !g.stdout.is_empty() {
            out.push_str("--- stdout\n");
            write_block(&mut out, &g.stdout);
        }
        if !g.stderr.is_empty() {
            out.push_str("--- stderr\n");
            write_block(&mut out, &g.stderr);
        }
        for f in &g.files {
            match &f.entry {
                Entry::File(data) => {
                    out.push_str(&format!("--- file {}\n", f.path));
                    write_block(&mut out, data);
                }
                Entry::BigFile { size, hash } => {
                    out.push_str(&format!("--- bigfile {} {size} {hash:016x}\n", f.path));
                }
                Entry::Dir => out.push_str(&format!("--- dir {}\n", f.path)),
                Entry::Link(target) => {
                    out.push_str(&format!(
                        "--- link {} -> {}\n",
                        f.path,
                        String::from_utf8_lossy(target)
                    ));
                }
                Entry::Other => out.push_str(&format!("--- other {}\n", f.path)),
            }
        }
        if !g.parse_stderr.is_empty() {
            out.push_str("--- bash-n stderr\n");
            write_block(&mut out, &g.parse_stderr);
        }
    }
    out
}

/// Where the lines of the current `---` block go.
enum Target {
    None,
    Stdout,
    Stderr,
    ParseStderr,
    File,
}

pub fn parse_file(text: &str) -> Result<Vec<Golden>, String> {
    let mut goldens: Vec<Golden> = Vec::new();
    let mut target = Target::None;
    for (n, line) in text.lines().enumerate() {
        let err = |m: &str| format!("line {}: {m}", n + 1);
        if let Some(rest) = line.strip_prefix("=== ") {
            let (index, name) = rest.split_once(": ").unwrap_or((rest, ""));
            goldens.push(Golden {
                index: index.parse().map_err(|_| err("bad case index"))?,
                name: name.to_string(),
                status: Status::Exit(0),
                stdout: Vec::new(),
                stderr: Vec::new(),
                files: Vec::new(),
                parse_status: 0,
                parse_stderr: Vec::new(),
            });
            target = Target::None;
            continue;
        }
        if line.is_empty() || (goldens.is_empty() && line.starts_with('#')) {
            continue;
        }
        let g = goldens
            .last_mut()
            .ok_or_else(|| err("content before a case"))?;
        if let Some(v) = line.strip_prefix("status: ") {
            g.status = Status::parse(v).ok_or_else(|| err("bad status"))?;
        } else if let Some(v) = line.strip_prefix("bash-n: ") {
            g.parse_status = v.parse().map_err(|_| err("bad bash-n status"))?;
        } else if let Some(block) = line.strip_prefix("--- ") {
            target = match block {
                "stdout" => Target::Stdout,
                "stderr" => Target::Stderr,
                "bash-n stderr" => Target::ParseStderr,
                _ => {
                    let (kind, rest) = block.split_once(' ').ok_or_else(|| err("bad block"))?;
                    let (path, entry) = match kind {
                        "file" => (rest.to_string(), Entry::File(Vec::new())),
                        "dir" => (rest.to_string(), Entry::Dir),
                        "other" => (rest.to_string(), Entry::Other),
                        "link" => {
                            let (p, t) = rest.split_once(" -> ").ok_or_else(|| err("bad link"))?;
                            (p.to_string(), Entry::Link(t.as_bytes().to_vec()))
                        }
                        "bigfile" => {
                            let mut parts = rest.rsplitn(3, ' ');
                            let hash = parts.next().and_then(|h| u64::from_str_radix(h, 16).ok());
                            let size = parts.next().and_then(|s| s.parse().ok());
                            let path = parts.next();
                            match (path, size, hash) {
                                (Some(p), Some(size), Some(hash)) => {
                                    (p.to_string(), Entry::BigFile { size, hash })
                                }
                                _ => return Err(err("bad bigfile")),
                            }
                        }
                        _ => return Err(err("unknown block")),
                    };
                    let is_file = matches!(entry, Entry::File(_));
                    g.files.push(FileEntry { path, entry });
                    if is_file { Target::File } else { Target::None }
                }
            };
        } else {
            let buf: &mut Vec<u8> = match target {
                Target::Stdout => &mut g.stdout,
                Target::Stderr => &mut g.stderr,
                Target::ParseStderr => &mut g.parse_stderr,
                Target::File => match g.files.last_mut() {
                    Some(FileEntry {
                        entry: Entry::File(data),
                        ..
                    }) => data,
                    _ => return Err(err("file content outside a file")),
                },
                Target::None => return Err(err("content outside a block")),
            };
            if line == NO_EOL {
                if buf.pop() != Some(b'\n') {
                    return Err(err("misplaced no-newline marker"));
                }
            } else if let Some(text) = line.strip_prefix('|') {
                buf.extend_from_slice(text.as_bytes());
                buf.push(b'\n');
            } else if let Some(text) = line.strip_prefix('\\') {
                buf.extend(unescape(text).map_err(|m| err(&m))?);
                buf.push(b'\n');
            } else {
                return Err(err("unexpected line"));
            }
        }
    }
    Ok(goldens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Golden {
        Golden {
            index: 3,
            name: "a case".into(),
            status: Status::Signal(9),
            stdout: b"one\n\ntwo\x1b[0m\r\n\xff\xfe end".to_vec(),
            stderr: b"bash: line 1: x\n".to_vec(),
            files: vec![
                FileEntry {
                    path: "out.txt".into(),
                    entry: Entry::File(b"x\\y\n|".to_vec()),
                },
                FileEntry {
                    path: "empty".into(),
                    entry: Entry::File(Vec::new()),
                },
                FileEntry {
                    path: "d".into(),
                    entry: Entry::Dir,
                },
                FileEntry {
                    path: "l".into(),
                    entry: Entry::Link(b"../t".to_vec()),
                },
                FileEntry {
                    path: "a b".into(),
                    entry: Entry::BigFile {
                        size: 100000,
                        hash: 0xdeadbeef,
                    },
                },
            ],
            parse_status: 2,
            parse_stderr: b"bash: line 2: syntax error\n".to_vec(),
        }
    }

    #[test]
    fn round_trip() {
        let goldens = vec![
            sample(),
            Golden {
                index: 4,
                name: String::new(),
                status: Status::Timeout,
                stdout: b"\n".to_vec(),
                stderr: Vec::new(),
                files: Vec::new(),
                parse_status: 0,
                parse_stderr: Vec::new(),
            },
        ];
        let text = write_file("header line", &goldens);
        assert!(text.starts_with("# header line\n"));
        assert_eq!(parse_file(&text).unwrap(), goldens);
    }

    #[test]
    fn readable_lines() {
        let mut out = String::new();
        write_block(&mut out, b"plain\ttab\nesc\x1b\n");
        assert_eq!(out, "|plain\ttab\n\\esc\\x1b\n");
        let mut out = String::new();
        write_block(&mut out, b"no newline");
        assert_eq!(out, "|no newline\n~ no newline at end\n");
    }
}
