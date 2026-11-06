#!/usr/bin/env python3
"""Compares psh-classify with research/corpus/analyze.py on the private corpus.

Local only: reads $PSH_CORPUS_DATA (default ~/.cache/psh-corpus)/commands.jsonl and the
features.jsonl that analyze.py writes, and prints aggregate numbers, never commands.
The command names it lists are the names of unknown commands among those analyze.py
calls read-only; check them before copying anything into a committed file.

  python3 tests/classify/corpus_accuracy.py
"""
import collections
import json
import os
import subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DATA = os.environ.get("PSH_CORPUS_DATA", os.path.expanduser("~/.cache/psh-corpus"))
READ = {"read-only", "cd", "echo", "shell-builtin"}
# analyze.py's primary order, mapped onto psh-classify's categories.
ORDER = [("pkg-install", "install"), ("vcs-mutate", "vcs"), ("file-mutate", "mutate"),
         ("build-test", "build"), ("interp-inline", "unknown"), ("script-run", "unknown"),
         ("network", "network"), ("process", "process"), ("long-running", "long-running")]


def analyze_primary(cats):
    for c, ours in ORDER:
        if c in cats:
            return ours
    return "read" if cats and set(cats) <= READ else "unknown"


def main():
    cmds = [json.loads(l)["cmd"] for l in open(os.path.join(DATA, "commands.jsonl"))]
    feats = [json.loads(l) for l in open(os.path.join(DATA, "features.jsonl"))]
    assert len(cmds) == len(feats)
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "psh-classify", "--example", "classify_lines"],
                   cwd=ROOT, check=True, timeout=1200)
    exe = os.path.join(ROOT, "target", "release", "examples", "classify_lines")
    data = b"\0".join(c.encode("utf-8", "replace").replace(b"\0", b" ") for c in cmds)
    out = subprocess.run([exe], input=data, capture_output=True, check=True, timeout=1200).stdout.decode()
    rows = out.split("\n")[:len(cmds)]
    n = len(cmds)
    ours = theirs = both = errors = 0
    primary, only_ours, only_theirs, names, conf = (collections.Counter() for _ in range(5))
    for f, r in zip(feats, rows):
        p, _cats, ro, err, reason, unknown = r.split("\t")
        ro, tro = ro == "true", bool(f["cats"]) and set(f["cats"]) <= READ
        errors += err == "true"
        ours += ro
        theirs += tro
        both += ro and tro
        primary[p] += 1
        conf[(analyze_primary(f["cats"]), p)] += 1
        if ro and not tro:
            only_ours[",".join(sorted(set(f["cats"]) - READ))] += 1
        if tro and not ro:
            only_theirs[reason or p] += 1
            names.update(x for x in unknown.split(",") if x)
    pct = lambda x: f"{x / n:.1%}"
    print(f"commands {n}, parse errors {errors} ({pct(errors)})")
    print(f"read-only: psh-classify {ours} ({pct(ours)}), analyze.py {theirs} ({pct(theirs)}), both {both}")
    print("primary:", ", ".join(f"{k} {pct(v)}" for k, v in primary.most_common()))
    print(f"read-only for psh-classify only: {sum(only_ours.values())}; analyze.py's other labels:")
    for k, v in only_ours.most_common(12):
        print(f"  {v:6} {k}")
    print(f"read-only for analyze.py only: {sum(only_theirs.values())}; psh-classify's reasons:")
    for k, v in only_theirs.most_common(12):
        print(f"  {v:6} {k}")
    print("  unknown command names there (check before publishing):",
          ", ".join(f"{k} {v}" for k, v in names.most_common(12)))
    print("primary, analyze.py -> psh-classify:")
    for (a, b), v in conf.most_common(20):
        print(f"  {v:6} {a} -> {b}")


if __name__ == "__main__":
    main()
