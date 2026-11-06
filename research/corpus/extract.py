#!/usr/bin/env python3
"""Extract every shell command agents ran from the Pi session corpora.

Sources:
  agent           ~/.pi/agent/sessions/**/*.jsonl  direct `bash` tool calls
  agent-codemode  same files, `bash` calls recorded inside `codemode` results
  p2              ~/.pi/p2/sessions/*.sqlite       direct `bash` tool calls
  p2-codemode     same DBs, nested `bash` calls recorded in codemode results
  *-static        codemode scripts without recorded nested calls: command
                  string literals pulled statically from `tools.bash({command})`

Output (PRIVATE, contains raw commands and output snippets; never commit):
  $PSH_CORPUS_DATA (default ~/.cache/psh-corpus)/commands.jsonl
"""
import glob
import json
import os
import re
import sqlite3
import sys
from multiprocessing import Pool

HOME = os.path.expanduser("~")
AGENT_DIR = os.path.join(HOME, ".pi/agent/sessions")
P2_DIR = os.path.join(HOME, ".pi/p2/sessions")
OUT_DIR = os.environ.get("PSH_CORPUS_DATA", os.path.join(HOME, ".cache/psh-corpus"))

EXIT_RE = re.compile(r"Command exited with code (-?\d+)")
TIMEOUT_RE = re.compile(r"Command timed out after (\d+) seconds")
ABORT_RE = re.compile(r"Command aborted")
NOTFOUND_RE = re.compile(r"(?:^|\n|: )([^\s:'\"`]{1,60}): (?:command not found)")
NOTFOUND2_RE = re.compile(r"command not found: ([^\s'\"`]{1,60})")  # zsh style


def result_text(content):
    if isinstance(content, str):
        return content
    out = []
    for c in content or []:
        if isinstance(c, dict) and c.get("type") == "text":
            out.append(c.get("text") or "")
    return "\n".join(out)


def analyze_result(text, is_error):
    r = {"is_error": bool(is_error)}
    tail = text[-600:]
    m = None
    for m in EXIT_RE.finditer(tail):
        pass
    r["exit_code"] = int(m.group(1)) if m else (None if is_error else 0)
    t = TIMEOUT_RE.search(tail)
    r["timed_out"] = bool(t)
    r["aborted"] = bool(ABORT_RE.search(tail))
    nf = set(NOTFOUND_RE.findall(text)) | set(NOTFOUND2_RE.findall(text))
    r["not_found"] = sorted(nf)[:5]
    # private snippet for failure classification; never published raw
    r["out_head"] = text[:400]
    r["out_tail"] = text[-500:] if len(text) > 400 else ""
    r["out_len"] = len(text)
    return r


# --------------------------------------------------------------- static JS
STR_RE = r"""(?P<q>['"`])(?P<s>(?:\\.|(?!(?P=q)).)*?)(?P=q)"""
BASH_CALL_RE = re.compile(r"tools\s*\.\s*bash\s*\(\s*\{(?P<body>.{0,6000}?)\}\s*\)", re.S)
CMD_FIELD_RE = re.compile(r"\bcommand\s*:\s*" + STR_RE, re.S)
CMD_VAR_RE = re.compile(r"\bcommand\s*(?:[,}]|:\s*[A-Za-z_$])")


def unescape_js(s, q):
    if q == "`":
        s = re.sub(r"\$\{[^}]*\}", "__X__", s)
    esc = {"n": "\n", "t": "\t", "r": "\r", "0": "\0"}
    out, i = [], 0
    while i < len(s):
        ch = s[i]
        if ch == "\\" and i + 1 < len(s):
            nx = s[i + 1]
            out.append(esc.get(nx, nx))
            i += 2
        else:
            out.append(ch)
            i += 1
    return "".join(out)


def static_bash_commands(code):
    cmds, unresolved = [], 0
    for m in BASH_CALL_RE.finditer(code or ""):
        body = m.group("body")
        f = CMD_FIELD_RE.search(body)
        if f:
            cmds.append(unescape_js(f.group("s"), f.group("q")))
        else:
            unresolved += 1
    # bash(...) via helper aliases like `const sh = (c) => tools.bash({command: c})`
    return cmds, unresolved


