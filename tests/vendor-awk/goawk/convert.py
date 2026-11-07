#!/usr/bin/env python3
# Converts the table of GoAWK's interpreter tests (interp/interp_test.go, `interpTests`;
# GoAWK is MIT, Copyright (c) 2022 Ben Hoyt, see LICENSE here) into cases.txt: one case
# per line, `src<TAB>stdin`, both escaped (\\ \n \t \xNN). Cases GoAWK marks `!gawk`
# (where gawk behaves differently) are left out: the expectations are gawk's.
#
#   tests/vendor-awk/goawk/convert.py vendor/goawk/interp/interp_test.go
import sys

def esc(b):
    out = []
    for c in b:
        if c == 0x5c: out.append('\\\\')
        elif c == 0x0a: out.append('\\n')
        elif c == 0x09: out.append('\\t')
        elif 0x20 <= c < 0x7f: out.append(chr(c))
        else: out.append('\\x%02x' % c)
    return ''.join(out)

def go_string(s, i):
    """A Go string literal at s[i]: (bytes, next index)."""
    if s[i] == '`':
        j = s.index('`', i + 1)
        return s[i+1:j].encode(), j + 1
    assert s[i] == '"', s[i:i+20]
    out = bytearray()
    j = i + 1
    while s[j] != '"':
        c = s[j]
        if c != '\\':
            out += c.encode(); j += 1; continue
        n = s[j+1]
        simple = {'n': 10, 't': 9, 'r': 13, 'a': 7, 'b': 8, 'f': 12, 'v': 11, '\\': 92, '"': 34, "'": 39}
        if n in simple:
            out.append(simple[n]); j += 2
        elif n == 'x':
            out.append(int(s[j+2:j+4], 16)); j += 4
        elif n == 'u':
            out += chr(int(s[j+2:j+6], 16)).encode(); j += 6
        elif n == 'U':
            out += chr(int(s[j+2:j+10], 16)).encode(); j += 10
        elif n in '01234567':
            out.append(int(s[j+1:j+4], 8)); j += 4
        else:
            raise ValueError('escape ' + n)
    return bytes(out), j + 1

def skip_ws(s, i):
    while True:
        while i < len(s) and s[i] in ' \t\r\n': i += 1
        if s.startswith('//', i):
            i = s.index('\n', i)
            continue
        return i

def expr(s, i):
    """String literals joined by +: (bytes or None if not a literal, next index)."""
    val = b''
    ok = True
    while True:
        i = skip_ws(s, i)
        if s[i] in '`"':
            b, i = go_string(s, i)
            if s.startswith('[1:]', i):
                b, i = b[1:], i + 4
            val += b
        else:
            ok = False
            depth = 0
            while depth > 0 or s[i] not in ',}':
                if s[i] in '({': depth += 1
                if s[i] in ')}': depth -= 1
                i += 1
        i = skip_ws(s, i)
        if s[i] == '+':
            i += 1
            continue
        return (val if ok else None), i

src = open(sys.argv[1]).read()
start = src.index('var interpTests = []interpTest{') + len('var interpTests = []interpTest{')
i = start
cases = []
while True:
    i = skip_ws(src, i)
    if src[i] == '}':
        break
    assert src[i] == '{', src[i:i+40]
    i += 1
    fields = []
    for k in range(5):
        v, i = expr(src, i)
        fields.append(v)
        i = skip_ws(src, i)
        if src[i] == ',':
            i += 1
    i = skip_ws(src, i)
    assert src[i] == '}', src[i:i+40]
    i += 1
    i = skip_ws(src, i)
    if src[i] == ',':
        i += 1
    cases.append(fields)

kept = 0
with open('cases.txt', 'w') as f:
    for prog, stdin, out, err, awkerr in cases:
        if prog is None or stdin is None:
            continue
        p = prog.decode('utf-8', 'replace')
        flags = p[p.index('#') + 1:].split() if '#' in p else []
        if '!gawk' in flags or '+posix' in flags:
            continue
        f.write(esc(prog) + '\t' + esc(stdin) + '\n')
        kept += 1
print(f'{len(cases)} cases, {kept} kept', file=sys.stderr)
