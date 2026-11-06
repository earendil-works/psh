#!/usr/bin/env python3
"""Aggregate commands.jsonl + features.jsonl (+ syntax.json) into results.json.

Only aggregate numbers and command *names* / flags end up in results.json.
"""
import datetime
import hashlib
import json
import os
import re
from collections import Counter, defaultdict

DATA = os.environ.get("PSH_CORPUS_DATA", os.path.expanduser("~/.cache/psh-corpus"))
HERE = os.path.dirname(os.path.abspath(__file__))

# Utilities that an in-process builtin set could plausibly implement
# (POSIX shell builtins + coreutils + common text tools).
BUILTIN_SET = set("""
cd pushd popd echo printf true false : test [ [[ set export unset local declare read exit return shift source .
trap eval wait break continue let shopt alias type command hash ulimit umask readonly typeset
ls cat head tail wc grep egrep fgrep rg sed awk sort uniq cut tr find xargs mkdir rm rmdir cp mv touch tee sleep
pwd date basename dirname realpath readlink stat diff cmp nl seq jq du df env printenv which chmod ln mktemp
paste comm tac rev base64 shasum md5 md5sum sha256sum sha1sum cksum od xxd hexdump file fold expr bc yes
column join split truncate id whoami uname hostname nproc tree timeout gtimeout time nice nohup kill
""".split())

SUB_SHOW = {"git", "cargo", "npm", "pnpm", "yarn", "bun", "gh", "uv", "go", "docker", "brew", "npx", "tmux",
            "jj", "rustup", "xcrun", "pip", "pip3", "deno", "swift", "launchctl", "kubectl"}

SRC_GROUP = {"agent": "agent", "agent-codemode": "agent-codemode", "agent-codemode-static": "agent-codemode",
             "p2": "p2", "p2-codemode": "p2-codemode", "p2-codemode-static": "p2-codemode"}

FEATURES = [
    ("pipe", "pipeline `|`"), ("and", "`&&`"), ("or", "`||`"), ("semicolon", "`;` separator"),
    ("multi_statement", "multiple top-level statements (`;`/newline)"), ("cd_prefix", "starts with `cd X && …`/`cd X;`"),
    ("redir_fd_dup", "fd dup `2>&1` / `>&2`"), ("redir_devnull", "redirect to /dev/null"),
    ("redir_stderr_devnull", "`2>/dev/null`"), ("redir_to_file", "redirect stdout to file (`>`, `>>`, `&>`)"),
    ("redir_append", "append `>>`"), ("redir_in", "input redirect `<`"), ("redir_tmp", "redirect into /tmp"),
    ("heredoc", "here-doc `<<`"), ("heredoc_quoted", "here-doc with quoted delimiter `<<'EOF'`"),
    ("heredoc_unquoted", "here-doc unquoted (expanding)"), ("heredoc_dash", "here-doc `<<-`"),
    ("heredoc_to_interp", "here-doc fed to python/node/sh"), ("heredoc_to_cat", "`cat <<EOF` (> file)"), ("heredoc_to_other", "here-doc to other command (git/gh/psql/…)"),
    ("herestring", "here-string `<<<`"), ("cmd_subst", "command substitution `$(…)`"), ("backtick", "backticks"),
    ("proc_subst", "process substitution `<(…)`"), ("subshell", "subshell `( … )`"), ("brace_group", "group `{ …; }`"),
    ("for", "`for … in`"), ("for_c", "`for ((…))`"), ("while", "`while`/`until`"), ("if", "`if`"),
    ("case", "`case`"), ("function", "function definition"), ("dbl_bracket", "`[[ … ]]`"),
    ("single_bracket", "`[ … ]`"), ("arith_cmd", "`(( … ))`"), ("arith_expansion", "`$(( … ))`"),
    ("array", "array literal `=( … )`"), ("array_ref", "array ref `${a[@]}`"),
    ("var_ref", "`$VAR`"), ("param_expansion", "`${…}`"), ("param_expansion_op", "`${x:-y}`/`${x#…}`/`${x/…}` ops"),
    ("env_prefix", "env-assignment prefix `FOO=1 cmd`"), ("assignment", "plain assignment `x=…`"),
    ("declaration", "`export`/`local`/`declare`"), ("glob_word", "unquoted glob word (`*`, `?`, `[..]`)"),
    ("brace_expansion", "brace expansion `{a,b}`/`{1..3}`"), ("background", "background `&`"),
    ("negation", "`! cmd`"), ("set_e", "`set -e`"), ("set_u", "`set -u`"), ("set_pipefail", "`set -o pipefail`"),
    ("set_x", "`set -x`"), ("ansi_c_string", "`$'…'` ANSI-C string"), ("single_quoted", "single-quoted string"),
    ("double_quoted", "double-quoted string"), ("line_continuation", "backslash line continuation"),
    ("comment", "comment `#`"), ("parse_error", "tree-sitter parse error"),
]

