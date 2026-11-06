# Oils spec-test comparison harness (run 2026-11-06)

Reproduces the conformance numbers in `../landscape.md`.

- `Dockerfile`: Debian bookworm + rust + Go 1.25 (auto-upgrades to 1.26) + bun + bash/dash/mksh/zsh/busybox.
- `build-linux.sh`: builds brush, yash-rs, gosh (mvdan/sh) and `dts-cli` (a ~20 line stdin
  driver for `deno_task_shell`) from checkouts in `/work/<repo>`.
- `shells/`: wrappers. `bunsh` writes stdin to a temp `.sh` file and runs `bun file.sh`
  (Bun Shell's `.sh` loader); `ash` is `busybox ash`.
- `sh_spec-py3.patch`: minimal Python 3 port of Oils' `test/sh_spec.py` (upstream is Python 2).
  Apply to an oils checkout (commit d114978) as `test/sh_spec3.py`; also change
  `spec/bin/*.py` shebangs to python3.
- `SUITE-compat.txt`: the 134 OSH-suite files minus `strict-options`, i.e. Oils' own
  `SUITE-compat` selection (`sh_spec.py --print-table`).
- `run-spec.sh`: runs every file with `--timeout 10`, 8 in parallel.
- `results/`: raw per-file output (ANSI stripped). `agg.py`/`agg2.py` aggregate
  (they read `/tmp/ls/results`; adjust the path).

Run: `docker build -t shspec . && docker run --rm -v $PWD:/work shspec bash /work/build-linux.sh && docker run --rm -v $PWD:/work shspec bash /work/run-spec.sh`
(with the repos cloned next to it as `/work/brush`, `/work/sh`, `/work/yash-rs`,
`/work/deno_task_shell`, `/work/oils`).
