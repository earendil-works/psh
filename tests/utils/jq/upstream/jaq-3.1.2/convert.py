#!/usr/bin/env python3
"""Converts jaq's Rust tests (give, gives, yields!) into jq's .test format.

Usage: convert.py JAQ_CHECKOUT OUT_DIR. Cases whose arguments are not plain literals
(closures, Rust expressions) are skipped and counted.
"""
import json, os, re, sys

def rust_string(s, i):
    """Parses a Rust string literal at s[i]; returns (text, end) or None."""
    if s.startswith('r', i):
        m = re.match(r'r(#*)"', s[i:])
        if not m:
            return None
        hashes = m.group(1)
        end = s.find('"' + hashes, i + len(m.group(0)))
        return s[i + len(m.group(0)):end], end + 1 + len(hashes)
    if s[i] != '"':
        return None
    out, j = [], i + 1
    while j < len(s):
        c = s[j]
        if c == '\\':
            n = s[j + 1]
            if n == 'n': out.append('\n')
            elif n == 't': out.append('\t')
            elif n == 'r': out.append('\r')
            elif n == '0': out.append('\0')
            elif n == '\n':
                j += 2
                while s[j] in ' \t\n': j += 1
                continue
            elif n == 'u':
                m = re.match(r'\{([0-9a-fA-F]+)\}', s[j + 2:])
                out.append(chr(int(m.group(1), 16))); j += 2 + len(m.group(0)); continue
            else: out.append(n)
            j += 2
            continue
        if c == '"':
            return ''.join(out), j + 1
        out.append(c); j += 1
    return None

def split_args(s, i):
    """The top-level comma-separated arguments of a call whose '(' is at s[i]."""
    depth, j, start, args = 0, i, i + 1, []
    while j < len(s):
        c = s[j]
        if c == '"' or (c == 'r' and re.match(r'r#*"', s[j:])):
            r = rust_string(s, j)
            if r is None: return None, j
            j = r[1]; continue
        if c in '([{': depth += 1
        elif c in ')]}':
            depth -= 1
            if depth == 0:
                args.append(s[start:j]); return [a.strip() for a in args if a.strip()], j + 1
        elif c == ',' and depth == 1:
            args.append(s[start:j]); start = j + 1
        j += 1
    return None, j

def value(a):
    a = a.strip()
    m = re.match(r'json!\((.*)\)$', a, re.S)
    if m: a = m.group(1).strip()
    r = rust_string(a, 0) if a[:1] in '"r' else None
    if r and r[1] == len(a): return json.dumps(r[0])
    a2 = re.sub(r',(\s*[\]}])', r'\1', a)
    a2 = re.sub(r'(\d)_(\d)', r'\1\2', a2)
    try:
        v = json.loads(a2)
    except Exception:
        return None
    return json.dumps(v, ensure_ascii=False)

def filt(a):
    a = a.strip()
    r = rust_string(a, 0) if a[:1] in '"r' else None
    if not r or r[1] != len(a.rstrip()): return None
    return ' '.join(r[0].split('\n')) if '"' not in r[0] or '\n' not in r[0] else None

def convert(path):
    s = open(path).read()
    cases, skipped = [], 0
    for m in re.finditer(r'(?<![\w!])(give|gives|yields!)\(', s):
        kind = m.group(1)
        args, _ = split_args(s, m.end() - 1)
        line = s.count('\n', 0, m.start()) + 1
        if not args:
            skipped += 1; continue
        if kind == 'yields!':
            if len(args) != 3: skipped += 1; continue
            inp, f, outs = 'null', filt(args[1]), [value(args[2])]
        elif kind == 'give':
            if len(args) != 3: skipped += 1; continue
            inp, f, outs = value(args[0]), filt(args[1]), [value(args[2])]
        else:
            if len(args) != 3: skipped += 1; continue
            inp, f = value(args[0]), filt(args[1])
            arr = args[2].strip()
            if not (arr.startswith('[') and arr.endswith(']')): skipped += 1; continue
            inner, _ = split_args('(' + arr[1:-1] + ')', 0)
            outs = [value(x) for x in (inner or [])]
        if inp is None or f is None or any(o is None for o in outs) or '\n' in f:
            skipped += 1; continue
        cases.append((line, f, inp, outs))
    return cases, skipped

def main():
    src, out = sys.argv[1], sys.argv[2]
    total_skipped = 0
    for crate in ['jaq-core', 'jaq-std', 'jaq-json']:
        tdir = os.path.join(src, crate, 'tests')
        lines = [f'# Converted from jaq 3.1.2 ({crate}/tests/*.rs, MIT, see ../README.md)',
                 '# by convert.py: give/gives/yields! cases with literal arguments.', '']
        for name in sorted(os.listdir(tdir)):
            if not name.endswith('.rs'): continue
            cases, skipped = convert(os.path.join(tdir, name))
            total_skipped += skipped
            for line, f, inp, outs in cases:
                lines += [f'# {crate}/tests/{name}:{line}', f, inp] + outs + ['']
        open(os.path.join(out, crate.replace('jaq-', '') + '.test'), 'w').write('\n'.join(lines))
    print('skipped', total_skipped, file=sys.stderr)

main()
