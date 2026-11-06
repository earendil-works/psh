#!/usr/bin/env python3
"""Render results.json into markdown tables (research/corpus/tables.md)."""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
R = json.load(open(os.path.join(HERE, "results.json")))
N = R["total"]
SRCS = ["agent", "agent-codemode", "p2", "p2-codemode"]
out = []
w = out.append


def pct(a, b):
    return f"{100.0 * a / b:.1f}%" if b else "–"


def table(head, rows):
    w("| " + " | ".join(head) + " |")
    w("|" + "|".join("---" for _ in head) + "|")
    for r in rows:
        w("| " + " | ".join(str(x) for x in r) + " |")
    w("")


w("## T1 Sources")
rows = []
for s in SRCS:
    lb = R["length_bytes"].get(s, {})
    rows.append([s, R["by_src"].get(s, 0), R["sessions"].get(s, 0), " → ".join(R["ts_range"].get(s, ["", ""])),
                 lb.get("p50"), lb.get("p90"), lb.get("p99")])
rows.append(["**total**", N, sum(R["sessions"].values()), "", "", "", ""])
table(["source", "commands", "sessions", "time range", "bytes p50", "p90", "p99"], rows)
w("Lines per command: " + ", ".join(f"{k}: {pct(v, N)}" for k, v in sorted(R["lines_hist"].items(), key=lambda x: ["1", "2-5", "6-20", "21-100", ">100"].index(x[0]))))
w("")

w("## T2 Top command names (first word of every simple command; % of commands containing it)")
rows = []
for i, (name, df, occ) in enumerate(R["top_names"][:90], 1):
    rows.append([i, f"`{name}`", df, pct(df, N), occ])
table(["#", "name", "commands", "% of cmds", "occurrences"], rows)

w("## T2b Top subcommands")
rows = [[f"`{k}`", v, pct(v, N)] for k, v in R["top_subs"][:60]]
table(["subcommand", "commands", "%"], rows)

w("## T3 Coverage: commands made up only of the top-N utilities")
rows = []
for n_ in ["5", "10", "20", "30", "50", "100", "200"]:
    c = R["coverage"][n_]
    rows.append([n_] + [f"{c.get(s, 0):.1f}%" for s in ["all", "agent", "p2", "agent-codemode"]])
table(["N", "all", "agent", "p2", "agent-codemode"], rows)
eff = [x for x, _ in R["eff_top"]]
w("Top-20 (by command frequency, wrappers unwrapped): " + ", ".join(f"`{x}`" for x in eff[:20]))
w("")
w("Top 21–50: " + ", ".join(f"`{x}`" for x in eff[20:50]))
w("")
w("## T3b Candidate in-process builtin set")
w(f"Builtin set ({len(R['builtin_set'])} names): " + " ".join(f"`{x}`" for x in R["builtin_set"]))
w("")
table(["metric", "all", "agent", "p2"], [
    ["only builtin-set utilities", f"{R['builtin_cov']['all']}%", f"{R['builtin_cov']['agent']}%", f"{R['builtin_cov']['p2']}%"],
    ["builtin set + at most one external program", f"{R['builtin_one_ext_cov']['all']}%", f"{R['builtin_one_ext_cov']['agent']}%", f"{R['builtin_one_ext_cov']['p2']}%"],
])
h = R["ext_count_hist"]
w("Distinct external (non-builtin) programs per command: " + ", ".join(f"{k}: {pct(h.get(k, 0), N)}" for k in ["0", "1", "2", "3+"]))
w("")
w("Most common external programs: " + ", ".join(f"`{k}` {pct(v, N)}" for k, v in R["ext_top"][:30]))
w("")

w("## T4 Categories (multi-label: % of commands touching the category)")
cats = sorted(R["cat_multi"]["all"].items(), key=lambda x: -x[1])
rows = []
for c, v in cats:
    rows.append([c, pct(v, N)] + [pct(R["cat_multi"].get(s, {}).get(c, 0), R["by_src"][s]) for s in ["agent", "p2"]]
                + [pct(R["cat_primary"]["all"].get(c, 0), N)])
table(["category", "all (multi)", "agent", "p2", "primary (exclusive)"], rows)
w(f"Pure read-only commands (only read-only utilities, cd, echo, builtins; no file redirect): "
  f"{pct(R['pure_read']['all'], N)} (agent {pct(R['pure_read']['agent'], R['by_src']['agent'])}, "
  f"p2 {pct(R['pure_read']['p2'], R['by_src']['p2'])})")
w("")

w("## T5 Shell syntax features (% of commands using it at least once)")
labels = dict((k, l) for k, l in R["feature_labels"])
rows = []
for k, l in R["feature_labels"]:
    a = R["features"]["all"].get(k, 0)
    rows.append([l, a, pct(a, N), pct(R["features"].get("agent", {}).get(k, 0), R["by_src"]["agent"]),
                 pct(R["features"].get("p2", {}).get(k, 0), R["by_src"]["p2"])])
rows.sort(key=lambda r: -r[1])
table(["feature", "commands", "all", "agent", "p2"], rows)

