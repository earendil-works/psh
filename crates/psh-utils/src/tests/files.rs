//! The file utilities (0015) on the test host: an in-memory fs with symlinks, virtual
//! time (now = 2023-11-14T22:13:20Z, UTC).

use std::rc::Rc;

use psh_engine::testhost::TestHost;
use psh_engine::{Script, Shell, Step};

struct Run {
    status: i32,
    stdout: String,
    stderr: String,
    host: Rc<TestHost>,
}

fn run_with(src: &str, latency: bool, setup: &dyn Fn(&TestHost)) -> Run {
    let host = Rc::new(TestHost::new());
    host.set_latency(latency);
    host.fs.write_file("/tmp/f", b"one\ntwo\n");
    host.fs.mkdir_all("/tmp/dir/sub");
    host.fs.write_file("/tmp/dir/sub/x", b"x\n");
    host.fs.symlink("dir/sub", "/tmp/ln");
    host.fs.symlink("nowhere", "/tmp/dangling");
    setup(&host);
    let mut cfg = host.config();
    cfg.utilities = crate::policy(false);
    let mut shell = Shell::new(cfg);
    shell.start(Script::command(src));
    let status = loop {
        match shell.step(13) {
            Step::Done(s) => break s,
            Step::Yield => {}
            other => panic!("{other:?} running {src:?}"),
        }
    };
    assert_eq!(shell.task_count(), 0, "tasks left after {src:?}");
    Run {
        status,
        stdout: String::from_utf8_lossy(&host.stdout()).into_owned(),
        stderr: String::from_utf8_lossy(&host.stderr()).into_owned(),
        host,
    }
}

/// Runs with and without latency; both must agree.
fn run_setup(src: &str, setup: &dyn Fn(&TestHost)) -> Run {
    let a = run_with(src, false, setup);
    let b = run_with(src, true, setup);
    assert_eq!(
        (a.status, &a.stdout, &a.stderr),
        (b.status, &b.stdout, &b.stderr),
        "latency changed {src:?}"
    );
    a
}

fn run(src: &str) -> Run {
    run_setup(src, &|_| {})
}

#[track_caller]
fn check(src: &str, stdout: &str) {
    let r = run(src);
    assert_eq!(r.stdout, stdout, "stdout of {src:?} (stderr: {})", r.stderr);
}

#[track_caller]
fn check_all(src: &str, status: i32, stdout: &str, stderr: &str) {
    let r = run(src);
    assert_eq!(
        (r.status, r.stdout.as_str(), r.stderr.as_str()),
        (status, stdout, stderr),
        "{src:?}"
    );
}

#[test]
fn basename_dirname() {
    check("basename /a/b/c.txt .txt", "c\n");
    check("basename -s .c a.c b/d.c", "a\nd\n");
    check("basename -a x/ y//", "x\ny\n");
    check("basename .c .c; basename //", ".c\n/\n");
    check_all(
        "basename a b c",
        1,
        "",
        "basename: extra operand 'c'\nTry 'basename --help' for more information.\n",
    );
    check("dirname /a//b// a x/y", "/a\n.\nx\n");
    check("dirname -z / x/y | wc -c", "4\n");
}

#[test]
fn readlink_realpath() {
    check("readlink ln dangling", "dir/sub\nnowhere\n");
    check_all("readlink f; echo $?", 0, "1\n", "");
    check_all("readlink -v f", 1, "", "readlink: f: Invalid argument\n");
    check(
        "readlink -f ln/x ln/../f dangling",
        "/tmp/dir/sub/x\n/tmp/dir/f\n/tmp/nowhere\n",
    );
    check_all("readlink -e dangling; echo $?", 0, "1\n", "");
    check("readlink -m a/../../b", "/b\n");
    check("cd dir; readlink -f ../ln", "/tmp/dir/sub\n");
    check(
        "realpath ln f missing",
        "/tmp/dir/sub\n/tmp/f\n/tmp/missing\n",
    );
    check_all(
        "realpath missing/x f/ f/x",
        1,
        "",
        "realpath: missing/x: No such file or directory\nrealpath: f/: Not a directory\n\
         realpath: f/x: Not a directory\n",
    );
    check("realpath -m f/x/../y", "/tmp/f/y\n");
    check("realpath -s ln/..", "/tmp\n");
    check("realpath -L ln/..", "/tmp\n");
    check("realpath ln/..", "/tmp/dir\n");
    check("realpath --relative-to=dir/sub f ln", "../../f\n.\n");
    check("realpath --relative-base=/tmp f /bin", "f\n/bin\n");
    check("cd dir; realpath", "/tmp/dir\n");
    check("realpath -q nope/x; echo $?", "1\n");
}

