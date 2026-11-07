//! onetrueawk's T.* scripts (tests/vendor-awk/onetrueawk-T, README.md there) run by the
//! psh binary with psh's awk (`PSH_UTILS_FORCE=awk`) and the system's other tools, so
//! Unix only. A script prints `BAD: ...` for each check that fails; psh may fail only
//! the checks gawk fails too (expected.T.gnu.txt: psh follows gawk where the two awks
//! differ, D1193), the rest is the ratchet expected-failures.T.txt (fails on new lines
//! and on stale ones; `PSH_UPDATE_RATCHET=1` rewrites it).

#[cfg(unix)]
#[test]
fn onetrueawk_t_scripts() {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/vendor-awk");
    let src = root.join("onetrueawk-T");
    let lines = |p: &Path| -> Vec<Vec<u8>> {
        std::fs::read(p)
            .unwrap_or_default()
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty() && !l.starts_with(b"#"))
            .map(<[u8]>::to_vec)
            .collect()
    };
    let gnu: BTreeSet<Vec<u8>> = lines(&root.join("expected.T.gnu.txt"))
        .into_iter()
        .collect();
    let mut files: Vec<String> = std::fs::read_dir(&src)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    let scripts: Vec<&String> = files.iter().filter(|f| f.starts_with("T.")).collect();
    let work = std::env::temp_dir().join(format!("psh-onetrueawk-{}", std::process::id()));
    let scale = std::env::var("PSH_TEST_TIME_SCALE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| *s >= 1.0)
        .unwrap_or(1.0);

    let mut failing: BTreeMap<Vec<u8>, ()> = BTreeMap::new();
    for t in &scripts {
        let dir = work.join(t.as_str());
        std::fs::create_dir_all(&dir).unwrap();
        for f in &files {
            std::fs::copy(src.join(f), dir.join(f)).unwrap();
        }
        // The suite's ./echo (its echo.c): the arguments, no escapes, -n.
        let echo = dir.join("echo");
        std::fs::write(
            &echo,
            "#!/bin/sh\nn=1\nif [ \"$1\" = -n ]; then n=0; shift; fi\nprintf '%s' \"$*\"\n[ $n = 1 ] && printf '\\n'\nexit 0\n",
        )
        .unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&echo, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let out_path = dir.join(".out");
        let out = std::fs::File::create(&out_path).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_psh"))
            .arg(t.as_str())
            .current_dir(&dir)
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin")
            .env("HOME", &dir)
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .env("awk", "awk")
            .env("PSH_UTILS_FORCE", "awk")
            .stdin(Stdio::null())
            .stdout(out.try_clone().unwrap())
            .stderr(out)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs_f64(120.0 * scale);
        let timed_out = loop {
            if child.try_wait().unwrap().is_some() {
                break false;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                break true;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let output = std::fs::read(&out_path).unwrap();
        let mut line = |l: &[u8]| {
            let mut id = t.as_bytes().to_vec();
            id.push(b'\t');
            id.extend_from_slice(l);
            if !gnu.contains(&id) {
                failing.insert(id, ());
            }
        };
        if timed_out {
            line(b"(timeout)");
        }
        if output.windows(8).any(|w| w == b"panicked") {
            line(b"(psh panicked)");
        }
        for l in output.split(|&b| b == b'\n') {
            if l.windows(3).any(|w| w == b"BAD") {
                line(l);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&work);

    let ratchet_path = root.join("expected-failures.T.txt");
    let ratchet: BTreeSet<Vec<u8>> = lines(&ratchet_path).into_iter().collect();
    let regressions: Vec<String> = failing
        .keys()
        .filter(|l| !ratchet.contains(*l))
        .map(|l| String::from_utf8_lossy(l).into_owned())
        .collect();
    let stale: Vec<String> = ratchet
        .iter()
        .filter(|l| !failing.contains_key(*l))
        .map(|l| String::from_utf8_lossy(l).into_owned())
        .collect();
    eprintln!(
        "onetrueawk T.*: {} scripts, {} checks fail beyond gawk's ({} expected)",
        scripts.len(),
        failing.len(),
        ratchet.len()
    );
    if std::env::var_os("PSH_UPDATE_RATCHET").is_some() {
        let mut out = b"# Checks of onetrueawk's T.* scripts psh fails and gawk passes \
(tests/vendor-awk/README.md):\n# `script<TAB>BAD line`. Update with PSH_UPDATE_RATCHET=1 \
cargo test -p psh --test onetrueawk\n"
            .to_vec();
        for l in failing.keys() {
            out.extend_from_slice(l);
            out.push(b'\n');
        }
        std::fs::write(&ratchet_path, out).unwrap();
        return;
    }
    assert!(
        regressions.is_empty() && stale.is_empty(),
        "onetrueawk T.*: new failures {regressions:#?}; now passing (PSH_UPDATE_RATCHET=1): {stale:#?}"
    );
}
