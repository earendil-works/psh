//! Built-in stand-ins for the Python helpers in Oils' `spec/bin` (Apache-2.0, Copyright
//! the Oils authors), so that neither the recording nor a test run needs Python, and
//! both see byte-identical helper behavior on every platform. The run's helper
//! directory links each name to this binary; `main` dispatches on `argv[0]`.
//!
//! The originals are Python 2 programs; their output follows Python 2's (`repr` of
//! byte strings, block-buffered stdout that is flushed after stderr is written).

use std::io::{Read, Write};

pub const NAMES: [&str; 5] = [
    "argv.py",
    "printenv.py",
    "read_from_fd.py",
    "show_fd_table.py",
    "stdout_stderr.py",
];

/// Runs the helper named by `argv0`'s file name, if it is one; returns its exit status.
pub fn dispatch(argv0: &std::ffi::OsStr, args: &[Vec<u8>]) -> Option<i32> {
    let name = std::path::Path::new(argv0).file_name()?.to_str()?;
    let name = name.strip_suffix(".exe").unwrap_or(name);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let status = match name {
        "argv.py" => {
            out = argv_repr(args);
            0
        }
        "printenv.py" => {
            for name in args {
                match std::env::var_os(os_str(name)) {
                    Some(v) => out.extend_from_slice(&bytes_of(&v)),
                    None => out.extend_from_slice(b"None"),
                }
                out.push(b'\n');
            }
            0
        }
        "read_from_fd.py" => read_from_fd(args, &mut out, &mut err),
        "show_fd_table.py" => {
            show_fd_table(&mut out);
            0
        }
        "stdout_stderr.py" => {
            let arg = |i: usize, d: &[u8]| args.get(i).cloned().unwrap_or_else(|| d.to_vec());
            out = arg(0, b"STDOUT");
            out.push(b'\n');
            err = arg(1, b"STDERR");
            err.push(b'\n');
            match args.get(2) {
                Some(s) => std::str::from_utf8(s)
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(1),
                None => 0,
            }
        }
        _ => return None,
    };
    // Python 2 writes stderr unbuffered and stdout at exit.
    let _ = std::io::stderr().write_all(&err);
    let _ = std::io::stdout().write_all(&out);
    let _ = std::io::stdout().flush();
    Some(status)
}

#[cfg(unix)]
fn bytes_of(s: &std::ffi::OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    s.as_bytes().to_vec()
}

#[cfg(not(unix))]
fn bytes_of(s: &std::ffi::OsStr) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}

#[cfg(unix)]
fn os_str(b: &[u8]) -> &std::ffi::OsStr {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::OsStr::from_bytes(b)
}

#[cfg(not(unix))]
fn os_str(b: &[u8]) -> std::ffi::OsString {
    String::from_utf8_lossy(b).into_owned().into()
}

/// The process arguments as bytes.
pub fn args_bytes() -> Vec<Vec<u8>> {
    std::env::args_os().skip(1).map(|a| bytes_of(&a)).collect()
}

/// Python 2's `repr` of a byte string.
fn py2_repr(s: &[u8]) -> String {
    let quote = if s.contains(&b'\'') && !s.contains(&b'"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for &b in s {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'\t' => out.push_str("\\t"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            _ if b as char == quote => {
                out.push('\\');
                out.push(quote);
            }
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push(quote);
    out
}

/// `print(sys.argv[1:])` in Python 2.
fn argv_repr(args: &[Vec<u8>]) -> Vec<u8> {
    let items: Vec<String> = args.iter().map(|a| py2_repr(a)).collect();
    format!("[{}]\n", items.join(", ")).into_bytes()
}

fn errno_text(e: &std::io::Error) -> String {
    let n = e.raw_os_error().unwrap_or(0);
    let text = e.to_string();
    let text = text.split(" (os error").next().unwrap_or(&text).to_string();
    format!("[Errno {n}] {text}")
}

/// Reads up to 1024 bytes (one read) from each fd named on the command line. The fd is
/// reached through `/dev/fd/N`: std offers no safe way to adopt a raw fd.
fn read_from_fd(args: &[Vec<u8>], out: &mut Vec<u8>, err: &mut Vec<u8>) -> i32 {
    for arg in args {
        let fd: i32 = match std::str::from_utf8(arg).ok().and_then(|s| s.parse().ok()) {
            Some(fd) => fd,
            None => {
                err.extend_from_slice(b"ValueError: invalid fd\n");
                return 1;
            }
        };
        let result = std::fs::File::open(format!("/dev/fd/{fd}")).and_then(|mut f| {
            let mut buf = vec![0u8; 1024];
            let n = f.read(&mut buf)?;
            buf.truncate(n);
            Ok(buf)
        });
        match result {
            Ok(data) => {
                out.extend_from_slice(format!("{fd}: ").as_bytes());
                out.extend_from_slice(&data);
            }
            Err(e) => {
                // A closed fd: Python's os.read reports EBADF; opening /dev/fd/N may
                // report ENOENT instead.
                let text = if e.kind() == std::io::ErrorKind::NotFound {
                    "[Errno 9] Bad file descriptor".to_string()
                } else {
                    errno_text(&e)
                };
                err.extend_from_slice(
                    format!("FATAL: Error reading from fd {fd}: {text}\n").as_bytes(),
                );
                return 1;
            }
        }
    }
    0
}

/// Lists `/proc/self/fd` with each link's target (Linux only, like the original).
fn show_fd_table(out: &mut Vec<u8>) {
    let Ok(entries) = std::fs::read_dir("/proc/self/fd") else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        match std::fs::read_link(e.path()) {
            Ok(target) => {
                out.extend_from_slice(format!("{name} {}\n", target.display()).as_bytes())
            }
            Err(err) => out.extend_from_slice(format!("{name} {}\n", errno_text(&err)).as_bytes()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python2_repr() {
        assert_eq!(argv_repr(&[]), b"[]\n");
        assert_eq!(
            argv_repr(&[b"a b".to_vec(), b"".to_vec(), b"it's".to_vec()]),
            b"['a b', '', \"it's\"]\n"
        );
        assert_eq!(py2_repr(b"both ' and \""), "'both \\' and \"'");
        assert_eq!(py2_repr(b"\xce\xbc\t\x01\\"), "'\\xce\\xbc\\t\\x01\\\\'");
    }
}