def parse_args(a):
    if isinstance(a, str):
        try:
            return json.loads(a)
        except Exception:
            return {}
    return a or {}


# --------------------------------------------------------------- agent
def scan_agent(path):
    rows, calls, results, codemode_static = [], {}, {}, []
    cwd, sid = None, os.path.basename(path)
    stats = {"codemode_calls": 0, "codemode_static_unresolved": 0}
    try:
        fh = open(path, "r", encoding="utf-8", errors="replace")
    except OSError:
        return rows, stats
    with fh:
        for line in fh:
            if '"type":"session"' in line[:40]:
                try:
                    d = json.loads(line)
                    cwd = d.get("cwd")
                    sid = d.get("id", sid)
                except Exception:
                    pass
                continue
            if '"toolCall"' not in line and '"toolResult"' not in line:
                continue
            if '"bash"' not in line and '"codemode"' not in line:
                continue
            try:
                d = json.loads(line)
            except Exception:
                continue
            m = d.get("message") or {}
            ts = m.get("timestamp")
            if m.get("role") == "assistant":
                for c in m.get("content") or []:
                    if isinstance(c, dict) and c.get("type") == "toolCall" and c.get("name") in ("bash", "codemode"):
                        calls[c.get("id")] = (c.get("name"), parse_args(c.get("arguments")), ts)
            elif m.get("role") == "toolResult" and m.get("toolName") in ("bash", "codemode"):
                results[m.get("toolCallId")] = m
    for cid, (name, args, ts) in calls.items():
        res = results.get(cid)
        if name == "bash":
            cmd = args.get("command")
            if not isinstance(cmd, str):
                continue
            row = {"src": "agent", "sid": sid, "cid": cid, "ts": ts, "cwd": cwd, "cmd": cmd,
                   "timeout_arg": args.get("timeout")}
            if res is not None:
                row.update(analyze_result(result_text(res.get("content")), res.get("isError")))
                row["has_result"] = True
                row["dur_ms"] = res.get("durationMs")
            else:
                row["has_result"] = False
            rows.append(row)
        else:
            stats["codemode_calls"] += 1
            nested = None
            if res is not None:
                nc = res.get("nestedCalls") or {}
                det = res.get("details") or {}
                nested = nc.get("calls") if nc.get("calls") is not None else det.get("calls")
            if nested is not None:
                for i, n in enumerate(nested):
                    if n.get("name") != "bash":
                        continue
                    a = n.get("arguments") if "arguments" in n else parse_args(n.get("args"))
                    cmd = (a or {}).get("command")
                    if isinstance(cmd, str):
                        rows.append({"src": "agent-codemode", "sid": sid, "cid": f"{cid}/{i}", "ts": ts, "cwd": cwd,
                                     "cmd": cmd, "timeout_arg": (a or {}).get("timeout"), "has_result": False,
                                     "nested_status": n.get("status"), "nested_ms": n.get("durationMs")})
            else:
                cmds, unres = static_bash_commands(args.get("code"))
                stats["codemode_static_unresolved"] += unres
                for i, cmd in enumerate(cmds):
                    rows.append({"src": "agent-codemode-static", "sid": sid, "cid": f"{cid}/s{i}", "ts": ts,
                                 "cwd": cwd, "cmd": cmd, "has_result": False})
    return rows, stats


