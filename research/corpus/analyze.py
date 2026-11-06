#!/usr/bin/env python3
"""Parse every extracted command with tree-sitter-bash and aggregate statistics.

Input : $PSH_CORPUS_DATA/commands.jsonl (from extract.py; private)
Output: $PSH_CORPUS_DATA/features.jsonl  (per-command parsed features; private)
        research/corpus/results.json      (aggregate numbers only; safe to share)

Run with the venv that has tree-sitter + tree-sitter-bash:
  uv venv .venv && uv pip install --python .venv/bin/python tree-sitter tree-sitter-bash
  .venv/bin/python analyze.py
"""
import json
import os
import re
import sys
from collections import Counter, defaultdict
from multiprocessing import Pool

import tree_sitter as ts
import tree_sitter_bash as tsb

HOME = os.path.expanduser("~")
DATA = os.environ.get("PSH_CORPUS_DATA", os.path.join(HOME, ".cache/psh-corpus"))
HERE = os.path.dirname(os.path.abspath(__file__))

LANG = ts.Language(tsb.language())
PARSER = None

# Commands that run another command given as their arguments.
WRAPPERS = {"sudo", "env", "time", "timeout", "gtimeout", "nohup", "exec", "command", "builtin", "nice",
            "xargs", "watch", "caffeinate", "stdbuf", "unbuffer", "script", "chronic", "doas", "arch"}
SHELLS = {"bash", "sh", "zsh", "dash", "ksh", "fish"}
SUBCMD_TOOLS = {"git", "cargo", "npm", "pnpm", "yarn", "bun", "gh", "docker", "uv", "go", "brew", "pip", "pip3",
                "npx", "kubectl", "jj", "rustup", "deno", "swift", "xcrun", "xcodebuild", "launchctl", "tmux",
                "apt", "apt-get", "make", "just", "gcloud", "aws", "fly", "flyctl", "wrangler", "pi", "sqlite3",
                "podman", "nix", "systemctl", "defaults", "security", "codesign", "lldb", "hyperfine", "ollama"}

# ----------------------------------------------------------------------------- tree helpers


def txt(node, src):
    return src[node.start_byte:node.end_byte].decode("utf-8", "replace")


def unquote(word):
    """Best-effort static value of a shell word (drop quotes)."""
    w = word
    if len(w) >= 2 and w[0] == w[-1] and w[0] in "'\"":
        return w[1:-1]
    return w.replace("'", "").replace('"', "")


def walk(node):
    stack = [node]
    while stack:
        n = stack.pop()
        yield n
        stack.extend(reversed(n.children))


# ----------------------------------------------------------------------------- per-command analysis


def simple_commands(root, src):
    """Yield (name, args, node) for every simple command, unwrapping wrappers
    (sudo/env/timeout/xargs...) and recording `bash -c '...'` payloads."""
    out = []
    for n in walk(root):
        if n.type != "command":
            continue
        name_node = n.child_by_field_name("name")
        if name_node is None:
            continue
        words = [txt(name_node, src)]
        for c in n.children_by_field_name("argument"):
            words.append(txt(c, src))
        prefix_assign = any(c.type == "variable_assignment" for c in n.children)
        out.append((words, n, prefix_assign))
    return out


def norm_name(w):
    w = unquote(w)
    if not w:
        return "<empty>"
    if "$" in w or "`" in w:
        return "<dynamic>"
    if "/" in w:
        base = w.rstrip("/").rsplit("/", 1)[-1]
        if w.startswith(("./", "../")) or "/target/" in w or "/bin/" not in w and not w.startswith("/usr"):
            return "<path>/" + base if base else "<path>"
        return base  # /usr/bin/env, /bin/ls, /opt/homebrew/bin/x -> basename
    return w