COMPAT_PATTERNS = [
    # (utility, label, regex on raw command)
    ("sed", "sed -i (any)", r"\bsed\s+(?:-[a-zA-Z]+\s+)*-[a-zA-Z]*i"),
    ("sed", "sed -i '' (BSD form)", r"\bsed\s+(?:-\w+\s+)*-i\s*(?:''|\"\")"),
    ("sed", "sed -i.ext (suffix attached, portable)", r"\bsed\s+(?:-\w+\s+)*-i\.\w+"),
    ("sed", "sed -i <script> (GNU form; breaks on BSD)", r"\bsed\s+(?:-[a-hj-zA-Z]+\s+)*-i\s+(?:-e\s+)?['\"]?(?:s|\d|/|y)[^'\" ]"),
    ("sed", "sed -E / -r", r"\bsed\s+(?:-\w+\s+)*-[a-zA-Z]*[Er]\b"),
    ("sed", "sed -n 'X,Yp' (range print)", r"\bsed\s+-n\s+['\"]?\d+(?:,\d+)?p"),
    ("sed", "sed GNU-only escapes (\\+ \\? \\| in BRE, \\s)", r"\bsed\b[^|;&]*['\"][^'\"]*\\[+?|s][^'\"]*['\"]"),
    ("grep", "grep -P (PCRE; not in BSD grep)", r"\bgrep\s+(?:-\w+\s+)*-[a-zA-Z]*P"),
    ("grep", "grep -E", r"\bgrep\s+(?:-\w+\s+)*-[a-zA-Z]*E"),
    ("grep", "grep -r/-R", r"\bgrep\s+(?:-\w+\s+)*-[a-zA-Z]*[rR]"),
    ("grep", "grep --include/--exclude", r"\bgrep\b[^|;&]*--(?:include|exclude)"),
    ("grep", "grep -o", r"\bgrep\s+(?:-\w+\s+)*-[a-zA-Z]*o"),
    ("grep", "grep --color/-z/--null-data/-T", r"\bgrep\b[^|;&]*(?:--color|--null-data|\s-z\b|\s-T\b)"),
    ("find", "find -printf (GNU)", r"\bfind\b[^|;&]*-printf"),
    ("find", "find -maxdepth/-mindepth", r"\bfind\b[^|;&]*-(?:max|min)depth"),
    ("find", "find -regex / -regextype", r"\bfind\b[^|;&]*-regex"),
    ("find", "find -delete", r"\bfind\b[^|;&]*-delete"),
    ("find", "find -exec … {} +", r"\bfind\b[^|;&]*-exec\b.*\{\}\s*\+"),
    ("find", "find -exec … \\;", r"\bfind\b[^|;&]*-exec\b.*\\;"),
    ("find", "find -print0", r"\bfind\b[^|;&]*-print0"),
    ("find", "find -newer/-mtime/-mmin", r"\bfind\b[^|;&]*-(?:newer|mtime|mmin)"),
    ("find", "find without path (GNU default .)", r"(?:^|[|;&]\s*)find\s+-"),
    ("xargs", "xargs -r / --no-run-if-empty (GNU)", r"\bxargs\s+(?:-\w+\s+)*(?:-[a-zA-Z]*r\b|--no-run-if-empty)"),
    ("xargs", "xargs -0", r"\bxargs\s+(?:-\w+\s+)*-[a-zA-Z]*0"),
    ("xargs", "xargs -I", r"\bxargs\s+(?:-\w+\s+)*-I"),
    ("xargs", "xargs -P", r"\bxargs\s+(?:-\w+\s+)*-P"),
    ("xargs", "xargs -d (GNU)", r"\bxargs\s+(?:-\w+\s+)*-d"),
    ("date", "date -d / --date (GNU)", r"\bdate\s+(?:-\w+\s+)*(?:-d\b|--date)"),
    ("date", "date -v / -j -f (BSD)", r"\bdate\s+(?:-\w+\s+)*-(?:v|j)\b"),
    ("date", "date +%N (GNU nanoseconds)", r"\bdate\b[^|;&]*%N"),
    ("date", "date +%s", r"\bdate\b[^|;&]*\+%s"),
    ("date", "date -u / -Iseconds / --iso", r"\bdate\b[^|;&]*(?:\s-u\b|-I|--iso|--rfc)"),
    ("stat", "stat -c / --format (GNU)", r"\bstat\s+(?:-\w+\s+)*(?:-c\b|--format|--printf)"),
    ("stat", "stat -f (BSD)", r"\bstat\s+(?:-\w+\s+)*-f\b"),
    ("readlink", "readlink -f", r"\breadlink\s+-[a-zA-Z]*f"),
    ("realpath", "realpath", r"\brealpath\b"),
    ("sort", "sort -V (version)", r"\bsort\s+(?:-\w+\s+)*-[a-zA-Z]*V"),
    ("sort", "sort -h", r"\bsort\s+(?:-\w+\s+)*-[a-zA-Z]*h"),
    ("head", "head -n -N (GNU negative)", r"\bhead\s+-n\s*-\d"),
    ("head", "head -c", r"\bhead\s+(?:-\w+\s+)*-c"),
    ("tail", "tail -n +N", r"\btail\s+-n\s*\+\d"),
    ("timeout", "timeout (GNU coreutils; not on stock macOS)", r"(?:^|[|;&(]\s*|\s)timeout\s+\d"),
    ("gtimeout", "gtimeout", r"\bgtimeout\b"),
    ("tac", "tac (GNU)", r"(?:^|[|;&]\s*)tac\b"),
    ("nproc", "nproc (GNU)", r"\bnproc\b"),
    ("sysctl", "sysctl -n hw.ncpu (macOS)", r"sysctl\s+-n\s+hw\."),
    ("base64", "base64 -d/--decode", r"\bbase64\s+(?:-\w+\s+)*(?:-d\b|--decode)"),
    ("base64", "base64 -D (BSD) / -w (GNU)", r"\bbase64\s+(?:-\w+\s+)*-(?:D|w)\b"),
    ("md5", "md5sum/sha256sum (GNU)", r"\b(?:md5sum|sha256sum|sha1sum)\b"),
    ("md5", "md5 / shasum (BSD/perl)", r"(?:^|[|;&]\s*)(?:md5|shasum)\b"),
    ("cp", "cp -a / -r / -R", r"\bcp\s+(?:-\w+\s+)*-[a-zA-Z]*[arR]"),
    ("cp", "cp --parents / -t (GNU)", r"\bcp\b[^|;&]*(?:--parents|\s-t\s)"),
    ("ls", "ls --color/--sort/--time-style (GNU)", r"\bls\b[^|;&]*--(?:color|sort|time-style|group-directories)"),
    ("du", "du --max-depth (GNU) / -d (BSD)", r"\bdu\b[^|;&]*(?:--max-depth|\s-d\s*\d)"),
    ("mktemp", "mktemp -d", r"\bmktemp\s+(?:-\w+\s+)*-[a-zA-Z]*d"),
    ("mktemp", "mktemp -t", r"\bmktemp\s+(?:-\w+\s+)*-[a-zA-Z]*t"),
    ("awk", "awk gawk-only (gensub/strftime/asort/-i inplace)", r"\b(?:g?awk)\b.*(?:gensub|strftime|asorti?\(|-i\s+inplace|systime)"),
    ("wc", "wc -l", r"\bwc\s+-l"),
    ("seq", "seq", r"(?:^|[|;&(\s])seq\s"),
    ("echo", "echo -e / -n", r"\becho\s+-[en]\b"),
    ("ps", "ps aux / ps -ef", r"\bps\s+(?:aux|-ef|-eo|axo|-o)"),
    ("lsof", "lsof -i / -ti", r"\blsof\s+(?:-\w+\s+)*-[a-zA-Z]*i"),
    ("tar", "tar --exclude / GNU long opts", r"\btar\b[^|;&]*--"),
    ("touch", "touch -d / -t", r"\btouch\s+-[dt]\b"),
    ("install", "install -D (GNU)", r"\binstall\s+(?:-\w+\s+)*-D"),
    ("ln", "ln -sf / -sfn", r"\bln\s+-s"),
    ("rm", "rm -rf", r"\brm\s+(?:-\w+\s+)*-[a-zA-Z]*r[a-zA-Z]*f|\brm\s+-[a-zA-Z]*f[a-zA-Z]*r"),
    ("pbcopy", "pbcopy/pbpaste/open/osascript (macOS-only)", r"(?:^|[|;&\s])(?:pbcopy|pbpaste|osascript|open\s+-a|sips|xcrun|defaults\s+read|launchctl|mdfind|plutil|caffeinate|ioreg)\b"),
]

