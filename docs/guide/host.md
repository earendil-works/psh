# The host interface

psh's engine opens no file, starts no program and reads no clock itself. Everything that
touches the outside world goes through one Rust trait, `Host`. This page explains what
that trait asks for, why every operation may finish later, how resources are released,
which methods a small host can leave out, and what each of psh's hosts implements.

> **In short.** `Host` has nine required methods (open, stat, access, read a directory,
> canonicalize, resolve a command, spawn, sleep and `$$`) and two dozen optional ones
> whose defaults answer "unsupported" or a neutral value. Operations return futures,
> files and programs are polled, and the engine only waits through wakers, so a host may
> answer at once, from another thread or from a JavaScript event loop. Every resource a
> host hands out is released when it is dropped, which is how a cancelled run leaks
> nothing.

## Why a trait of futures

The engine runs every pipeline stage, subshell and background job on one thread
([The engine](engine.md)). If it blocked in a `read` on a pipe whose writer is another
task of the same shell, it would deadlock. So nothing in the trait may block the engine
thread on something the engine has to do first:

- File system operations return a `HostFuture<T>`, a boxed `std` future. A host with a
  local disk answers with a ready future; one with a remote or virtual file system
  answers later and wakes the engine's waker.
- An open file is a `HostFile` with `poll_read` and `poll_write`.
- `spawn` itself is synchronous and quick; the started program is an `ExtProc` whose end
  is polled with `poll_wait`.

The engine copes with an answer that is not there yet at every call site. The test
host proves it: in its latency mode every operation is pending once before it
completes, and the engine's and the utilities' tests run each script with and without
it and require the same output.

> **Trade-off.** One trait with defaults over a family of small traits or capability
> objects. A host is one value to pass, and an operation added later gets a default, so
> hosts that predate it keep building. The cost: the trait is wide, and a host learns
> what it left out only when a script needs it (`mkdir: Operation not supported`).

Paths are bytes, absolute and `/`-separated, as the shell sees them; they need not be
UTF-8. On Windows a drive stays in front (`C:/x`), and the host turns the path into a
native one at its boundary. The engine makes every path absolute against the shell's
working directory before it calls the host.

Resources end with `Drop`. Dropping a `HostFile` closes it; dropping an `ExtProc` that
is still running abandons it: the host sends `TERM`, `KILL` after a grace period, and
reaps it in the background. The engine cancels a run by dropping its tasks, so every
file, pipe and program the run held is released without a cleanup path of its own.

## The required methods

| Method | What it does | The shell uses it for |
| --- | --- | --- |
| `open(path, OpenOptions)` | opens a file as a `HostFile` | redirections, `source`, utilities |
| `stat(path, follow)` | kind, size, mode, mtime, device, inode, owner | `test -f`, globbing, `cd` |
| `access(path, Access)` | may the user read, write or execute it | `test -r/-w/-x`, `command -v` |
| `read_dir(path)` | the names in a directory, without `.` and `..` | globbing, `ls`, `find` |
| `canonicalize(path)` | the physical path | `pwd -P`, `cd -P` |
| `resolve(name, PATH, cwd)` | finds a command name on `PATH` | running a program |
| `spawn(SpawnSpec)` | starts a program | every external command |
| `sleep(duration)` | a future that completes later | `sleep`, `timeout`, `exit_grace` |
| `pid()` | the shell's process id | `$$` |

`OpenOptions` holds `read`, `write`, `append`, `create`, `truncate`, `create_new` (for
`noclobber`) and `mode`: the exact permission bits of a file the open creates, with the
shell's `umask` already applied. A host applies them only to a file the open really
created, and never by changing the process's umask, which other threads and the
embedder share; the Unix host creates the file exclusively and then sets its mode.

## Open files and started programs