def unwrap(words):
    """Return list of effective command word-lists: wrapper then wrapped."""
    res = [words]
    w = [unquote(x) for x in words]
    name = norm_name(words[0])
    i = 1
    if name in WRAPPERS:
        # skip options and option values / durations / VAR=val
        while i < len(w):
            a = w[i]
            if a.startswith("-"):
                # options with value
                if name == "timeout" and a in ("-s", "-k", "--signal", "--kill-after"):
                    i += 2
                    continue
                if name == "xargs" and a in ("-I", "-n", "-P", "-L", "-d", "-E", "-s") and len(a) == 2:
                    i += 2
                    continue
                if name == "sudo" and a in ("-u", "-g"):
                    i += 2
                    continue
                if name == "nice" and a == "-n":
                    i += 2
                    continue
                if name == "watch" and a in ("-n", "-d"):
                    i += 2 if a == "-n" else 1
                    continue
                i += 1
                continue
            if name == "env" and "=" in a:
                i += 1
                continue
            if name in ("timeout", "gtimeout") and re.fullmatch(r"[\d.]+[smhd]?", a):
                i += 1
                continue
            if name == "arch" and a.startswith("-"):
                i += 1
                continue
            break
        if i < len(words):
            res += unwrap(words[i:])
    return res


VALUE_OPTS = {"-C", "-c", "-S", "-L", "--filter", "-F", "--prefix", "-w", "--workspace", "--manifest-path",
              "-p", "--dir", "--git-dir", "--work-tree", "--repo", "-R", "--package", "--cwd", "-f", "--file",
              "--target", "-t", "-u", "--config", "--color", "--namespace", "-n", "--profile", "--region"}


def subcommand(words):
    skip = False
    for a in words[1:]:
        a = unquote(a)
        if skip:
            skip = False
            continue
        if a in VALUE_OPTS:
            skip = True
            continue
        if a.startswith("-"):
            continue
        if "=" in a and not a.startswith("="):
            continue
        return a
    return None


INTERP = {"python", "python3", "python3.12", "python3.13", "python3.14", "node", "perl", "ruby", "deno", "bun",
          "osascript", "php", "lua", "tsx", "ts-node", "Rscript", "julia", "swift", "uv", "uvx"}
READ_ONLY = {"ls", "cat", "rg", "grep", "egrep", "fgrep", "find", "fd", "head", "tail", "wc", "pwd", "file", "stat",
             "du", "df", "tree", "which", "type", "whereis", "diff", "cmp", "jq", "yq", "sort", "uniq", "cut", "tr",
             "xxd", "hexdump", "od", "realpath", "readlink", "basename", "dirname", "date", "printenv", "uname",
             "nl", "column", "less", "more", "strings", "otool", "nm", "objdump", "readelf", "ldd", "lipo", "sw_vers",
             "id", "whoami", "hostname", "test", "[", "[[", "true", "false", "comm", "paste", "fold", "rev", "tac",
             "md5", "md5sum", "shasum", "sha256sum", "sha1sum", "cksum", "base64", "seq", "expr", "bc", "awk",
             "gawk", "sed", "ast-grep", "sg", "zcat", "gzcat", "tokei", "cloc", "getconf", "sysctl", "nproc",
             "locale", "mdls", "mdfind", "plutil", "codesign", "spctl", "dwarfdump", "size", "bat", "exa", "eza",
             "unzip", "tar", "zipinfo", "pdftotext", "pdfinfo", "ffprobe", "identify", "sips", "command", "hash",
             "compgen", "ulimit", "locate", "lsd", "join", "split", "csplit", "fmt", "pr", "iconv", "jot",
             "printf", "echo", "fc-list", "file", "otool", "vm_stat", "system_profiler", "ioreg", "diskutil",
             "sqlite3", "xmllint", "tput", "stty", "tty", "uptime", "w", "who", "last", "env", "set", "export",
             "declare", "local", "read", "shift", "unset", "source", ".", "cd", "pushd", "popd", "dirs", "exit",
             "return", "break", "continue", "wait", "trap", "eval", "let", "shopt", "alias", "printf"}
MUTATE = {"mv", "rm", "mkdir", "cp", "touch", "chmod", "chown", "ln", "tee", "truncate", "rmdir", "install",
          "patch", "rsync", "ditto", "mktemp", "dd", "shred", "unlink", "trash", "xattr", "chflags", "setfile",
          "apply_patch", "zip", "gzip", "gunzip", "xz", "bzip2", "zstd", "7z", "split"}