FAIL_PATTERNS = [
    ("timeout", None),
    ("aborted", None),
    ("command not found", r"command not found|: not found\b|No such file or directory.*\(os error 2\)"),
    ("zsh: no matches found (glob)", r"zsh:\d*:? no matches found"),
    ("shell syntax/quoting error", r"syntax error near unexpected token|unexpected EOF while looking for matching|parse error near|unmatched [\"'`]|bad substitution|unterminated (?:quoted )?string|syntax error: unexpected end of file|unexpected end of file|zsh:\d*: parse error|unexpected token"),
    ("BSD/GNU option mismatch", r"(?:illegal|invalid|unrecognized|unknown) option|unknown primary or operator|invalid command code|extra characters at the end of .* command|illegal time format|unknown action|undefined label"),
    ("usage message", r"(?m)^usage: |^Usage: "),
    ("permission denied", r"[Pp]ermission denied|Operation not permitted"),
    ("arg list too long", r"[Aa]rgument list too long"),
    ("network error", r"Could not resolve host|Connection refused|Failed to connect|curl: \(\d+\)|getaddrinfo|ECONNREFUSED|ENOTFOUND|SSL"),
    ("rust build/test failure", r"error\[E\d+\]|error: could not compile|test result: FAILED|error: test failed|panicked at"),
    ("js/ts build/test failure", r"npm (?:ERR|error)|ERR_PNPM|error TS\d+|Tests? +\d+ failed|FAIL\s|✗|AssertionError|vitest"),
    ("python error", r"Traceback \(most recent call last\)|SyntaxError:|ModuleNotFoundError|ImportError"),
    ("node runtime error", r"node:internal|Error: Cannot find module|ReferenceError|TypeError:|SyntaxError:"),
    ("git error", r"(?m)^fatal: |^error: (?:pathspec|Your local changes|failed to push|cannot)"),
    ("missing file", r"No such file or directory|cannot access|does not exist|not a directory"),
]