A `HostFile` reads and writes through `poll_read` and `poll_write`; `Ready(Ok(0))` is
end of file, and a write to a pipe without readers is `BrokenPipe`. Two optional methods
describe it: `is_terminal` (`test -t`) and `kind`, what `fstat` would say (a file, a
pipe, a device), which utilities such as `rg` use to decide whether standard input is
worth reading. A `HostFile` is also `Any`: when the engine passes one of a host's own
files to `spawn`, the host recognises it and hands the real descriptor to the program.

A `SpawnSpec` is everything a program starts with: the resolved `program`, `argv` with
the name as written first, the complete `env`, `cwd`, the descriptors, the signals that
start ignored (`sig_ignore`, from `trap '' SIG` and for `&` jobs), the `umask`, the
resource limits of `ulimit`, and `detach` for `nohup` and `setsid`. Each descriptor is a
`ChildFd`:

| `ChildFd` | The program gets |
| --- | --- |
| `File(file)` | one of the host's files, as it is |
| `Pipe { input }` | a new pipe; the engine gets the other end back in `Spawned::pipes` and pumps it |
| `Dup(n)` | the same as descriptor `n` of the list, so `2>&1` into a pipe is one pipe and keeps the order of writes |
| `Null` | the null device |

`spawn` returns `Spawned`: the `ExtProc` and the engine's pipe ends. An error names why
the program could not start (`NotFound`, `PermissionDenied`), or is the marker
`RunInShell`: the file is something the operating system cannot start but bash would
run as a script (on Unix an executable without `#!`; on Windows a `sh` or `bash`
script, or a file with neither an interpreter line nor an executable extension). The
engine then reads the file through the host and runs it in a fresh subshell.

An `ExtProc` has `poll_wait`, which returns `Exited(code)` or `Signaled(n)`; `signal`;
`release`, which lets it run on and reaps it in the background (`nohup`, background
jobs left at the end, as bash leaves them); `disown`, which also takes it out of the
run, so the host does not kill it with the run's leftovers; and `pid`, the real process
id for `$!`, where there is one. Signal numbers are Linux's on every platform; hosts
map them (on Windows, `TERM` and `INT` become a console break, `KILL` ends the job).

## Processes and pipes

These have defaults that work for a host without an operating system underneath:

| Method | What it does | Default |
| --- | --- | --- |
| `pipe()` | a new OS pipe, both ends files of the host | unsupported |
| `kill(pid, sig)` | `kill` with a numeric pid; negative is a group | unsupported |
| `path_entries(PATH)` | splits a `PATH`-like value into entries | split on `:` |
| `standard_path()` | the system's standard `PATH` (`command -p`) | `/usr/bin:/bin:/usr/sbin:/sbin` |
| `native_paths()` | how a host path becomes a path `std::fs` can open | none |
| `strips_crlf()` | whether lines from this host's programs end in `\r\n` | no |
| `rlimit(resource)` | the process's soft and hard limit (`ulimit`) | none, so `ulimit` rejects the option |

`pipe` lets two programs in a pipeline share a kernel pipe. A pipe between the shell's
own commands is a buffer in memory; when a program starts on one while it holds no
bytes, the engine asks the host for an OS pipe and switches to it, so no byte between
two programs passes through psh. Without `pipe`, the engine pumps bytes through the
`ChildFd::Pipe` ends instead.

`path_entries` exists because the engine's own lookups (`type`, `command -v`, `hash`,
`source`) walk `PATH` one entry at a time, and on Windows an entry like `C:\Windows`
contains a colon. The Windows host splits on `;`, and on `:` except after a drive
letter.

`native_paths` decides how utilities built on synchronous libraries (`rg`, among
others) run. With it, they run on a worker thread and use the file system directly; the
OS hosts and the std host offer it. Without it, as in WebAssembly, a sandbox or the test
host, they run inline over the host's own futures, so they see its virtual file system.

`strips_crlf` makes `read` drop a `\r` before the newline of a whole line, and command
substitution drop one before each trailing newline it removes. It is a property of the
host, not of the build, so an embedder decides it for its own host and tests cover both
rules on every platform.

> **Not built yet.** The Windows host keeps the default and answers no; only the test
> host can be switched to yes.

## File system changes

