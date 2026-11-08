// The live examples in psh.wasm: runs every ```sh live block of the guide the way the
// site's live.js does (a fresh shell over js/psh.mjs's MemHost, in an empty /work, with
// the environment crates/psh-docs/src/run.rs uses) and compares stdout and stderr, as
// they interleave, and the exit status with the block's gen:run region, which psh-docs
// made natively. A difference means the page would show one output before Run and
// another after it.
//
//   node crates/psh-docs/tools/live-check.mjs [psh.wasm]   (default target/wasm/psh.wasm)
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../..", import.meta.url));
const { loadPsh, MemHost } = await import(`${root}/js/psh.mjs`);
const wasm = await WebAssembly.compile(await readFile(process.argv[2] ?? `${root}/target/wasm/psh.wasm`));
const ENV = { HOME: "/work", PATH: "/usr/bin:/bin" };
const LIVE = /^```(?:sh|bash) [^\n]*\blive\b[^\n]*\n([\s\S]*?)^```\n<!-- gen:run -->\n(`{3,})text\n([\s\S]*?)^\2\n<!-- \/gen -->/gm;

let n = 0;
let bad = 0;
// The guide's pages: those SUMMARY.md links, and the front page.
const summary = await readFile(`${root}/docs/guide/SUMMARY.md`, "utf8");
const pages = [...summary.matchAll(/\]\(([^)]+\.md)\)/g)].map((m) => m[1]).concat("index.md");
for (const f of pages) {
  const md = await readFile(`${root}/docs/guide/${f}`, "utf8");
  for (const m of md.matchAll(LIVE)) {
    n++;
    const host = new MemHost();
    host.fs.mkdir("/work");
    const psh = await loadPsh(wasm, { host });
    const chunks = [];
    const shell = psh.createShell({ cwd: "/work", env: ENV, onStdout: (b) => chunks.push(b), onStderr: (b) => chunks.push(b) });
    const status = await shell.run(m[1], { signal: AbortSignal.timeout(5000) });
    shell.free();
    let out = new TextDecoder().decode(Buffer.concat(chunks));
    if (status !== 0) out += `${out && !out.endsWith("\n") ? "\n" : ""}(exit status ${status})\n`;
    if (out !== m[3]) {
      bad++;
      console.error(`live-check: docs/guide/${f}: psh.wasm prints\n${out}but the build recorded\n${m[3]}`);
    }
  }
}
if (bad) {
  console.error(`live-check: ${bad} of ${n} live examples differ in psh.wasm`);
  process.exit(1);
}
console.log(`live-check: ${n} live examples agree`);