NF_RE = re.compile(r"(?m)^(?:\S*(?:bash|sh|zsh|env)(?:: line \d+)?:\s*)?(?:line \d+:\s*)?([\w.+-]+): (?:command )?not found\s*$|"
                   r"zsh:\d*:? command not found: ([\w.+-]+)")
SILENT = [
    ("command not found", re.compile(r"command not found")),
    ("zsh: no matches found", re.compile(r"zsh:\d*:? no matches found")),
    ("illegal/invalid/unrecognized option", re.compile(r"(?:illegal|invalid|unrecognized) option")),
    ("shell syntax error", re.compile(r"syntax error near unexpected token|unexpected EOF while looking|parse error near|bad substitution")),
    ("No such file or directory", re.compile(r"No such file or directory")),
]
OPT_ERR_RE = re.compile(r"(?m)^(?:\S*/)?([a-z][\w.-]{0,20}): (?:\d+: )?(?:illegal|invalid|unrecognized|unknown) (?:option|primary|command code)")
BASH4 = [
    ("mapfile/readarray", r"\b(?:mapfile|readarray)\b"),
    ("declare/local -A (assoc array)", r"\b(?:declare|local|typeset)\s+-[a-zA-Z]*A"),
    ("${x,,} / ${x^^} case conversion", r"\$\{\w+(?:,,|\^\^)"),
    ("|& (pipe stderr)", r"\|&"),
    ("&>> (append both)", r"&>>"),
    ("&> (redirect both)", r"&>(?!>)"),
    ("shopt -s globstar / **", r"globstar"),
    ("coproc", r"\bcoproc\b"),
    ("${x@Q} transformations", r"\$\{\w+@[QEPAa]\}"),
    ("read -d '' / -a", r"\bread\s+(?:-\w+\s+)*-[a-zA-Z]*[da]"),
    ("printf -v", r"\bprintf\s+-v\b"),
    ("[[ =~ ]] regex", r"\[\[[^\]]*=~"),
    ("$EPOCHSECONDS/$SRANDOM/BASH_REMATCH", r"EPOCHSECONDS|EPOCHREALTIME|SRANDOM|BASH_REMATCH"),
    ("zsh-only syntax (=(..), ${(f)..}, setopt)", r"\bsetopt\b|\$\{\([a-zA-Z@]+\)|\bautoload\b"),
]


def pct(a, b):
    return round(100.0 * a / b, 2) if b else 0.0


