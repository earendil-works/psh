# 0037: Investigate Windows NUL artifacts and guard null-device paths

status: done
agent: coordinator
branch: issue/0037-windows-null-device
depends-on: -
touches: crates/psh-os/src/windows/{path,mod}.rs, crates/psh/tests/cli.rs, docs/guide/windows.md, docs/decisions.md, ARCH.md
guide: docs/guide/windows.md
decisions: D1370–D1379

## Goal

Investigate the reserved-name artifacts reported in
https://github.com/earendil-works/pi/issues/7547 and ensure psh does not create them
when a script intends to use the Windows null device.

## Acceptance criteria

- [x] Trace redirections, utility file opens and external standard handles.
- [x] Add native Windows regressions for `/dev/null` and Windows null-device names,
      including long working-directory paths; assert no directory entries are created.
- [x] Fix any reproduced null-device path gap without rewriting external argv.
- [x] Document the findings and the boundary for explicitly invoked other shells.

## Notes

The upstream thread contains conflicting reports: one user reports `/dev/null`
creating `nul` under Git Bash, while another cannot reproduce it. It provides no
exact failing session. Do not assert that Git Bash always mishandles `/dev/null`.

## Log

- 2026-11-08: Traced engine redirections through `WindowsHost::open` and
  `path::native`; `/dev/null` maps to `NUL`. Native child standard handles are
  duplicated opened handles, not textual redirections or MSYS argv conversion.
  Threaded utilities use the same native-path conversion. `ChildFd::Null` and
  unspecified native standard handles open `NUL` directly.
- 2026-11-08: Probed the Windows VM's binary (0036 / 3a964cba; subsequent main edits
  are documentation only). `/dev/null` read/write/append/noclobber and bare
  `NUL`, `nul`, `./nul` leave no entries, including a cwd over 400 characters.
  `>NUL.txt` in that long cwd silently creates an NTFS reserved-name file.
  Probes are in primary `work/scratch/0037/probe.ps1`. No upstream source read.
- 2026-11-08: Opened a dedicated worktree for this user-directed investigation;
  existing Windows work is paused and its worktree is untouched.
- 2026-11-08: Added CLI regressions for input, output, append, stderr, combined
  output, read/write descriptors, noclobber, `tee` file arguments and inherited
  native stdout. Each checks actual directory entries in short and >400-character
  directories. Added explicit verbatim artifact cleanup coverage and path unit tests.
- 2026-11-08: Canonicalize null-device aliases in `path::native` before std's
  long-path conversion, preserving explicit verbatim paths for cleanup. Extended the
  minimal touches to `windows/mod.rs` so `stat` also recognizes these character
  devices (`test -c`, noclobber). No new dependencies or engine changes.
- 2026-11-08: The new tests fail before the fix; the full native arm64 Windows
  `test` leg passes after it. Native-child coverage uses a short child cwd because
  CreateProcess refuses the long cwd; the redirection is opened from the long cwd
  before the subshell changes directory. Updated ARCH.md, the Windows guide and
  decision D1370; no live examples or ratchets affected.

## Follow-ups

- The probe also found `cmd.exe /d /c "echo discarded"` fails with a syntax error
  when the whole command is one quoted argument. That is unrelated to null-device
  opens and is not changed here.
- Native process spawning from a >MAX_PATH cwd fails as NotADirectory, even when
  the executable and redirection are valid. Win32 limits lpCurrentDirectory; the
  null-device regression changes only the child's cwd to avoid that separate limit.

## Result

Confirmed that psh's `/dev/null` redirections do not create the artifacts reported
upstream. Fixed a reproduced related hole: Windows null-device aliases with an
extension (such as `NUL.txt`) could become literal reserved-name files, particularly
in long paths. Canonical null-device path conversion and character-device stat now
cover aliases; explicit verbatim paths remain available to remove existing artifacts.

Three native Windows CLI tests and a path unit test cover the boundary. The arm64
Windows workspace tests pass. No arbitrary external argv or embedded shell scripts
are rewritten; those programs retain responsibility for their own path semantics.