`stat_extra` returns the rest of `stat(2)` (access, change and birth times, links,
blocks, block size, device number) for `ls -l`, `stat` and `du`; its default derives
them from `stat` (times from mtime, one link, the size in blocks). `mkdir`,
`remove_file`, `remove_dir`, `rename`, `symlink`, `hard_link`, `read_link`, `set_mode`
and `set_times` serve the file utilities and default to `Unsupported`, which a utility
reports as `Operation not supported` and fails: nothing is emulated. `remove_file`
removes a symlink itself, never its target, and a host that cannot make a symlink may
make an equivalent (a junction on Windows), never a copy.

## Facts about the machine

| Method | Feeds | Default |
| --- | --- | --- |
| `info()` | `UID`, `EUID`, `GROUPS`, `PPID`, `HOSTNAME`, `OSTYPE`, `MACHTYPE`, `HOSTTYPE` | uid 1000, `localhost`, `OSTYPE` `unknown` |
| `umask()` | where the shell's `umask` starts | `022` |
| `now()` | `$EPOCHSECONDS`, `printf '%(…)T'`, `date` | the epoch |
| `utc_offset(secs)` | local time | UTC |
| `user_name(uid)`, `group_name(gid)` | `ls -l`, `stat %U` | none: numbers are shown |
| `home_dir(user)` | `~user` | none: the word stays as written |

The Unix host answers from the running system in bash's formats. On macOS:

```text
$ psh -c 'echo "$OSTYPE $MACHTYPE $HOSTTYPE"; umask; command -pv cat; echo ~root'
darwin25.6.0 aarch64-apple-darwin25.6.0 aarch64
0022
/bin/cat
/var/root
```

## What each host implements

All five implement the required methods, `stat_extra` and every file system change.
Beyond that:

| Optional method | Unix | Windows | std | test | JavaScript |
| --- | --- | --- | --- | --- | --- |
| `pipe`, `kill` | yes | yes | no | no | no |
| `path_entries` | default | `;`, and `:` not after a drive | default | switchable separator | default |
| `standard_path` | `confstr(_CS_PATH)` | the system directories | default | default | default |
| `native_paths` | yes | yes | yes | no | no |
| `strips_crlf` | no | no | no | switchable | no |
| `rlimit` | yes | no | no | fixed values | no |
| `info` | yes | yes | default | default | default |
| `umask` | the process's | `022` | the process's | default | default |
| `now` | yes | yes | yes | a fixed instant | `Date.now()` |
| `utc_offset` | yes | yes | UTC | UTC | from JavaScript |
| `user_name` | yes | the own user | no | `user` | from JavaScript |
| `group_name` | yes | no | no | `users` | from JavaScript |
| `home_dir` | yes | the own user | no | `root` | no |
| `ExtProc::pid` | yes | yes | no | yes | no |

The Unix and Windows hosts run real programs in sessions or job objects, so a cancel
reaches a whole tree ([Processes on Unix](processes.md), [psh on Windows](windows.md)).
The std host uses the standard library's process API: no process groups, and any signal
kills. The test host has an in-memory file system, fake commands written as futures,
virtual time and counters for leak checks. The JavaScript host forwards every operation
to `js/psh.mjs`, whose `MemHost` keeps files in memory and runs commands written in
JavaScript ([psh in the browser](wasm.md)).

## A host of your own

A host with no programs, no clock and a few read-only files needs only the nine
required methods. This one (imports left out) compiles against the engine and runs
utilities over its files:

