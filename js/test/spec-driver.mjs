// The wasm engine of psh-spec (`psh-spec --wasm`): runs conformance cases in psh.wasm
// through js/psh.mjs's MemHost. psh-spec starts one driver per worker and sends cases
// over stdin; answers go to stdout. Frames are a u32 little-endian length, then the
// payload; payload fields are u32s and length-prefixed byte strings
// (crates/psh-spec/src/wasm.rs has the layouts).
//
// The case's working directory, its HOME and REPO_ROOT are mounted from the real file
// system, loaded lazily; after the run, what the case changed under its working
// directory is written back, so psh-spec snapshots it as for any other engine. Everything
// else (/tmp, ...) exists only in memory. There are no external programs: a command
// lookup that fails is reported back (`missing`), which tells psh-spec the case needed
// one.
//
//   node js/test/spec-driver.mjs PATH/TO/psh.wasm
import * as nodefs from "node:fs";
import { loadPsh, MemFs, MemHost, newNode } from "../psh.mjs";

const wasmPath = process.argv[2];
const wasm = nodefs.readFileSync(wasmPath);
// A trap (a panic, an overflow) leaves an instance unusable: the next case gets a new one.
let psh = await loadPsh(wasm);

// --- mounts of the real file system ---------------------------------------------------

function diskNode(path) {
  const st = nodefs.lstatSync(path);
  const mode = st.mode & 0o7777;
  if (st.isSymbolicLink()) return newNode("symlink", { target: nodefs.readlinkSync(path), mode });
  if (st.isDirectory()) return diskDir(path, mode);
  if (st.isFile()) return newNode("file", { data: new Uint8Array(nodefs.readFileSync(path)), mode });
  return newNode("other", { mode });
}

function diskDir(path, mode = 0o755) {
  const node = MemFs.lazyDir(() => {
    const children = new Map();
    for (const name of nodefs.readdirSync(path)) {
      try {
        children.set(name, diskNode(`${path}/${name}`));
      } catch {}
    }
    return children;
  });
  node.mode = mode;
  return node;
}

// Writes the memory tree of a mounted directory back to disk: only directories the case
// looked into can have changed.
function writeBack(node, path) {
  if (!node.loaded) return;
  const onDisk = new Set(nodefs.readdirSync(path));
  for (const name of onDisk) {
    if (!node.children.has(name)) nodefs.rmSync(`${path}/${name}`, { recursive: true, force: true });
  }
  for (const [name, child] of node.children) {
    const p = `${path}/${name}`;
    let st = null;
    try {
      st = nodefs.lstatSync(p);
    } catch {}
    if (child.type === "dir") {
      if (st && !st.isDirectory()) nodefs.rmSync(p, { force: true });
      if (!st || !st.isDirectory()) nodefs.mkdirSync(p);
      writeBack(child, p);
      nodefs.chmodSync(p, child.mode | 0o700);
    } else if (child.type === "file") {
      if (st && !st.isFile()) nodefs.rmSync(p, { recursive: true, force: true });
      nodefs.writeFileSync(p, child.data);
      nodefs.chmodSync(p, child.mode);
    } else if (child.type === "symlink") {
      if (st) nodefs.rmSync(p, { recursive: true, force: true });
      nodefs.symlinkSync(child.target, p);
    }
  }
}

// --- frames ------------------------------------------------------------------------------

class Reader {
  constructor(buf) {
    this.buf = buf;
    this.pos = 0;
  }
  u32() {
    const v = this.buf.readUInt32LE(this.pos);
    this.pos += 4;
    return v;
  }
  bytes() {
    const n = this.u32();
    const b = this.buf.subarray(this.pos, this.pos + n);
    this.pos += n;
    return b;
  }
  str() {
    return this.bytes().toString("utf8");
  }
}

function frame(parts) {
  const chunks = [];
  for (const p of parts) {
    if (typeof p === "number") {
      const b = Buffer.alloc(4);
      b.writeUInt32LE(p >>> 0);
      chunks.push(b);
    } else {
      const b = Buffer.from(p);
      const len = Buffer.alloc(4);
      len.writeUInt32LE(b.length);
      chunks.push(len, b);
    }
  }
  const body = Buffer.concat(chunks);
  const head = Buffer.alloc(4);
  head.writeUInt32LE(body.length);
  return Buffer.concat([head, body]);
}

async function runCase(req) {
  const r = new Reader(req);
  const source = r.bytes();
  const arg0 = r.str();
  const args = [];
  for (let n = r.u32(); n > 0; n--) args.push(r.bytes());
  const dashC = r.u32() === 1;
  const hasStdin = r.u32() === 1;
  const stdin = r.bytes();
  const env = {};
  for (let n = r.u32(); n > 0; n--) env[r.str()] = r.str();
  const cwd = r.str();
  const home = r.str();
  const repo = r.str();
  const timeoutMs = r.u32();

  const fs = new MemFs();
  fs.mkdir("/tmp");
  const cwdNode = diskDir(cwd);
  fs.mount(cwd, cwdNode);
  if (home && home !== cwd && !home.startsWith(`${cwd}/`)) fs.mount(home, diskDir(home));
  if (repo) fs.mount(repo, diskDir(repo));
  const host = new MemHost({ fs, pid: 99999999 });
  psh.host = host;

  const shell = psh.createShell({ cwd, env, stdin: hasStdin ? new Uint8Array(stdin) : undefined });
  let timedOut = false;
  let trapped = false;
  const ac = new AbortController();
  const timer = setTimeout(() => {
    timedOut = true;
    ac.abort();
  }, timeoutMs);
  let status;
  try {
    status = await shell.run(new Uint8Array(source), { arg0, args: args.map((a) => new Uint8Array(a)), dashC, signal: ac.signal });
  } catch (e) {
    status = 2;
    shell.err.write(new TextEncoder().encode(`psh-spec wasm driver: ${e.message}\n`));
    trapped = true;
  } finally {
    clearTimeout(timer);
  }
  const res = shell.result(status);
  if (trapped) psh = await loadPsh(wasm);
  else shell.free();
  writeBack(cwdNode, cwd);
  return frame([timedOut ? 1 : 0, status, res.stdoutBytes, res.stderrBytes, host.missing.join("\n")]);
}

// --- the loop ------------------------------------------------------------------------------

let pending = Buffer.alloc(0);
let busy = Promise.resolve();
process.stdin.on("data", (chunk) => {
  pending = Buffer.concat([pending, chunk]);
  while (pending.length >= 4) {
    const n = pending.readUInt32LE(0);
    if (pending.length < 4 + n) break;
    const req = pending.subarray(4, 4 + n);
    pending = pending.subarray(4 + n);
    busy = busy.then(async () => {
      const out = await runCase(req);
      await new Promise((resolve) => process.stdout.write(out, resolve));
    });
  }
});
process.stdin.on("end", () => busy.then(() => process.exit(0)));
