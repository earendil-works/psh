// Smoke tests of js/psh.mjs over target/wasm/{psh,psh-parser}.wasm (scripts/build-wasm):
//   node js/test/smoke.mjs
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { loadPsh, MemHost } from "../psh.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
let failures = 0;
function check(name, cond, detail) {
  if (!cond) {
    failures++;
    console.error(`FAIL ${name}${detail === undefined ? "" : `: ${JSON.stringify(detail)}`}`);
  }
}

const parser = await loadPsh(await readFile(`${root}target/wasm/psh-parser.wasm`));
{
  const r = parser.parse("echo hi | wc -l");
  check("parse ok", r.ok && r.json.ok === true, r);
  const bad = parser.parse("if then");
  check("parse error", !bad.ok && bad.json.ok === false, bad);
  const c = parser.classify("rm -rf build && ls");
  check("classify", c.ok && JSON.stringify(c.json).includes("rm"), c);
  check("no shell in the parser module", (() => {
    try {
      parser.createShell();
      return false;
    } catch {
      return true;
    }
  })());
}

const host = new MemHost();
const psh = await loadPsh(await readFile(`${root}target/wasm/psh.wasm`), { host });
host.fs.mkdir("/work");
host.fs.writeFile("/work/a.txt", "one\ntwo\nthree\n");
host.fs.symlink("a.txt", "/work/lnk");
host.command("upper", async (ctx) => {
  for await (const chunk of ctx.stdin) await ctx.stdout.write(new TextDecoder().decode(chunk).toUpperCase());
  return 0;
});
host.command("cat", async (ctx) => {
  if (ctx.args.length === 0) {
    for await (const chunk of ctx.stdin) await ctx.stdout.write(chunk);
    return 0;
  }
  for (const a of ctx.args) await ctx.stdout.write(ctx.fs.readFile(ctx.path(a)));
  return 0;
});
host.command("fail", async (ctx) => {
  await ctx.stderr.write(`fail: ${ctx.args.join(" ")}\n`);
  return 3;
});
host.command("slow", async (ctx) => {
  await new Promise((r) => setTimeout(r, 20));
  await ctx.stdout.write("slow done\n");
  return 0;
});

async function t(name, cmd, want, opts = {}) {
  const r = await psh.run(cmd, { cwd: "/work", ...opts });
  for (const [k, v] of Object.entries(want)) check(`${name}: ${k}`, r[k] === v, { got: r[k], want: v });
}

await t("echo", "echo hello world", { status: 0, stdout: "hello world\n", stderr: "" });
await t("status", "false; echo $?", { stdout: "1\n" });
await t("vars and loops", 'x=3; for i in a b; do echo "$i$x"; done', { stdout: "a3\nb3\n" });
await t("redirect and read back", 'echo data > out.txt; v=$(cat < out.txt); echo "[$v]"', { stdout: "[data]\n" });
check("file written", new TextDecoder().decode(host.fs.readFile("/work/out.txt")) === "data\n");
await t("a host command reads a file", "cat a.txt", { stdout: "one\ntwo\nthree\n" });
await t("append", "echo 1 >> log; echo 2 >> log; cat log", { stdout: "1\n2\n" });
await t("here-doc to a host command", "cat <<'EOF'\nx $y\nEOF", { stdout: "x $y\n" });
await t("stderr redirect", "fail a 2>&1 | upper", { stdout: "FAIL: A\n" });
await t("glob", "echo *.txt", { stdout: "a.txt out.txt\n" });
await t("host command in a pipeline", "echo abc | upper", { stdout: "ABC\n" });
await t("host command status", "fail x y; echo st=$?", { stdout: "st=3\n", stderr: "fail: x y\n" });
await t("command not found", "nope 2>&1; echo $?", { stdout: "psh: line 1: nope: command not found\n127\n" });
await t("async host command", "slow; echo after", { stdout: "slow done\nafter\n" });
await t("command substitution", 'v=$(upper <<< "q"); echo "$v"', { stdout: "Q\n" });
await t("stdin", "cat", { stdout: "input\n" }, { stdin: "input\n" });
await t("subshell and cd", "mkdir_not_needed=1; (cd /; pwd); pwd", { stdout: "/\n/work\n" });
await t("syntax error", "if then", { status: 2 });
await t("command-line options", 'echo "$-"; shopt -q extglob && echo extglob; case :$SHELLOPTS: in *:pipefail:*) echo pipefail;; esac', { stdout: "ehBcs\nextglob\npipefail\n" }, { options: ["-e", "-o", "pipefail", "-O", "extglob", "-s"] });
await t("errexit from the command line", "false; echo not", { status: 1, stdout: "" }, { options: ["-e"] });
await t("mkdir, touch, rmdir", "mkdir -p d/e && mkdir -m 700 m && touch -d @1700000000.123456789 d/f && rmdir d/e && echo ok", { stdout: "ok\n" });
check("mkdir mode", host.fs.lookup("/work/m").mode === 0o700, host.fs.lookup("/work/m").mode.toString(8));
check("touch -d exact ns", host.fs.lookup("/work/d/f").mtimeNs === 1700000000123456789n, String(host.fs.lookup("/work/d/f").mtimeNs));
check("rmdir", !host.fs.lookup("/work/d").children.has("e"));
await t("readlink of a symlink", "readlink lnk", { stdout: "a.txt\n" }, {});
await t("the clock", "printf '%(%s)T'", { stdout: String(Math.floor(Date.now() / 1000)) });
{
  const r = await psh.run("f() { f; }; f");
  check("deep recursion is an error, not a crash", r.status === 1 && r.stderr.includes("maximum function nesting level exceeded"), r.stderr);
}

if (failures) {
  console.error(`smoke: ${failures} failure(s)`);
  process.exit(1);
}
console.log("smoke: ok");