w("## T5b Output limiting / error suppression idioms")
table(["idiom", "commands", "%"], [[k, v, pct(v, N)] for k, v in sorted(R["outlim"].items(), key=lambda x: -x[1])])

w("## T5c bash-4+/zsh-specific constructs (matter for /bin/bash 3.2 and zsh)")
table(["construct", "commands"], [[k, v] for k, v in R["bash4"]])
w("(Regex on raw text: most `|&` hits are inside quoted strings/regexes; only ~20 such commands failed.)")
w("")

w("## T5d Parse validity")
sy = R["syntax"]
rows = []
for s in ["agent", "p2"]:
    d = sy.get(s, {})
    tot = d.get("bash_ok", 0) + d.get("bash_fail", 0)
    rows.append([s, tot, d.get("bash_fail", 0), pct(d.get("bash_fail", 0), tot), d.get("zsh_fail", 0), pct(d.get("zsh_fail", 0), tot)])
table(["source", "checked", "bash -n fail", "%", "zsh -n fail", "%"], rows)
w("tree-sitter-bash vs `bash -n`: " + ", ".join(f"{k}: {v}" for k, v in R["ts_vs_bash"].items()))
w("")

w("## T6 GNU/BSD-sensitive flag usage (commands; with result; failed)")
rows = sorted(([k, v[0], v[1], v[2], pct(v[2], v[1])] for k, v in R["compat"].items()), key=lambda r: -r[1])
table(["pattern", "commands", "with result", "failed", "fail %"], rows)

w("## T6b Top flags per utility")
rows = []
for u, fl in R["flags_by_util"].items():
    if fl:
        rows.append([f"`{u}`", " ".join(f"`{f}`×{c}" for f, c in fl[:14])])
table(["utility", "flags (count)"], rows)

w("## T6c Option errors seen in output (`X: illegal/invalid/unrecognized option`)")
table(["utility", "occurrences"], [[f"`{k}`", v] for k, v in R["opt_err"][:20]])

w("## T7 Failures")
fs = R["fail_src"]
table(["source", "with result", "failed", "rate"],
      [[s, fs[s]["with_result"], fs[s]["failed"], pct(fs[s]["failed"], fs[s]["with_result"])] for s in fs])
F = R["fails"]["failed"]
table(["failure kind", "count", "% of failures", "agent", "p2"],
      [[k, v, pct(v, F), R["fail_kinds_src"].get("agent", {}).get(k, 0), R["fail_kinds_src"].get("p2", {}).get(k, 0)]
       for k, v in R["fail_kinds"]])
w("Exit codes of failed commands: " + ", ".join(f"{k}: {v}" for k, v in R["exit_codes"][:15]))
w("")
w("### Errors hidden in successful output (message present but overall exit 0)")
table(["message", "commands with message", "of which exit 0"], [[k, v[0], f"{v[1]} ({pct(v[1], v[0])})"] for k, v in R["silent"].items()])
w("### `command not found` (per command, all outputs)")
table(["missing", "commands", "failed"], [[f"`{k}`", a, b] for k, a, b in R["not_found"][:25]])
w("### Failure rate by category")
table(["category", "with result", "failed", "rate"],
      sorted(([k, v[0], v[1], pct(v[1], v[0])] for k, v in R["fail_by_cat"].items() if v[0] > 100), key=lambda r: -r[1]))
w("### Failure rate by first command")
table(["first", "with result", "failed", "rate"], [[f"`{k}`", a, b, pct(b, a)] for k, a, b in R["fail_by_first"][:30]])
w("Timeouts by first command: " + ", ".join(f"`{k}` {v}" for k, v in R["timeouts_first"][:15]))
w("")
w("codemode nested bash status: " + ", ".join(f"{k}: {v}" for k, v in R["nested_status"].items()))
w("")

w("## T8 Long-running / background / interactive")
table(["pattern", "commands", "%", "agent", "p2"],
      sorted(([k, v, pct(v, N), R["long_running_src"].get("agent", {}).get(k, 0), R["long_running_src"].get("p2", {}).get(k, 0)]
              for k, v in R["long_running"].items()), key=lambda r: -r[1]))
w("sleep durations (max per command): " + ", ".join(f"{k}: {R['sleeps'].get(k, 0)}" for k in ["<=1s", "<=5s", "<=30s", "<=120s", ">120s"]))
w("")
w("Tool `timeout` argument set by the model: " + ", ".join(f"{k}: {pct(v, N)}" for k, v in R["timeout_args"].items()))
w("")
table(["source", "n with duration", "p50 ms", "p90 ms", "p99 ms", ">10s", ">60s"],
      [[s, d["n"], d["p50_ms"], d["p90_ms"], d["p99_ms"], f"{d['gt10s_pct']}%", f"{d['gt60s_pct']}%"] for s, d in R["durations"].items()])

w("## T9 Windows / remote")
table(["signal", "commands"], [[k, v] for k, v in sorted(R["windows"].items(), key=lambda x: -x[1])] +
      [[k, v] for k, v in R["win_fail"].items()] + [[k, v] for k, v in R["remote"].items()])

open(os.path.join(HERE, "tables.md"), "w").write("\n".join(out))
print("wrote tables.md")