GIT_READ = {"status", "log", "diff", "show", "rev-parse", "blame", "grep", "ls-files", "ls-tree", "describe",
            "shortlog", "reflog", "cat-file", "rev-list", "merge-base", "name-rev", "whatchanged", "remote",
            "config", "branch", "tag", "stash", "worktree", "for-each-ref", "show-ref", "help", "version",
            "--version", "check-ignore", "count-objects", "fsck", "verify-commit", "range-diff", "symbolic-ref",
            "diff-tree", "ls-remote", "submodule", "bisect", "notes", "var", "show-branch", "difftool"}
GIT_MUT = {"add", "commit", "push", "checkout", "switch", "reset", "rebase", "merge", "cherry-pick", "restore",
           "rm", "mv", "init", "clone", "fetch", "pull", "am", "apply", "revert", "clean", "gc", "update-index",
           "update-ref", "commit-tree", "read-tree", "write-tree", "format-patch", "filter-branch", "filter-repo",
           "lfs", "prune", "repack", "sparse-checkout", "maintenance"}
BUILD = {"cargo", "make", "gmake", "cmake", "ninja", "meson", "go", "pytest", "tox", "nox", "xcodebuild", "tsc",
         "vitest", "jest", "mocha", "eslint", "biome", "prettier", "ruff", "mypy", "pyright", "black", "isort",
         "flake8", "pylint", "gcc", "g++", "clang", "clang++", "cc", "c++", "rustc", "zig", "gradle", "gradlew",
         "mvn", "just", "rustfmt", "clippy-driver", "wasm-pack", "wasm-bindgen", "wasm-opt", "maturin", "tsx",
         "esbuild", "vite", "webpack", "rollup", "turbo", "nx", "lerna", "swiftc", "swift", "xcrun", "lldb",
         "gdb", "valgrind", "hyperfine", "insta", "cargo-insta", "nextest", "bazel", "buck", "scons", "ctest",
         "ld", "ar", "strip", "lipo", "emcc", "wasmtime", "wasmer", "node-gyp", "playwright", "cypress",
         "shellcheck", "shfmt", "stylua", "luacheck", "clang-format", "clang-tidy", "dprint", "oxlint", "knip",
         "svelte-check", "astro", "next", "nuxt", "docker", "podman", "act", "terraform", "pre-commit",
         "codesign", "xcode-select", "ibtool", "actool", "llvm-objdump", "llvm-cov", "llvm-profdata", "qemu-system-x86_64"}
JS_PM = {"npm", "pnpm", "yarn", "bun", "npx", "bunx", "pnpx", "deno"}
INSTALL_SUB = {"install", "i", "ci", "add", "remove", "uninstall", "rm", "update", "upgrade", "sync", "link",
               "unlink", "dedupe", "prune"}
NETWORK = {"curl", "wget", "gh", "scp", "sftp", "nc", "ncat", "dig", "nslookup", "host", "ping",
           "traceroute", "http", "https", "xh", "httpie", "aria2c", "telnet", "openssl", "mosh", "tailscale",
           "fly", "flyctl", "wrangler", "vercel", "gcloud", "aws", "az", "kubectl", "lynx", "w3m", "yt-dlp",
           "ffmpeg-net"}
PROC = {"ps", "kill", "pkill", "killall", "lsof", "pgrep", "sleep", "nohup", "jobs", "wait", "top", "htop",
        "launchctl", "tmux", "screen", "disown", "fg", "bg", "timeout", "gtimeout", "caffeinate", "watch",
        "systemctl", "service", "fuser", "netstat", "ss", "vmmap", "sample", "spindump", "log", "open",
        "osascript", "pbcopy", "pbpaste", "say", "afplay", "screencapture", "trap"}