```rust
struct MapHost(BTreeMap<Vec<u8>, Vec<u8>>);
struct MapFile(RefCell<(Vec<u8>, usize)>); // bytes, and how far they were read

impl HostFile for MapFile {
    fn poll_read(&self, _: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let (data, pos) = &mut *self.0.borrow_mut();
        let n = buf.len().min(data.len() - *pos);
        buf[..n].copy_from_slice(&data[*pos..*pos + n]);
        *pos += n;
        Poll::Ready(Ok(n))
    }
    fn poll_write(&self, _: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.0.borrow_mut().0.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }
}

fn ready<T: 'static>(v: T) -> HostFuture<T> {
    Box::pin(std::future::ready(v))
}

impl Host for MapHost {
    fn open(&self, path: &[u8], o: OpenOptions) -> HostFuture<io::Result<Rc<dyn HostFile>>> {
        ready(match self.0.get(path) {
            _ if o.write => Err(io::ErrorKind::ReadOnlyFilesystem.into()),
            Some(data) => Ok(Rc::new(MapFile(RefCell::new((data.clone(), 0))))),
            None => Err(io::ErrorKind::NotFound.into()),
        })
    }
    fn stat(&self, path: &[u8], _follow: bool) -> HostFuture<io::Result<Metadata>> {
        let (kind, size) = match self.0.get(path) {
            Some(d) => (FileKind::File, d.len() as u64),
            None if path == b"/" => (FileKind::Dir, 0),
            None => return ready(Err(io::ErrorKind::NotFound.into())),
        };
        let (mode, mtime_ns, dev, ino, uid, gid) = (0o555, 0, 1, 1, 1000, 1000);
        ready(Ok(Metadata { kind, size, mode, mtime_ns, dev, ino, uid, gid }))
    }
    fn access(&self, path: &[u8], mode: Access) -> HostFuture<io::Result<bool>> {
        ready(Ok(self.0.contains_key(path) && mode == Access::Read))
    }
    fn read_dir(&self, _path: &[u8]) -> HostFuture<io::Result<Vec<DirEntry>>> {
        let names = self.0.keys().map(|k| DirEntry { name: k[1..].to_vec() });
        ready(Ok(names.collect()))
    }
    fn canonicalize(&self, path: &[u8]) -> HostFuture<io::Result<Vec<u8>>> {
        ready(Ok(path.to_vec()))
    }
    fn resolve(&self, _name: &[u8], _path: &[u8], _cwd: &[u8]) -> HostFuture<Option<Vec<u8>>> {
        ready(None) // no programs
    }
    fn spawn(&self, _spec: SpawnSpec) -> io::Result<Spawned> {
        Err(io::ErrorKind::NotFound.into())
    }
    fn sleep(&self, _duration: Duration) -> HostFuture<()> {
        ready(()) // no clock: every sleep is over at once
    }
    fn pid(&self) -> u32 {
        1
    }
}
```

With `/notes` in the map, `psh_utils::policy(true)` as the utilities and one `MapFile`
as both stdout and stderr, `sort notes | head -2; echo *; git status; echo > x` gives:

```text
apple
banana
notes
psh: line 1: git: command not found
psh: line 1: x: Read-only file system
```

## Where to look

- [crates/psh-engine/src/host.rs](../../crates/psh-engine/src/host.rs): `Host`,
  `HostFile`, `ExtProc`, `SpawnSpec`, `ChildFd`, `RunInShell` and the defaults.
- [crates/psh-os/src/unix/mod.rs](../../crates/psh-os/src/unix/mod.rs) and
  [crates/psh-os/src/windows/mod.rs](../../crates/psh-os/src/windows/mod.rs): the OS
  hosts; [crates/psh-os/src/unix/spawn.rs](../../crates/psh-os/src/unix/spawn.rs) and
  [crates/psh-os/src/windows/spawn.rs](../../crates/psh-os/src/windows/spawn.rs): their
  programs.
- [crates/psh-engine/src/std_host.rs](../../crates/psh-engine/src/std_host.rs): the std
  host; [crates/psh-engine/src/testhost.rs](../../crates/psh-engine/src/testhost.rs):
  the test host and its latency mode.
- [crates/psh-wasm/src/host.rs](../../crates/psh-wasm/src/host.rs) and
  [js/psh.mjs](../../js/psh.mjs): the JavaScript host and `MemHost`.
- [crates/psh-engine/src/pipe.rs](../../crates/psh-engine/src/pipe.rs): in-memory pipes
  and their switch to `Host::pipe`.
