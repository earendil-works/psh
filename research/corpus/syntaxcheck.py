#!/usr/bin/env python3
"""Ground-truth syntax validity: run `bash -n` (bash 5) and `zsh -n` on every
unique command. Writes $PSH_CORPUS_DATA/syntax.json {cmd_hash: [bash_ok, zsh_ok]}.
`bash -n` only parses; nothing is executed."""
import hashlib
import json
import os
import subprocess
import multiprocessing as mp

DATA = os.environ.get("PSH_CORPUS_DATA", os.path.expanduser("~/.cache/psh-corpus"))
BASH = "/opt/homebrew/bin/bash" if os.path.exists("/opt/homebrew/bin/bash") else "bash"


def h(cmd):
    return hashlib.sha1(cmd.encode("utf-8", "replace")).hexdigest()


def check(cmd):
    def run(argv):
        try:
            p = subprocess.run(argv, input=cmd.encode("utf-8", "replace"), capture_output=True, timeout=10)
            return p.returncode == 0
        except Exception:
            return None
    return h(cmd), [run([BASH, "-n"]), run(["zsh", "-n"])]


def main():
    pool = mp.get_context("spawn").Pool(16)  # spawn before loading data: cheap forks in workers
    cmds = {}
    for l in open(os.path.join(DATA, "commands.jsonl")):
        c = json.loads(l)["cmd"]
        cmds[h(c)] = c
    with pool:
        res = dict(pool.imap_unordered(check, cmds.values(), chunksize=256))
    json.dump(res, open(os.path.join(DATA, "syntax.json"), "w"))
    print(len(res), "unique; bash -n fail", sum(1 for v in res.values() if v[0] is False),
          "zsh -n fail", sum(1 for v in res.values() if v[1] is False))


if __name__ == "__main__":
    main()
