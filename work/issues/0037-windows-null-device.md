# 0037: Investigate Windows NUL artifacts and guard null-device paths

status: in-progress
agent: coordinator
branch: issue/0037-windows-null-device
depends-on: -
touches: crates/psh-os/src/windows/path.rs, crates/psh/tests/cli.rs, docs/guide/windows.md, docs/decisions.md, ARCH.md
guide: docs/guide/windows.md
decisions: D1370–D1379

## Goal

Investigate the reserved-name artifacts reported in
https://github.com/earendil-works/pi/issues/7547 and ensure psh does not create them
when a script intends to use the Windows null device.

## Acceptance criteria

- [ ] Trace redirections, utility file opens and external standard handles.
- [ ] Add native Windows regressions for `/dev/null` and Windows null-device names,
      including long working-directory paths; assert no directory entries are created.
- [ ] Fix any reproduced null-device path gap without rewriting external argv.
- [ ] Document the findings and the boundary for explicitly invoked other shells.

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

## Follow-ups

- The probe also found `cmd.exe /d /c "echo discarded"` fails with a syntax error
  when the whole command is one quoted argument. That is unrelated to null-device
  opens and is not changed here.

## Result

Pending.
