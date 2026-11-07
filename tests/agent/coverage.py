#!/usr/bin/env python3
"""How much of the shape corpus (tests/shapes/shapes.txt) the agent cases cover.

  python3 tests/agent/coverage.py [--missing N]

Every top-level command of every case (and every case as a whole) is turned into a
skeleton with scripts/shape-corpus's rules, so it can be compared with the corpus
skeletons. Three levels of matching, from strict to loose:

  exact   placeholders unified (w1, w2, ... are all `W`), everything else equal
  args    also: runs of plain arguments (words, paths, quoted words, numbers) are one
  syntax  also: every word is `W` (command names and options too); only the syntax
          (operators, quoting, redirections, here-docs, expansions, keywords) remains

The share is of the commands the committed shapes stand for (`COUNT` summed), and of
all commands of the corpus. Needs tree-sitter-bash like scripts/shape-corpus (it
re-runs itself under research/corpus/.venv). Reads no private data.
"""
import importlib.machinery
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
CORPUS_COMMANDS = 295401

_loader = importlib.machinery.SourceFileLoader("shape_corpus", os.path.join(ROOT, "scripts", "shape-corpus"))
_spec = importlib.util.spec_from_loader("shape_corpus", _loader)
sc = importlib.util.module_from_spec(_spec)
_loader.exec_module(sc)

KEYWORDS = set(sc.KEYWORDS)
ARG_TOKEN = re.compile(r"^[\"']?[~./]*(?:W|\d+)(?:[/.:@%+,=-]*(?:W|\d+))*[\"']?$")


def unify(sk):
    return re.sub(r"\b[wv]\d+\b", "W", sk)


def collapse_args(sk):
    out = []
    for line in sk.split("\n"):
        toks = line.split(" ")
        new = []
        for t in toks:
            core = t.rstrip(";")
            tail = t[len(core):]
            a = "A" if ARG_TOKEN.match(core) else core
            if a == "A" and new and new[-1] == "A":
                new[-1] += tail
                continue
            new.append(a + tail)
        out.append(" ".join(new))
    return "\n".join(out)


def syntax_only(sk):
    def word(m):
        w = m.group(0)
        return w if w in KEYWORDS else "W"
    s = re.sub(r"[A-Za-z_][A-Za-z0-9_.-]*", word, collapse_args(sk))
    s = re.sub(r"\b\d+\b", "W", s)
    # Runs of words collapse too: `W W W` is one.
    s = re.sub(r"(?:-{0,2}W)(?: -{0,2}W)+", "W", s)
    return s


LEVELS = [("exact", unify), ("args", lambda s: collapse_args(unify(s))), ("syntax", lambda s: syntax_only(unify(s)))]


def cases():
    """(file, name, code) of every agent case, as psh-spec reads them."""
    for f in sorted(os.listdir(HERE)):
        if not f.endswith(".test.sh"):
            continue
        name, lines = None, []
        for line in open(os.path.join(HERE, f), encoding="utf-8"):
            if line.startswith("#### "):
                if name is not None:
                    yield f, name, "".join(lines)
                name, lines = line[5:].strip(), []
            elif name is not None and not line.startswith("#"):
                lines.append(line)
        if name is not None:
            yield f, name, "".join(lines)


def skeletons(code, words, opts, variables):
    """The skeleton of the whole code and of each top-level command in it."""
    out = set()
    segs, _, _ = sc.template(code.strip())
    out.add(sc.render(segs, words, opts, variables).strip())
    src = code.encode()
    if sc.PARSER is None:
        sc.PARSER = sc.ts.Parser(sc.ts.Language(sc.tsb.language()))
    tree = sc.PARSER.parse(src)
    for node in tree.root_node.named_children:
        if node.type == "comment":
            continue
        text = src[node.start_byte:node.end_byte].decode()
        segs, _, _ = sc.template(text)
        out.add(sc.render(segs, words, opts, variables).strip())
    return out


def main():
    words, opts, variables = sc.load_vocab()
    missing_n = int(sys.argv[sys.argv.index("--missing") + 1]) if "--missing" in sys.argv else 0
    corpus = []
    for line in open(os.path.join(ROOT, "tests", "shapes", "shapes.txt"), encoding="utf-8"):
        count, _, sk = line.rstrip("\n").partition("\t")
        corpus.append((int(count), sc.check.unescape(sk)))
    total = sum(c for c, _ in corpus)
    mine = set()
    n = 0
    for _, _, code in cases():
        n += 1
        mine |= skeletons(code, words, opts, variables)
    print(f"{n} cases; corpus shapes stand for {total} of {CORPUS_COMMANDS} commands")
    for level, f in LEVELS:
        have = {f(s) for s in mine}
        covered = sum(c for c, s in corpus if f(s) in have)
        print(f"{level:7} {covered:7} commands  {covered / total:6.1%} of the shapes  "
              f"{covered / CORPUS_COMMANDS:6.1%} of the corpus")
    if missing_n:
        f = LEVELS[1][1]
        have = {f(s) for s in mine}
        agg = {}
        for c, s in corpus:
            k = f(s)
            if k not in have:
                agg[k] = agg.get(k, 0) + c
        for k, c in sorted(agg.items(), key=lambda kv: -kv[1])[:missing_n]:
            print(f"{c:6}  {sc.check.escape(k)[:150]}")


if __name__ == "__main__":
    main()