# --------------------------------------------------------------- p2
def scan_p2(path):
    rows = []
    stats = {"codemode_calls": 0, "codemode_static_unresolved": 0}
    sid = os.path.basename(path).split(".")[0]
    try:
        db = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
        cur = db.execute(
            "select conversation_id, record from entries where record like '%bash%' or record like '%codemode%' order by id")
    except Exception as e:
        print("p2 open failed", path, e, file=sys.stderr)
        return rows, stats
    calls, results, cwds = {}, {}, {}
    for conv, rec in cur:
        try:
            d = json.loads(rec)
        except Exception:
            continue
        for m in d.get("model") or []:
            if not isinstance(m, dict):
                continue
            if m.get("role") == "system":
                c = (m.get("sections") or {}).get("cwd") or ""
                mm = re.search(r"<cwd>\s*(.*?)\s*</cwd>", c, re.S)
                if mm:
                    cwds[conv] = mm.group(1)
            elif m.get("role") == "assistant":
                for c in m.get("content") or []:
                    if isinstance(c, dict) and c.get("type") == "toolCall" and c.get("name") in ("bash", "codemode"):
                        calls[(conv, c.get("id"))] = (c.get("name"), parse_args(c.get("arguments")), m.get("timestamp"))
            elif m.get("role") == "toolResult" and m.get("toolName") in ("bash", "codemode"):
                results[(conv, m.get("toolCallId"))] = m
    for (conv, cid), (name, args, ts) in calls.items():
        res = results.get((conv, cid))
        if ts is None and res is not None:
            ts = res.get("timestamp")
        cwd = cwds.get(conv)
        if name == "bash":
            cmd = args.get("command")
            if not isinstance(cmd, str):
                continue
            row = {"src": "p2", "sid": f"{sid}:{conv}", "cid": cid, "ts": ts, "cwd": cwd, "cmd": cmd,
                   "timeout_arg": args.get("timeout")}
            if res is not None:
                row.update(analyze_result(result_text(res.get("content")), res.get("isError")))
                row["has_result"] = True
                row["dur_ms"] = res.get("durationMs")
            else:
                row["has_result"] = False
            rows.append(row)
        else:
            stats["codemode_calls"] += 1
            nested = None
            if res is not None:
                nested = (res.get("details") or {}).get("calls")
            if nested:
                for i, n in enumerate(nested):
                    if n.get("name") != "bash":
                        continue
                    a = n.get("arguments") if "arguments" in n else parse_args(n.get("args"))
                    cmd = (a or {}).get("command")
                    if isinstance(cmd, str):
                        rows.append({"src": "p2-codemode", "sid": f"{sid}:{conv}", "cid": f"{cid}/{i}", "ts": ts,
                                     "cwd": cwd, "cmd": cmd, "timeout_arg": (a or {}).get("timeout"),
                                     "has_result": False, "nested_status": n.get("status"),
                                     "nested_ms": n.get("durationMs")})
            else:
                cmds, unres = static_bash_commands(args.get("code"))
                stats["codemode_static_unresolved"] += unres
                for i, cmd in enumerate(cmds):
                    rows.append({"src": "p2-codemode-static", "sid": f"{sid}:{conv}", "cid": f"{cid}/s{i}",
                                 "ts": ts, "cwd": cwd, "cmd": cmd, "has_result": False})
    return rows, stats


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    agent_files = glob.glob(os.path.join(AGENT_DIR, "**/*.jsonl"), recursive=True)
    p2_files = sorted(glob.glob(os.path.join(P2_DIR, "*.sqlite")))
    seen = set()
    totals = {"agent_files": len(agent_files), "p2_files": len(p2_files), "dupes": 0,
              "codemode_calls": {"agent": 0, "p2": 0}, "codemode_static_unresolved": {"agent": 0, "p2": 0}}
    out = open(os.path.join(OUT_DIR, "commands.jsonl"), "w")
    with Pool(16) as pool:
        for kind, it in (("agent", pool.imap_unordered(scan_agent, agent_files, chunksize=8)),
                         ("p2", pool.imap_unordered(scan_p2, p2_files))):
            for rows, st in it:
                totals["codemode_calls"][kind] += st["codemode_calls"]
                totals["codemode_static_unresolved"][kind] += st["codemode_static_unresolved"]
                for r in rows:
                    key = (r["src"].split("-")[0], r["cid"], r["cmd"])
                    if key in seen:  # forked/copied sessions repeat entries
                        totals["dupes"] += 1
                        continue
                    seen.add(key)
                    out.write(json.dumps(r) + "\n")
    out.close()
    json.dump(totals, open(os.path.join(OUT_DIR, "extract_stats.json"), "w"), indent=1)
    print(json.dumps(totals))


if __name__ == "__main__":
    main()