#[test]
fn mkdir_rmdir() {
    let r = run("mkdir a; mkdir -p b/c/d; mkdir -m 700 m; mkdir -pv p/q");
    assert_eq!(r.status, 0, "{}", r.stderr);
    assert_eq!(
        r.stdout,
        "mkdir: created directory 'p'\nmkdir: created directory 'p/q'\n"
    );
    check_all(
        "mkdir f dir x/y",
        1,
        "",
        "mkdir: cannot create directory 'f': File exists\n\
         mkdir: cannot create directory 'dir': File exists\n\
         mkdir: cannot create directory 'x/y': No such file or directory\n",
    );
    check_all(
        "mkdir -p f/x",
        1,
        "",
        "mkdir: cannot create directory 'f': Not a directory\n",
    );
    check_all(
        "mkdir -p dir/sub ./dir//new/../n2 && echo ok",
        0,
        "ok\n",
        "",
    );
    check_all("mkdir -m bad z", 1, "", "mkdir: invalid mode 'bad'\n");
    check_all(
        "rmdir dir",
        1,
        "",
        "rmdir: failed to remove 'dir': Directory not empty\n",
    );
    check_all("rmdir --ignore-fail-on-non-empty dir", 0, "", "");
    let r = run("mkdir -p a/b/c; rmdir -pv a/b/c");
    assert_eq!(
        (r.status, r.stdout.as_str()),
        (
            0,
            "rmdir: removing directory, 'a/b/c'\nrmdir: removing directory, 'a/b'\n\
             rmdir: removing directory, 'a'\n"
        )
    );
    assert!(!r.host.fs.exists("/tmp/a"));
    check_all(
        "rmdir f",
        1,
        "",
        "rmdir: failed to remove 'f': Not a directory\n",
    );
}

#[test]
fn touch() {
    let r = run("touch new f && echo ok");
    assert_eq!(r.stdout, "ok\n");
    assert_eq!(r.host.fs.read_file("/tmp/new").as_deref(), Some(&b""[..]));
    check_all("touch -c none; echo $?", 0, "0\n", "");
    check_all(
        "touch x/y",
        1,
        "",
        "touch: cannot touch 'x/y': No such file or directory\n",
    );
    check_all(
        "touch -d nonsense f",
        1,
        "",
        "touch: invalid date format 'nonsense'\n",
    );
    check_all("touch -t 99 f", 1, "", "touch: invalid date format '99'\n");
}

/// What the dates of `touch -d`/`-t` mean, read back through `ls`-free means: the
/// mtime via the test host.
#[test]
fn touch_dates() {
    use psh_engine::Host;
    let cases: &[(&str, i64)] = &[
        ("-d '2024-01-02 03:04:05'", 1_704_164_645),
        ("-d 2024-01-02T03:04:05Z", 1_704_164_645),
        ("-d '2024-01-02 03:04:05 +0100'", 1_704_164_645 - 3600),
        ("-d @1700000000", 1_700_000_000),
        ("-d '2 days ago'", 1_700_000_000 - 2 * 86400),
        ("-d '-1 hour'", 1_700_000_000 - 3600),
        ("-d '+1 week'", 1_700_000_000 + 7 * 86400),
        ("-d yesterday", 1_700_000_000 - 86400),
        ("-d 'Jan 2 2024'", 1_704_153_600),
        ("-d '2 Jan 2024 10:00'", 1_704_153_600 + 36000),
        ("-d '1 month ago'", 1_700_000_000 - 31 * 86400),
        ("-d '2024-01-02 3pm'", 1_704_153_600 + 15 * 3600),
        ("-t 202401020304.05", 1_704_164_645),
        ("-t 2401020304", 1_704_164_640),
        ("-r f", 0),
    ];
    for (args, want) in cases {
        let r = run(&format!("touch {args} t"));
        assert_eq!(r.status, 0, "touch {args}: {}", r.stderr);
        let fut = r.host.stat(b"/tmp/t", true);
        let m = futures_ready(fut).unwrap();
        if *want != 0 {
            assert_eq!(m.mtime_ns / 1_000_000_000, *want, "touch {args}");
        }
    }
}

/// Polls a host future that is ready at once (the test host without latency).
fn futures_ready<T>(mut f: psh_engine::host::HostFuture<T>) -> T {
    use std::task::{Context, Poll, Waker};
    let mut cx = Context::from_waker(Waker::noop());
    match f.as_mut().poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("pending"),
    }
}

/// A host without the file system changes (D1150): the utilities fail clearly.
#[test]
fn unsupported_host() {
    let unsupported = |h: &TestHost| h.fs.set_unsupported(true);
    let r = run_setup("mkdir a", &unsupported);
    assert_eq!(
        (r.status, r.stderr.as_str()),
        (
            1,
            "mkdir: cannot create directory 'a': Operation not supported\n"
        )
    );
    let r = run_setup("touch f", &unsupported);
    assert_eq!(
        (r.status, r.stderr.as_str()),
        (1, "touch: setting times of 'f': Operation not supported\n")
    );
    let r = run_setup("rmdir dir/sub", &unsupported);
    assert_eq!(r.status, 1);
    let r = run_setup("readlink -v ln", &unsupported);
    assert_eq!(
        (r.status, r.stderr.as_str()),
        (1, "readlink: ln: Operation not supported\n")
    );
}