SERVER_HINT = re.compile(r"(?:\bhttp\.server\b|\brunserver\b|\b(?:npm|pnpm|yarn|bun)\s+(?:run\s+)?(?:dev|start|serve|preview)\b|"
                         r"\bvite(?:\s+(?:dev|preview|serve))?\s*(?:$|&|--port|--host)|--watch\b|\bnodemon\b|\blive-server\b|"
                         r"\buvicorn\b|\bgunicorn\b|\bflask\s+run\b|\bnext\s+dev\b|\bwrangler\s+dev\b|\bcargo\s+watch\b|"
                         r"\bwatchexec\b|\bserve\s+-[lp]\b|\bnpx\s+serve\b|\bpython3?\s+-m\s+http)", re.M)


def classify_simple(words):
    """Classify one (already unwrapped) simple command -> set of categories."""
    name = norm_name(words[0])
    args = [unquote(a) for a in words[1:]]
    sub = subcommand(words)
    cats = set()
    if name in ("git",):
        if sub in GIT_MUT:
            cats.add("vcs-mutate")
            if sub in ("clone", "fetch", "pull", "push"):
                cats.add("network")
        elif sub == "stash" and (len(args) == 0 or args[0] in ("push", "pop", "apply", "drop", "save", "clear")):
            cats.add("vcs-mutate")
        elif sub == "branch" and any(a in ("-d", "-D", "-m", "-M", "--delete", "-f") for a in args):
            cats.add("vcs-mutate")
        elif sub == "tag" and any(not a.startswith("-") for a in args[1:]) and "-l" not in args and "--list" not in args:
            cats.add("vcs-mutate")
        elif sub == "worktree" and len(args) > 1 and args[1] in ("add", "remove", "prune", "move"):
            cats.add("vcs-mutate")
        elif sub == "config" and len([a for a in args if not a.startswith("-")]) >= 3:
            cats.add("vcs-mutate")
        else:
            cats.add("read-only")
        return cats
    if name == "jj":
        cats.add("vcs-mutate" if sub in ("new", "commit", "describe", "squash", "rebase", "edit", "abandon",
                                          "git", "bookmark", "split", "restore", "absorb") else "read-only")
        return cats
    if name == "gh":
        cats.add("network")
        if sub in ("pr", "issue", "release", "repo") and len(args) > 1 and args[1] in (
                "create", "merge", "close", "edit", "comment", "review", "delete", "reopen", "fork", "clone"):
            cats.add("vcs-mutate")
        return cats
    if name in ("sed",):
        if any(a == "-i" or a.startswith("-i") or a == "--in-place" or (a.startswith("-") and not a.startswith("--") and "i" in a[1:] and len(a) <= 4) for a in args):
            cats.add("file-mutate")
        else:
            cats.add("read-only")
        return cats
    if name in ("perl",):
        if any(re.match(r"-[a-zA-Z]*i", a) for a in args):
            cats.add("file-mutate")
        elif any(re.match(r"-[a-zA-Z]*[eE]", a) for a in args):
            cats.add("interp-inline")
        else:
            cats.add("script-run")
        return cats
    if name in ("tar", "unzip"):
        if name == "unzip" and ("-l" in args or "-p" in args or "-Z" in args):
            cats.add("read-only")
        elif name == "tar" and args and re.match(r"-?[a-zA-Z]*t", args[0]) and "x" not in args[0] and "c" not in args[0]:
            cats.add("read-only")
        else:
            cats.add("file-mutate")
        return cats
    if name in ("tail",) and any(a in ("-f", "-F", "--follow") or re.match(r"-\w*[fF]", a) for a in args):
        cats.add("long-running")
        return cats
    if name in JS_PM:
        if sub in INSTALL_SUB or (name in ("npx", "bunx", "pnpx")):
            if name in ("npx", "bunx", "pnpx"):
                cats.add("build-test")
            else:
                cats.add("pkg-install")
        elif name == "deno" and sub == "eval":
            cats.add("interp-inline")
        elif sub in ("view", "info", "ls", "list", "outdated", "why", "explain", "config", "root", "bin",
                     "--version", "-v", "search", "pack", "show", "audit", "query"):
            cats.add("read-only")
            if sub in ("view", "info", "search", "outdated", "audit"):
                cats.add("network")
        elif sub in ("publish", "login", "version", "dist-tag", "deprecate", "unpublish"):
            cats.add("vcs-mutate")
            cats.add("network")
        else:
            cats.add("build-test")
        return cats
    if name in ("pip", "pip3") or (name in ("python", "python3") and args[:2] == ["-m", "pip"]):
        cats.add("pkg-install" if "install" in args or "uninstall" in args else "read-only")
        return cats
    if name in ("uv", "uvx"):
        if sub in ("add", "sync", "remove", "lock") or (sub == "pip" and "install" in args) or (sub == "tool" and "install" in args) or sub == "venv":
            cats.add("pkg-install")
        elif sub == "run" or name == "uvx":
            # uv run python -c ... / uv run pytest
            rest = [a for a in args[1:] if not a.startswith("-")] if name == "uv" else args
            if rest and rest[0] in ("python", "python3") and ("-c" in args or "-" in args):
                cats.add("interp-inline")
            elif rest and rest[0] in BUILD | {"pytest"}:
                cats.add("build-test")
            else:
                cats.add("script-run")
        else:
            cats.add("build-test")
        return cats
    if name == "cargo":
        if sub in ("install", "add", "remove", "update", "fetch", "uninstall"):
            cats.add("pkg-install")
        elif sub in ("tree", "metadata", "search", "--version", "-V", "pkgid", "locate-project", "info"):
            cats.add("read-only")
        else:
            cats.add("build-test")
        return cats
    if name == "go":
        cats.add("pkg-install" if sub in ("get", "install", "mod") else "build-test")
        return cats
    if name in ("brew", "apt", "apt-get", "port", "gem", "rustup", "nvm", "pyenv", "asdf", "mise", "conda", "yum",
                "dnf", "pacman", "apk", "winget", "choco", "scoop", "dpkg", "rpm", "pipx", "sdkmanager", "nix-env"):
        if sub in ("install", "add", "upgrade", "update", "reinstall", "uninstall", "remove", "target", "toolchain",
                   "component", "use", "default", "link", "unlink", "tap", "cask", "-S", "sync"):
            cats.add("pkg-install")
        else:
            cats.add("read-only")
        return cats
    if name in INTERP:
        if name in ("python", "python3", "python3.12", "python3.13", "python3.14") or name.startswith("python"):
            if "-c" in args or "-" in args or not [a for a in args if not a.startswith("-")]:
                cats.add("interp-inline")
            elif args[:1] == ["-m"] and len(args) > 1:
                m = args[1]
                if m in ("pytest", "unittest", "mypy", "ruff", "build", "compileall", "py_compile", "black"):
                    cats.add("build-test")
                elif m in ("http.server",):
                    cats.add("long-running")
                elif m in ("json.tool", "site", "pydoc"):
                    cats.add("read-only")
                elif m in ("venv",):
                    cats.add("pkg-install")
                else:
                    cats.add("script-run")
            elif args and args[0] in ("--version", "-V"):
                cats.add("read-only")
            else:
                cats.add("script-run")
        elif name == "node":
            if any(a in ("-e", "-p", "--eval", "--print", "-") for a in args) or not [a for a in args if not a.startswith("-")]:
                cats.add("interp-inline")
            elif args and args[0] in ("--version", "-v"):
                cats.add("read-only")
            elif any(a in ("--test",) for a in args):
                cats.add("build-test")
            else:
                cats.add("script-run")
        elif name in ("ruby", "php", "lua", "osascript"):
            cats.add("interp-inline" if any(a in ("-e", "-r") for a in args) else "script-run")
        elif name in ("tsx", "ts-node"):
            cats.add("interp-inline" if "-e" in args else "script-run")
        elif name in ("bun",):
            cats.add("build-test")
        else:
            cats.add("script-run")
        return cats
    if name in SHELLS:
        if "-c" in args or any(re.fullmatch(r"-\w*c", a) for a in args):
            cats.add("shell-c")
        elif not args or args[0] in ("-s", "-"):
            cats.add("shell-c")
        elif args and args[0] in ("-n",):
            cats.add("read-only")
        else:
            cats.add("script-run")
        return cats
    if name in ("cd", "pushd", "popd"):
        cats.add("cd")
        return cats
    if name in ("echo", "printf", "true", ":", "false"):
        cats.add("echo")
        return cats
    if name in ("export", "set", "unset", "source", ".", "shopt", "local", "declare", "trap", "eval",
                "alias", "read", "exit", "return", "shift", "ulimit", "umask", "let", "readonly", "typeset"):
        cats.add("shell-builtin")
        return cats
    if name in MUTATE:
        if name == "rsync" and any(":" in a for a in args):
            cats.add("network")
        if name == "mktemp":
            cats.add("file-mutate")
            return cats
        cats.add("file-mutate")
        return cats
    if name in NETWORK:
        cats.add("network")
        return cats
    if name in PROC:
        cats.add("process")
        if name in ("watch",):
            cats.add("long-running")
        return cats
    if name in READ_ONLY:
        cats.add("read-only")
        return cats
    if name in BUILD or name == "pytest":
        cats.add("build-test")
        return cats
    if name == "<dynamic>":
        cats.add("dynamic")
        return cats
    if name in ("prlctl", "ssh", "docker", "podman", "utmctl", "VBoxManage", "multipass", "lima", "limactl"):
        if name in ("docker", "podman") and subcommand(words) not in ("exec", "run", "compose"):
            cats.add("build-test")
            return cats
        cats.add("remote-vm")
        return cats
    if name in ("psql", "mysql", "redis-cli", "mongosh", "clickhouse"):
        cats.add("network")
        return cats
    if name in ("gofmt", "golangci-lint", "samply", "wasm-objdump", "actionlint", "cargo-fuzz", "perf", "instruments",
                "xctrace", "dtrace", "pi"):
        cats.add("build-test")
        return cats
    if name.startswith("<path>"):
        cats.add("script-run")
        return cats
    if name in ("powershell", "pwsh", "cmd", "cmd.exe", "wsl", "powershell.exe"):
        cats.add("windows")
        return cats
    if name in ("vim", "vi", "nano", "emacs", "less", "more", "man", "top", "htop", "code", "subl", "open"):
        cats.add("interactive-or-gui")
        return cats
    cats.add("other")
    return cats