def main():
    syntax = {}
    sp = os.path.join(DATA, "syntax.json")
    if os.path.exists(sp):
        syntax = json.load(open(sp))
    R = {}
    by_src = Counter()
    sessions = defaultdict(set)
    ts_range = {}
    feat_any = defaultdict(Counter)  # src -> feature -> commands with feature
    feat_total = Counter()
    name_df = Counter()
    name_occ = Counter()
    sub_df = Counter()
    eff_sets = []
    cat_multi = defaultdict(Counter)
    cat_primary = defaultdict(Counter)
    pure_read = Counter()
    flags_by_util = defaultdict(Counter)
    compat = defaultdict(lambda: [0, 0, 0])  # label -> [commands, with result, failed]
    compat_fail_kind = defaultdict(Counter)
    fails = Counter()
    fail_src = defaultdict(Counter)
    fail_kinds = Counter()
    fail_kinds_src = defaultdict(Counter)
    benign = Counter()
    exit_codes = Counter()
    not_found = Counter()
    not_found_failed = Counter()
    silent = defaultdict(lambda: [0, 0])  # label -> [present in output, of which exit 0]
    opt_err = Counter()
    bash4 = Counter()
    outlim = Counter()
    OUTLIM_RE = [(l, re.compile(p)) for l, p in (
        ("pipes into head", r"\|\s*head\b"), ("pipes into tail", r"\|\s*tail\b"),
        ("2>&1 | head/tail", r"2>&1\s*\|\s*(?:head|tail)\b"),
        ("sed -n 'N,Mp' file (paged read)", r"\bsed\s+-n\s+['\"]?\d+,\d+p"),
        ("any output limiter (head/tail/sed -n N,Mp)", r"\|\s*(?:head|tail)\b|\bsed\s+-n\s+['\"]?\d+,\d+p|^\s*(?:head|tail)\s"),
        ("|| true / || : (ignore failure)", r"\|\|\s*(?:true|:)\b"),
        ("2>/dev/null", r"2>\s*/dev/null"),
        ("; echo \"exit=$?\" style status echo", r"echo\s+[\"']?[^\"'\n]{0,20}\$\?"),
    )]
    not_found_samples = Counter()
    fail_by_cat = defaultdict(lambda: [0, 0])
    fail_by_first = defaultdict(lambda: [0, 0])
    timeouts_first = Counter()
    sleeps = Counter()
    lr = Counter()
    lr_src = defaultdict(Counter)
    dur = defaultdict(list)
    timeout_args = Counter()
    lengths = defaultdict(list)
    lines_hist = Counter()
    syntax_res = defaultdict(Counter)
    ts_tree_vs_bash = Counter()
    windows = Counter()
    win_fail = Counter()
    ssh = Counter()
    nested_status = Counter()
    month = defaultdict(Counter)
    n = 0

    cf = open(os.path.join(DATA, "commands.jsonl"))
    ff = open(os.path.join(DATA, "features.jsonl"))
    compat_re = [(u, l, re.compile(p)) for u, l, p in COMPAT_PATTERNS]
    BASH4_RE = [(l, re.compile(p)) for l, p in BASH4]
    fail_re = [(k, re.compile(p) if p else None) for k, p in FAIL_PATTERNS]
    for cl, fl in zip(cf, ff):
        r = json.loads(cl)
        f = json.loads(fl)
        assert r["cid"] == f["cid"]
        n += 1
        src = SRC_GROUP[r["src"]]
        by_src[src] += 1
        sessions[src].add(r["sid"])
        if r.get("ts"):
            a, b = ts_range.get(src, (1 << 62, 0))
            ts_range[src] = (min(a, r["ts"]), max(b, r["ts"]))
            month[src][datetime.datetime.fromtimestamp(r["ts"] / 1000).strftime("%Y-%m")] += 1
        cmd = r["cmd"]
        ft = f["feat"]
        lengths[src].append(ft.get("bytes", len(cmd)))
        ln = ft.get("lines", 1)
        lines_hist["1" if ln == 1 else "2-5" if ln <= 5 else "6-20" if ln <= 20 else "21-100" if ln <= 100 else ">100"] += 1
        for k, v in ft.items():
            if k in ("lines", "bytes"):
                continue
            if v:
                feat_any[src][k] += 1
                feat_any["all"][k] += 1
        names = f["names"]
        names = ["<local-script>" if x.startswith("<path>") else x for x in names]
        for x in set(names):
            name_df[x] += 1
        for x in names:
            name_occ[x] += 1
        for x in set(f["subs"]):
            if x.split(" ", 1)[0] in SUB_SHOW:
                sub_df[x] += 1
        eff = set(f["eff"])
        eff_sets.append((src, eff))
        cats = set(f["cats"])
        for c in cats:
            cat_multi[src][c] += 1
            cat_multi["all"][c] += 1
        prio = ["windows", "remote-vm", "pkg-install", "vcs-mutate", "file-mutate", "build-test", "interp-inline", "shell-c",
                "script-run", "network", "long-running", "process", "interactive-or-gui", "other", "read-only",
                "echo", "shell-builtin", "cd", "dynamic", "empty-or-assign"]
        prim = next((p for p in prio if p in cats), "empty-or-assign")
        cat_primary[src][prim] += 1
        cat_primary["all"][prim] += 1
        if cats and cats <= {"read-only", "cd", "echo", "shell-builtin"}:
            pure_read[src] += 1
            pure_read["all"] += 1
        for u, fl_ in f["flags"]:
            if re.fullmatch(r"-{1,2}[A-Za-z0-9][A-Za-z0-9_.-]{0,30}|-\d+|-", fl_):
                flags_by_util[u][fl_] += 1
        hs = hashlib.sha1(cmd.encode("utf-8", "replace")).hexdigest()
        if hs in syntax:
            bok, zok = syntax[hs]
            syntax_res[src][("bash_ok" if bok else "bash_fail") if bok is not None else "bash_na"] += 1
            syntax_res[src][("zsh_ok" if zok else "zsh_fail") if zok is not None else "zsh_na"] += 1
            ts_tree_vs_bash[(bool(ft.get("parse_error")), bok)] += 1

        has_res = r.get("has_result")
        out = (r.get("out_head") or "") + "\n" + (r.get("out_tail") or "")
        failed = bool(has_res and (r.get("is_error") or (r.get("exit_code") not in (0, None))))
        if r.get("nested_status"):
            nested_status[r["nested_status"]] += 1
        # compat
        for u, label, rx in compat_re:
            if u in cmd and rx.search(cmd):
                compat[label][0] += 1
                if has_res:
                    compat[label][1] += 1
                    if failed:
                        compat[label][2] += 1
                        if re.search(r"(?:illegal|invalid|unrecognized|unknown) option|unknown primary|invalid command code|extra characters|illegal time format|command not found|usage:", out):
                            compat_fail_kind[label]["flag/usage/not-found error"] += 1

        first = f.get("first") or "<none>"
        if has_res:
            fails["with_result"] += 1
            fail_src[src]["with_result"] += 1
            for c in cats:
                fail_by_cat[c][0] += 1
            fail_by_first[first][0] += 1
            if failed:
                fails["failed"] += 1
                fail_src[src]["failed"] += 1
                ec = r.get("exit_code")
                exit_codes["timeout" if r.get("timed_out") else "aborted" if r.get("aborted") else str(ec)] += 1
                for c in cats:
                    fail_by_cat[c][1] += 1
                fail_by_first[first][1] += 1
                kind = None
                if r.get("timed_out"):
                    kind = "timeout"
                    timeouts_first[first] += 1
                elif r.get("aborted"):
                    kind = "aborted"
                else:
                    for k, rx in fail_re:
                        if rx is not None and rx.search(out):
                            kind = k
                            break
                if kind is None:
                    body = out.strip()
                    last_pipe_cmd = f["eff"][-1] if f["eff"] else ""
                    if ec == 1 and (body.startswith("(no output)") or len(body) < 40) and any(
                            x in eff for x in ("grep", "rg", "diff", "test", "[", "pgrep", "cmp", "git")):
                        kind = "benign: no-match/false (exit 1, no output)"
                    elif ec == 141:
                        kind = "SIGPIPE (141)"
                    elif ec in (130, 137, 143, -1, 255) and "kill" in cmd:
                        kind = "killed/signal"
                    else:
                        kind = "other non-zero (tool-specific)"
                fail_kinds[kind] += 1
                fail_kinds_src[src][kind] += 1
                pass
        nfs = set()
        for m in NF_RE.finditer(out):
            nf2 = (m.group(1) or m.group(2) or "").rsplit("/", 1)[-1]
            if re.fullmatch(r"[A-Za-z0-9_.+-]{1,30}", nf2):
                nfs.add(nf2)
        for nf2 in nfs:
            not_found[nf2] += 1
            not_found_failed[nf2] += int(failed)
        for lab, rx in OUTLIM_RE:
            if rx.search(cmd):
                outlim[lab] += 1
        if has_res:
            for lab, rx in SILENT:
                if rx.search(out):
                    silent[lab][0] += 1
                    silent[lab][1] += int(not failed)
            for m in OPT_ERR_RE.finditer(out):
                opt_err[m.group(1)] += 1
        for lab, rx in BASH4_RE:
            if rx.search(cmd):
                bash4[lab] += 1
        # long running
        if f.get("sleep") is not None:
            s = f["sleep"]
            sleeps["<=1s" if s <= 1 else "<=5s" if s <= 5 else "<=30s" if s <= 30 else "<=120s" if s <= 120 else ">120s"] += 1
        if ft.get("background"):
            lr["background &"] += 1
            lr_src[src]["background &"] += 1
        for key, rx in (("nohup", r"\bnohup\b"), ("tail -f/-F", r"\btail\s+(?:-\w+\s+)*-[a-zA-Z]*[fF]\b"),
                        ("watch", r"(?:^|[|;&]\s*)watch\s"), ("sleep", r"\bsleep\s+[\d.]"),
                        ("timeout wrapper (timeout/gtimeout/timeout.sh)", r"\b(?:g?timeout\s+\d|timeout\.sh)"),
                        ("server/dev-server/watch-mode hint", None),
                        ("tmux (send-keys/capture-pane, etc.)", r"\btmux\b"),
                        ("polling loop (loop containing sleep)", r"(?:while|until|for)\b[\s\S]*\bsleep\b"),
                        ("kill/pkill/lsof (process cleanup)", r"\b(?:pkill|killall|kill\s+-?\w*\s*\$?\w|lsof\s+-t?i)"),
                        ("disown/setsid", r"\b(?:disown|setsid)\b"),
                        ("interactive-risk (vim/less/top/git rebase -i/python REPL)", r"(?:^|[|;&]\s*)(?:vim?|nano|less|more|top|htop)\b(?!\S)|git\s+rebase\s+-i\b|git\s+add\s+-p\b")):
            if key.startswith("server"):
                hit = bool(ft.get("server_hint"))
            else:
                hit = bool(re.search(rx, cmd))
            if hit:
                lr[key] += 1
                lr_src[src][key] += 1
        if r.get("dur_ms") is not None:
            dur[src].append(r["dur_ms"])
        ta = r.get("timeout_arg")
        if ta is not None:
            try:
                t = float(ta)
                timeout_args["<=30s" if t <= 30 else "<=120s" if t <= 120 else "<=600s" if t <= 600 else ">600s"] += 1
            except Exception:
                timeout_args["invalid"] += 1
        else:
            timeout_args["none"] += 1
        # windows / remote
        if re.search(r"\bprlctl\b", cmd):
            windows["prlctl (Parallels VM) commands"] += 1
            if re.search(r"Windows", cmd):
                windows["prlctl exec into Windows VM"] += 1
                if has_res and failed:
                    win_fail["prlctl Windows failed"] += 1
                if has_res:
                    win_fail["prlctl Windows with result"] += 1
        if re.search(r"\b(?:powershell(?:\.exe)?|pwsh)\b", cmd, re.I):
            windows["mentions powershell/pwsh"] += 1
        if re.search(r"\bcmd(?:\.exe)?\s+/[cCkK]\b", cmd):
            windows["cmd /c"] += 1
        if re.search(r"\b[A-Z]:(?:\\{1,2}|/)(?:Users|Windows|Program|pitest|temp|tmp|dev|src|work)\b", cmd):
            windows["contains drive path C:\\"] += 1
        if re.search(r"\bwsl(?:\.exe)?\b", cmd):
            windows["wsl"] += 1
        if (r.get("cwd") or "").startswith(tuple("ABCDEFGHIJKLMNOPQRSTUVWXYZ")) and ":" in (r.get("cwd") or "")[:3]:
            windows["session cwd is a Windows path"] += 1
        if re.search(r"is not recognized as an internal or external command", out):
            windows["output: 'is not recognized as an internal or external command'"] += 1
        if re.search(r"(?:^|[|;&(]\s*)ssh\s", cmd):
            ssh["ssh"] += 1
            if has_res:
                ssh["ssh with result"] += 1
                if failed:
                    ssh["ssh failed"] += 1
        if re.search(r"(?:^|[|;&(]\s*)(?:scp|rsync)\s", cmd):
            ssh["scp/rsync"] += 1
        if re.search(r"\bdocker\s+(?:exec|run)\b", cmd):
            ssh["docker exec/run"] += 1

    # coverage curves
    # rank by document frequency of first-word names (excluding <path>/<dynamic> buckets merged)
    def bucket(x):
        return "<local-script>" if x.startswith("<path>") else x
    eff_df = Counter()
    for _, e in eff_sets:
        for x in {bucket(y) for y in e}:
            eff_df[x] += 1
    ranked = [x for x, _ in eff_df.most_common()]
    coverage = {}
    for N in (5, 10, 20, 30, 50, 100, 200):
        top = set(ranked[:N])
        cov = Counter()
        for src, e in eff_sets:
            if {bucket(y) for y in e} <= top:
                cov[src] += 1
                cov["all"] += 1
        coverage[N] = {k: pct(v, by_src[k] if k != "all" else n) for k, v in cov.items()}
    # builtin-set coverage
    bi = Counter()
    bi_one_ext = Counter()
    ext_count_hist = Counter()
    ext_df = Counter()
    for src, e in eff_sets:
        e2 = {bucket(y) for y in e}
        ext = e2 - BUILTIN_SET
        k = len(ext)
        ext_count_hist["0" if k == 0 else "1" if k == 1 else "2" if k == 2 else "3+"] += 1
        for x in ext:
            ext_df[x] += 1
        if k == 0:
            bi[src] += 1
            bi["all"] += 1
        if k <= 1:
            bi_one_ext[src] += 1
            bi_one_ext["all"] += 1

    def dstats(v):
        if not v:
            return {}
        v = sorted(v)
        q = lambda p: v[min(len(v) - 1, int(p * len(v)))]
        return {"n": len(v), "p50_ms": q(0.5), "p90_ms": q(0.9), "p99_ms": q(0.99),
                "gt10s_pct": pct(sum(1 for x in v if x > 10000), len(v)),
                "gt60s_pct": pct(sum(1 for x in v if x > 60000), len(v))}

    def lstats(v):
        v = sorted(v)
        q = lambda p: v[min(len(v) - 1, int(p * len(v)))]
        return {"p50": q(0.5), "p90": q(0.9), "p99": q(0.99), "max": v[-1]}

    R["total"] = n
    R["by_src"] = dict(by_src)
    R["sessions"] = {k: len(v) for k, v in sessions.items()}
    R["ts_range"] = {k: [datetime.datetime.fromtimestamp(a / 1000).strftime("%Y-%m-%d"),
                         datetime.datetime.fromtimestamp(b / 1000).strftime("%Y-%m-%d")] for k, (a, b) in ts_range.items()}
    R["month"] = {k: dict(sorted(v.items())) for k, v in month.items()}
    R["extract_stats"] = json.load(open(os.path.join(DATA, "extract_stats.json")))
    R["length_bytes"] = {k: lstats(v) for k, v in lengths.items()}
    R["lines_hist"] = dict(lines_hist)
    R["features"] = {k: dict(v) for k, v in feat_any.items()}
    R["feature_labels"] = FEATURES
    R["top_names"] = [(x, c, name_occ[x]) for x, c in name_df.most_common(150)]
    R["top_subs"] = sub_df.most_common(120)
    R["eff_top"] = eff_df.most_common(120)
    R["coverage"] = coverage
    R["builtin_set"] = sorted(BUILTIN_SET)
    R["builtin_cov"] = {k: pct(v, by_src[k] if k != "all" else n) for k, v in bi.items()}
    R["builtin_one_ext_cov"] = {k: pct(v, by_src[k] if k != "all" else n) for k, v in bi_one_ext.items()}
    R["ext_count_hist"] = dict(ext_count_hist)
    R["ext_top"] = ext_df.most_common(60)
    R["cat_multi"] = {k: dict(v) for k, v in cat_multi.items()}
    R["cat_primary"] = {k: dict(v) for k, v in cat_primary.items()}
    R["pure_read"] = dict(pure_read)
    top_utils = ["grep", "rg", "sed", "head", "tail", "find", "ls", "cat", "sort", "wc", "cut", "awk", "xargs",
                 "git", "cargo", "npm", "curl", "python3", "node", "rm", "cp", "mkdir", "date", "tr", "uniq", "jq",
                 "echo", "printf", "sleep", "tee", "nl", "diff", "stat", "du", "ps", "kill", "pkill", "lsof", "tar"]
    R["flags_by_util"] = {u: flags_by_util[u].most_common(25) for u in top_utils}
    R["compat"] = {k: v for k, v in compat.items()}
    R["compat_fail_kind"] = {k: dict(v) for k, v in compat_fail_kind.items()}
    R["fails"] = dict(fails)
    R["fail_src"] = {k: dict(v) for k, v in fail_src.items()}
    R["exit_codes"] = exit_codes.most_common(25)
    R["fail_kinds"] = fail_kinds.most_common()
    R["fail_kinds_src"] = {k: dict(v) for k, v in fail_kinds_src.items()}
    R["not_found"] = [(k, v, not_found_failed[k]) for k, v in not_found.most_common(60)]
    R["silent"] = dict(silent)
    R["opt_err"] = opt_err.most_common(30)
    R["bash4"] = bash4.most_common()
    R["outlim"] = dict(outlim)
    R["fail_by_cat"] = {k: v for k, v in fail_by_cat.items()}
    R["fail_by_first"] = sorted(((k, v[0], v[1]) for k, v in fail_by_first.items() if v[0] >= 300),
                                key=lambda x: -x[1])[:50]
    R["timeouts_first"] = timeouts_first.most_common(25)
    R["sleeps"] = dict(sleeps)
    R["long_running"] = dict(lr)
    R["long_running_src"] = {k: dict(v) for k, v in lr_src.items()}
    R["durations"] = {k: dstats(v) for k, v in dur.items()}
    R["timeout_args"] = dict(timeout_args)
    R["syntax"] = {k: dict(v) for k, v in syntax_res.items()}
    R["ts_vs_bash"] = {f"ts_err={a},bash_ok={b}": c for (a, b), c in ts_tree_vs_bash.items()}
    R["windows"] = dict(windows)
    R["win_fail"] = dict(win_fail)
    R["remote"] = dict(ssh)
    R["nested_status"] = dict(nested_status)
    json.dump(R, open(os.path.join(HERE, "results.json"), "w"), indent=1)
    print("wrote results.json; commands", n)


if __name__ == "__main__":
    main()