def word_has_unquoted_glob(node, src):
    # word nodes that contain * ? or [..] outside of quotes
    if node.type == "word":
        t = txt(node, src)
        return bool(re.search(r"(?<!\\)[*?]|\[[^\]]+\]", t)) and not t.startswith("-")
    return False


def analyze_one(row):
    global PARSER
    if PARSER is None:
        PARSER = ts.Parser(LANG)
    cmd = row["cmd"]
    src = cmd.encode("utf-8", "replace")
    tree = PARSER.parse(src)
    root = tree.root_node
    f = Counter()
    f["parse_error"] = int(root.has_error)
    f["lines"] = cmd.count("\n") + 1
    f["bytes"] = len(src)
    names, effective, subs, cats = [], [], [], set()
    flags = []  # (utility, flag-ish token)
    top_level_statements = [c for c in root.named_children if c.type != "comment"]
    if len(top_level_statements) > 1:
        f["multi_statement"] = 1
    for n in walk(root):
        t = n.type
        if t == "pipeline":
            f["pipe"] += 1
        elif t == "list":
            for c in n.children:
                if c.type == "&&":
                    f["and"] += 1
                elif c.type == "||":
                    f["or"] += 1
        elif t == ";":
            f["semicolon"] += 1
        elif t == "&" and n.parent is not None and n.parent.type in ("program", "compound_statement", "do_group", "list", "subshell", "redirected_statement"):
            f["background"] += 1
        elif t == "subshell":
            f["subshell"] += 1
        elif t == "compound_statement":
            f["brace_group"] += 1
        elif t == "command_substitution":
            if txt(n, src).startswith("`"):
                f["backtick"] += 1
            else:
                f["cmd_subst"] += 1
        elif t == "process_substitution":
            f["proc_subst"] += 1
        elif t == "heredoc_redirect":
            f["heredoc"] += 1
            if txt(n, src).startswith("<<-"):
                f["heredoc_dash"] += 1
            delim = n.child_by_field_name("delimiter") if hasattr(n, "child_by_field_name") else None
            dt = txt(delim, src) if delim is not None else ""
            if not dt:
                for c in n.children:
                    if c.type == "heredoc_start":
                        dt = txt(c, src)
            if dt and (dt[0] in "'\"" or "\\" in dt):
                f["heredoc_quoted"] += 1
            else:
                f["heredoc_unquoted"] += 1
        elif t == "herestring_redirect":
            f["herestring"] += 1
        elif t == "file_redirect":
            s = txt(n, src)
            dest = n.child_by_field_name("destination")
            d = txt(dest, src) if dest is not None else ""
            op = re.match(r"\s*(\d*)\s*([<>&|]+)", s)
            opstr = (op.group(1) + op.group(2)) if op else s[:3]
            if re.fullmatch(r"\d*>&\d*", opstr) or re.fullmatch(r"\d*>&", opstr):
                f["redir_fd_dup"] += 1  # 2>&1
            elif d in ("/dev/null",):
                f["redir_devnull"] += 1
                if opstr.startswith("2"):
                    f["redir_stderr_devnull"] += 1
            elif "<" in opstr:
                f["redir_in"] += 1
            elif ">>" in opstr:
                f["redir_append"] += 1
                f["redir_to_file"] += 1
            elif "&>" in opstr:
                f["redir_amp_out"] += 1
                f["redir_to_file"] += 1
            else:
                f["redir_out"] += 1
                f["redir_to_file"] += 1
            if d.startswith(("/tmp", "$TMPDIR", "${TMPDIR")) or "/tmp/" in d:
                f["redir_tmp"] += 1
        elif t == "for_statement":
            f["for"] += 1
        elif t == "c_style_for_statement":
            f["for_c"] += 1
        elif t == "while_statement":
            f["while"] += 1
            if txt(n, src).lstrip().startswith("until"):
                f["until"] += 1
        elif t == "if_statement":
            f["if"] += 1
        elif t == "case_statement":
            f["case"] += 1
        elif t == "function_definition":
            f["function"] += 1
        elif t == "array":
            f["array"] += 1
        elif t == "test_command":
            s = txt(n, src)
            if s.startswith("[["):
                f["dbl_bracket"] += 1
            elif s.startswith("(("):
                f["arith_cmd"] += 1
            else:
                f["single_bracket"] += 1
        elif t == "arithmetic_expansion":
            f["arith_expansion"] += 1
        elif t == "brace_expression":
            f["brace_expansion"] += 1
        elif t == "expansion":
            s = txt(n, src)
            f["param_expansion"] += 1
            if re.search(r"[:]?[-=?+]|#|%|/|\^|,|:\d", s[2:-1]) and not re.fullmatch(r"\$\{#?\w+\}", s):
                f["param_expansion_op"] += 1
            if re.match(r"\$\{\w+\[", s):
                f["array_ref"] += 1
        elif t == "simple_expansion":
            f["var_ref"] += 1
        elif t == "ansi_c_string":
            f["ansi_c_string"] += 1
        elif t == "raw_string":
            f["single_quoted"] += 1
        elif t == "string":
            f["double_quoted"] += 1
        elif t == "negated_command":
            f["negation"] += 1
        elif t == "declaration_command":
            f["declaration"] += 1
        elif t == "variable_assignment":
            if n.parent is not None and n.parent.type == "command":
                f["env_prefix"] += 1
            elif n.parent is not None and n.parent.type != "declaration_command":
                f["assignment"] += 1
        elif t == "word" and word_has_unquoted_glob(n, src) and n.parent is not None and n.parent.type in ("command", "concatenation", "for_statement"):
            # exclude arguments to find -name / rg -g etc? count as glob candidates; refined below
            f["glob_word"] += 1
        elif t == "comment":
            f["comment"] += 1
        elif t == "line_continuation" or (t == "\\" and False):
            f["line_continuation"] += 1

    if "\\\n" in cmd:
        f["line_continuation"] = cmd.count("\\\n")

    sc = simple_commands(root, src)
    first_name = None
    for words, node, prefix_assign in sc:
        chain = unwrap(words)
        for i, w in enumerate(chain):
            nm = norm_name(w[0])
            if i == 0:
                names.append(nm)
                if first_name is None:
                    first_name = nm
            effective.append(nm)
            sub = subcommand(w) if nm in SUBCMD_TOOLS else None
            if sub is not None:
                subs.append(f"{nm} {sub}")
            own = w[1:len(w) - len(chain[i + 1])] if i + 1 < len(chain) else w[1:]
            for a in own:
                a2 = unquote(a)
                if a2.startswith("-") and len(a2) < 40:
                    flags.append((nm, a2.split("=", 1)[0]))
            if nm in WRAPPERS and i == 0 and len(chain) > 1:
                continue  # classify the wrapped command, not the wrapper
            c = classify_simple(w)
            # glob arguments really used as globs (not patterns for find/rg/grep)
            cats |= c
        # set -e detection
        if norm_name(words[0]) == "set":
            a = " ".join(unquote(x) for x in words[1:])
            if re.search(r"-\w*e", a) or "errexit" in a:
                f["set_e"] += 1
            if "pipefail" in a:
                f["set_pipefail"] += 1
            if re.search(r"-\w*u", a) or "nounset" in a:
                f["set_u"] += 1
            if re.search(r"-\w*x", a):
                f["set_x"] += 1
    if first_name in ("cd", "pushd") and (f["and"] or f["semicolon"] or f["multi_statement"]):
        f["cd_prefix"] = 1
    if f["redir_to_file"]:
        # redirect into a file (not /dev/null) counts as a mutation
        cats.add("file-mutate")
    # heredoc fed to interpreter. tree-sitter attaches `cd x && python3 - <<'PY'` heredocs to the
    # whole list, so take the right-most command of the redirected body.
    for n in walk(root):
        if n.type != "redirected_statement" or not any(c.type == "heredoc_redirect" for c in n.children):
            continue
        body = n.child_by_field_name("body")
        while body is not None and body.type in ("list", "pipeline", "negated_command"):
            body = body.named_children[-1] if body.named_children else None
        if body is None or body.type != "command":
            continue
        nn = body.child_by_field_name("name")
        if nn is None:
            continue
        ws = [txt(nn, src)] + [txt(c, src) for c in body.children_by_field_name("argument")]
        nm = norm_name(unwrap(ws)[-1][0])
        if nm in INTERP or nm.startswith("python") or nm in SHELLS or nm in ("uv", "uvx", "npx", "bun"):
            f["heredoc_to_interp"] += 1
            cats.add("interp-inline")
        elif nm == "cat":
            f["heredoc_to_cat"] += 1
        else:
            f["heredoc_to_other"] += 1
    if not sc and not cats:
        cats.add("empty-or-assign")
    if SERVER_HINT.search(cmd):
        f["server_hint"] = 1
    m = re.findall(r"\bsleep\s+([\d.]+)", cmd)
    sleep = max((float(x) for x in m), default=None)
    out = {
        "src": row["src"], "ts": row.get("ts"), "names": names, "eff": effective, "subs": subs, "cats": sorted(cats),
        "feat": dict(f), "flags": flags, "sleep": sleep, "first": first_name,
        "is_error": row.get("is_error"), "exit_code": row.get("exit_code"), "timed_out": row.get("timed_out"),
        "aborted": row.get("aborted"), "not_found": row.get("not_found"), "has_result": row.get("has_result"),
        "nested_status": row.get("nested_status"), "timeout_arg": row.get("timeout_arg"),
        "dur_ms": row.get("dur_ms"), "cwd": row.get("cwd"),
    }
    return out


def main():
    rows = [json.loads(l) for l in open(os.path.join(DATA, "commands.jsonl"))]
    with Pool(16) as pool:
        feats = pool.map(analyze_one, rows, chunksize=500)
    with open(os.path.join(DATA, "features.jsonl"), "w") as fh:
        for r, ft in zip(rows, feats):
            ft["cid"] = r["cid"]
            fh.write(json.dumps(ft) + "\n")
    print("parsed", len(feats), "parse errors", sum(f["feat"]["parse_error"] for f in feats))


if __name__ == "__main__":
    main()
