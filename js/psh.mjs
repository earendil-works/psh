// psh in JavaScript: the glue for the psh-wasm module (crates/psh-wasm) and a small
// host for it: an in-memory file system, commands written in JS, and the shell's step
// loop on the event loop. No Asyncify or JSPI: the module never waits; it asks this
// side for every operation (`host_call`) and is told the answers (`psh_complete`),
// at once or later.
//
//   import { loadPsh } from "./psh.mjs";
//   const psh = await loadPsh(await readFile("psh.wasm"));
//   const host = psh.host;                       // a MemHost
//   host.fs.writeFile("/work/a.txt", "hello\n");
//   host.command("upper", async (ctx) => {       // a host command
//     for await (const chunk of ctx.stdin) await ctx.stdout.write(chunk.toUpperCase?.() ?? chunk);
//     return 0;
//   });
//   const r = await psh.run("cat a.txt | upper", { cwd: "/work" });
//   // r.status, r.stdout, r.stderr (strings), r.stdoutBytes, r.stderrBytes
//
// The parser module (built without the shell) offers psh.parse() and psh.classify().

const ABI_VERSION = 1;

// host_call kinds (crates/psh-wasm/src/sys.rs).
const K = {
  OPEN: 1, STAT: 2, ACCESS: 3, READ_DIR: 4, CANONICALIZE: 5, RESOLVE: 6, SPAWN: 7,
  SLEEP: 8, READ: 9, WRITE: 10, CLOSE: 11, WAIT: 12, SIGNAL: 13, RELEASE: 14,
  ABANDON: 15, WAKE: 16, PANIC: 17, PID: 18,
};
// Open flags (crates/psh-wasm/src/host.rs).
const F = { READ: 1, WRITE: 2, APPEND: 4, CREATE: 8, TRUNCATE: 16, CREATE_NEW: 32 };
// Child descriptor kinds of a spawn spec.
const FD = { FILE: 0, PIPE_IN: 1, PIPE_OUT: 2, DUP: 3, NULL: 4 };
// Errno numbers the module understands (Linux numbering).
const E = {
  PERM: 1, NOENT: 2, BADF: 9, ACCES: 13, EXIST: 17, NOTDIR: 20, ISDIR: 21, INVAL: 22,
  NOSPC: 28, PIPE: 32, NAMETOOLONG: 36, NOTEMPTY: 39, LOOP: 40, NOTSUP: 95,
};
const CLOSED = 0xffffffff;

const enc = new TextEncoder();
const dec = new TextDecoder();
const bytes = (s) => (typeof s === "string" ? enc.encode(s) : s);

/** An error with an errno, for host operations. */
export class FsError extends Error {
  constructor(errno, message) {
    super(message ?? `errno ${errno}`);
    this.errno = errno;
  }
}

// --- the in-memory file system --------------------------------------------------------

let nextIno = 1;

function newNode(type, extra) {
  return { type, ino: nextIno++, mode: type === "dir" ? 0o755 : 0o644, mtime: Date.now(), ...extra };
}

function splitPath(path) {
  return path.split("/").filter((p) => p.length > 0);
}

/** An in-memory file system: directories, files and symlinks; paths are absolute. */
export { newNode };

export class MemFs {
  constructor() {
    this.root = newNode("dir", { children: new Map() });
  }

  /**
   * A directory node whose entries are made on first use by `load()` (a Map of name to
   * node): mounts of other storage, filled lazily. `node.loaded` tells whether it was.
   */
  static lazyDir(load, extra = {}) {
    const node = newNode("dir", extra);
    node.loaded = false;
    Object.defineProperty(node, "children", {
      configurable: true,
      enumerable: true,
      get() {
        const children = load();
        Object.defineProperty(node, "children", { value: children, writable: true, enumerable: true });
        node.loaded = true;
        return children;
      },
    });
    return node;
  }

  /** Puts `node` at `path` (its parent directories are made). */
  mount(path, node) {
    const parts = splitPath(path);
    if (parts.length === 0) throw new FsError(E.EXIST);
    this.mkdir("/" + parts.slice(0, -1).join("/"));
    const { dir, name } = this.parentOf(path);
    dir.children.set(name, node);
  }

  // Walks to `path`: the node, or throws. `follow`: resolve a final symlink.
  lookup(path, follow = true, depth = 0) {
    if (depth > 40) throw new FsError(E.LOOP);
    let node = this.root;
    const parts = splitPath(path);
    let prefix = "";
    for (let i = 0; i < parts.length; i++) {
      const name = parts[i];
      if (node.type !== "dir") throw new FsError(E.NOTDIR);
      if (name === ".") continue;
      if (name === "..") {
        // Physical parent: resolve the prefix again without its last part.
        const up = splitPath(prefix).slice(0, -1).join("/");
        prefix = "/" + up;
        node = this.lookup(prefix, true, depth + 1);
        continue;
      }
      let child = node.children.get(name);
      if (!child) throw new FsError(E.NOENT);
      const last = i === parts.length - 1;
      if (child.type === "symlink" && (!last || follow)) {
        const target = child.target.startsWith("/") ? child.target : `${prefix}/${child.target}`;
        const rest = parts.slice(i + 1).join("/");
        return this.lookup(rest ? `${target}/${rest}` : target, follow, depth + 1);
      }
      prefix = `${prefix}/${name}`;
      node = child;
    }
    return node;
  }

  // The physical path of an existing path.
  realpath(path, depth = 0) {
    if (depth > 40) throw new FsError(E.LOOP);
    const out = [];
    const parts = splitPath(path);
    let node = this.root;
    for (let i = 0; i < parts.length; i++) {
      const name = parts[i];
      if (node.type !== "dir") throw new FsError(E.NOTDIR);
      if (name === ".") continue;
      if (name === "..") {
        out.pop();
        node = this.lookup("/" + out.join("/"));
        continue;
      }
      const child = node.children.get(name);
      if (!child) throw new FsError(E.NOENT);
      if (child.type === "symlink") {
        const target = child.target.startsWith("/") ? child.target : `/${out.join("/")}/${child.target}`;
        const resolved = this.realpath(target, depth + 1);
        const rest = parts.slice(i + 1).join("/");
        return this.realpath(rest ? `${resolved}/${rest}` : resolved, depth + 1);
      }
      out.push(name);
      node = child;
    }
    return "/" + out.join("/");
  }

  parentOf(path) {
    const parts = splitPath(path);
    const name = parts.pop();
    if (name === undefined) throw new FsError(E.EXIST);
    const dir = this.lookup("/" + parts.join("/"));
    if (dir.type !== "dir") throw new FsError(E.NOTDIR);
    return { dir, name };
  }

  mkdir(path, { recursive = true } = {}) {
    let cur = "";
    for (const part of splitPath(path)) {
      cur += "/" + part;
      let node;
      try {
        node = this.lookup(cur);
      } catch (e) {
        if (!(e instanceof FsError) || e.errno !== E.NOENT) throw e;
        const { dir, name } = this.parentOf(cur);
        node = newNode("dir", { children: new Map() });
        dir.children.set(name, node);
        dir.mtime = Date.now();
        continue;
      }
      if (node.type !== "dir") throw new FsError(E.NOTDIR);
      if (!recursive && cur === path) throw new FsError(E.EXIST);
    }
  }

  writeFile(path, data, mode) {
    const parts = splitPath(path);
    if (parts.length > 1) this.mkdir("/" + parts.slice(0, -1).join("/"));
    const { dir, name } = this.parentOf(path);
    const node = newNode("file", { data: bytes(data) });
    if (mode !== undefined) node.mode = mode;
    dir.children.set(name, node);
    return node;
  }

  readFile(path) {
    const node = this.lookup(path);
    if (node.type !== "file") throw new FsError(E.ISDIR);
    return node.data;
  }

  symlink(target, path) {
    const { dir, name } = this.parentOf(path);
    if (dir.children.has(name)) throw new FsError(E.EXIST);
    dir.children.set(name, newNode("symlink", { target, mode: 0o777 }));
  }

  remove(path) {
    const { dir, name } = this.parentOf(path);
    const node = dir.children.get(name);
    if (!node) throw new FsError(E.NOENT);
    if (node.type === "dir" && node.children.size > 0) throw new FsError(E.NOTEMPTY);
    dir.children.delete(name);
  }

  /** Every path below `path` with its node, depth first (for snapshots). */
  *walk(path = "/") {
    const node = this.lookup(path, false);
    if (node.type !== "dir") return;
    for (const [name, child] of [...node.children].sort()) {
      const p = path === "/" ? `/${name}` : `${path}/${name}`;
      yield [p, child];
      if (child.type === "dir") yield* this.walk(p);
    }
  }
}

// --- handles: files, pipes, captures ----------------------------------------------------

class FileHandle {
  constructor(node, flags) {
    this.node = node;
    this.flags = flags;
    this.pos = 0;
  }
  read(max) {
    if (!(this.flags & F.READ)) throw new FsError(E.BADF);
    if (this.node.type === "dir") throw new FsError(E.ISDIR);
    const data = this.node.data.subarray(this.pos, this.pos + max);
    this.pos += data.length;
    return data;
  }
  write(data) {
    if (!(this.flags & F.WRITE)) throw new FsError(E.BADF);
    const node = this.node;
    if (this.flags & F.APPEND) this.pos = node.data.length;
    const end = this.pos + data.length;
    if (end > node.data.length) {
      const grown = new Uint8Array(end);
      grown.set(node.data);
      node.data = grown;
    }
    node.data.set(data, this.pos);
    this.pos = end;
    node.mtime = Date.now();
    return data.length;
  }
}

/** /dev/full: reads zeros, every write fails with ENOSPC. */
class FullHandle {
  read(max) {
    return new Uint8Array(max);
  }
  write() {
    throw new FsError(E.NOSPC);
  }
}

/** /dev/zero: reads zeros, writes vanish. */
class ZeroHandle {
  read(max) {
    return new Uint8Array(max);
  }
  write(data) {
    return data.length;
  }
}

/** /dev/urandom and /dev/random: random bytes, writes vanish. */
class RandomHandle {
  read(max) {
    const b = new Uint8Array(Math.min(max, 65536));
    globalThis.crypto.getRandomValues(b);
    return b;
  }
  write(data) {
    return data.length;
  }
}

class NullHandle {
  read() {
    return new Uint8Array(0);
  }
  write(data) {
    return data.length;
  }
}

/** Collects what is written: a shell's stdout and stderr. */
class Capture {
  constructor(onData) {
    this.chunks = [];
    this.onData = onData;
  }
  read() {
    throw new FsError(E.BADF);
  }
  write(data) {
    const copy = data.slice();
    this.chunks.push(copy);
    this.onData?.(copy);
    return data.length;
  }
  bytes() {
    const len = this.chunks.reduce((n, c) => n + c.length, 0);
    const out = new Uint8Array(len);
    let i = 0;
    for (const c of this.chunks) {
      out.set(c, i);
      i += c.length;
    }
    return out;
  }
}

/** Bytes to read once: a shell's stdin. */
class Source {
  constructor(data) {
    this.data = bytes(data ?? new Uint8Array(0));
    this.pos = 0;
  }
  read(max) {
    const d = this.data.subarray(this.pos, this.pos + max);
    this.pos += d.length;
    return d;
  }
  write() {
    throw new FsError(E.BADF);
  }
}

/** A pipe between the shell and a host command; unbounded. */
class Pipe {
  constructor() {
    this.chunks = [];
    this.readers = 0;
    this.writers = 0;
    this.waiting = []; // reads waiting for data: (data) => void
  }
  push(data) {
    if (this.readers === 0) throw new FsError(E.PIPE);
    if (data.length === 0) return 0;
    this.chunks.push(data.slice());
    this.flush();
    return data.length;
  }
  take(max) {
    const out = [];
    let n = 0;
    while (this.chunks.length && n < max) {
      const c = this.chunks[0];
      const k = Math.min(c.length, max - n);
      out.push(c.subarray(0, k));
      n += k;
      if (k === c.length) this.chunks.shift();
      else this.chunks[0] = c.subarray(k);
    }
    const buf = new Uint8Array(n);
    let i = 0;
    for (const c of out) {
      buf.set(c, i);
      i += c.length;
    }
    return buf;
  }
  // Answers waiting reads with data, or with EOF when no writer is left.
  flush() {
    while (this.waiting.length && (this.chunks.length || this.writers === 0)) {
      const { max, answer } = this.waiting.shift();
      answer(this.take(max));
    }
  }
}

class PipeEnd {
  constructor(pipe, reader) {
    this.pipe = pipe;
    this.reader = reader;
    if (reader) pipe.readers++;
    else pipe.writers++;
  }
  // Returns the data, or undefined when the read must wait (then `later` answers it).
  readAsync(max, later) {
    if (!this.reader) throw new FsError(E.BADF);
    const p = this.pipe;
    if (p.chunks.length || p.writers === 0) return p.take(max);
    p.waiting.push({ max, answer: later });
    return undefined;
  }
  write(data) {
    if (this.reader) throw new FsError(E.BADF);
    return this.pipe.push(data);
  }
  close() {
    if (this.reader) {
      this.pipe.readers--;
    } else {
      this.pipe.writers--;
      this.pipe.flush();
    }
  }
}

// --- the host ---------------------------------------------------------------------------

/** What a host command gets: its arguments, environment, stdio and the file system. */
class CommandContext {
  constructor(host, spec, stdio, abort) {
    this.host = host;
    this.fs = host.fs;
    this.argvBytes = spec.argv;
    this.argv = spec.argv.map((a) => dec.decode(a));
    this.args = this.argv.slice(1);
    this.env = Object.fromEntries(spec.env.map(([k, v]) => [dec.decode(k), dec.decode(v)]));
    this.cwd = dec.decode(spec.cwd);
    this.signal = abort.signal;
    this.stdin = new StreamReader(stdio[0]);
    this.stdout = new StreamWriter(stdio[1]);
    this.stderr = new StreamWriter(stdio[2]);
    this.fds = stdio;
  }
  /** A path relative to the command's cwd, made absolute. */
  path(p) {
    return p.startsWith("/") ? p : `${this.cwd}/${p}`;
  }
}

class StreamReader {
  constructor(handle) {
    this.handle = handle;
  }
  /** The next chunk, or null at end of file. */
  read(max = 65536) {
    const h = this.handle;
    if (!h) return Promise.resolve(null);
    if (h.readAsync) {
      return new Promise((resolve) => {
        const d = h.readAsync(max, (data) => resolve(data.length ? data : null));
        if (d !== undefined) resolve(d.length ? d : null);
      });
    }
    const d = h.read(max);
    return Promise.resolve(d.length ? d : null);
  }
  async bytes() {
    const parts = [];
    for (let c; (c = await this.read()) !== null; ) parts.push(c);
    const out = new Uint8Array(parts.reduce((n, c) => n + c.length, 0));
    let i = 0;
    for (const c of parts) {
      out.set(c, i);
      i += c.length;
    }
    return out;
  }
  async text() {
    return dec.decode(await this.bytes());
  }
  async *[Symbol.asyncIterator]() {
    for (let c; (c = await this.read()) !== null; ) yield c;
  }
}

class StreamWriter {
  constructor(handle) {
    this.handle = handle;
  }
  /** Writes a string or bytes; rejects with an FsError (EPIPE) when nobody reads. */
  async write(data) {
    if (!this.handle) throw new FsError(E.BADF);
    this.handle.write(bytes(data));
  }
}

/**
 * The default host: a MemFs, commands in JS (`command(name, fn)`), timers. Commands are
 * found under /usr/bin/NAME on any PATH lookup; their function gets a CommandContext
 * and returns the exit status (or a promise of it).
 */
export class MemHost {
  constructor({ fs = new MemFs(), pid = 4242 } = {}) {
    this.fs = fs;
    // The devices and directories every system has: /dev/null, /dev/zero and /dev/full
    // (character devices), /tmp (sticky, world-writable).
    fs.mkdir("/dev");
    for (const dev of ["null", "zero", "full", "random", "urandom"]) {
      if (!fs.root.children.get("dev").children.has(dev))
        fs.root.children.get("dev").children.set(dev, newNode("char", { device: dev, mode: 0o666 }));
    }
    fs.mkdir("/tmp");
    fs.lookup("/tmp").mode = 0o1777;
    for (const d of ["/bin", "/etc", "/home", "/usr/bin", "/usr/lib", "/var"]) fs.mkdir(d);
    this.pid = pid;
    this.commands = new Map();
    this.handles = new Map();
    this.nextHandle = 1;
    this.procs = new Map();
    this.nextPid = 5000;
    /** Names the shell looked for and did not find, since the last reset. */
    this.missing = [];
  }

  command(name, fn) {
    this.commands.set(name, fn);
    return this;
  }

  addHandle(h) {
    const id = this.nextHandle++;
    this.handles.set(id, h);
    return id;
  }

  // The operations. Each returns { err, value, data } or undefined (answered later
  // through `answer`).
  open(path, flags) {
    let node;
    try {
      node = this.fs.lookup(path);
      if (flags & F.CREATE_NEW) throw new FsError(E.EXIST);
    } catch (e) {
      if (!(e instanceof FsError) || e.errno !== E.NOENT || !(flags & F.CREATE)) throw e;
      // The parent must exist (no implicit mkdir, unlike MemFs.writeFile).
      const { dir, name } = this.fs.parentOf(path);
      node = newNode("file", { data: new Uint8Array(0) });
      dir.children.set(name, node);
      dir.mtime = Date.now();
    }
    if (node.type === "char") {
      const h = { full: FullHandle, zero: ZeroHandle, random: RandomHandle, urandom: RandomHandle }[node.device] ?? NullHandle;
      return { value: this.addHandle(new h()) };
    }
    if (node.type === "dir" && flags & F.WRITE) throw new FsError(E.ISDIR);
    if (flags & F.TRUNCATE && node.type === "file") node.data = new Uint8Array(0);
    return { value: this.addHandle(new FileHandle(node, flags)) };
  }

  stat(path, follow) {
    const n = this.fs.lookup(path, follow);
    const size = n.type === "file" ? n.data.length : n.type === "symlink" ? n.target.length : n.type === "dir" ? 4096 : 0;
    const text = `${n.type} ${n.mode} ${size} ${n.mtime * 1e6} 1 ${n.ino} 1000 1000`;
    return { data: enc.encode(text) };
  }

  access(path, mode) {
    const n = this.fs.lookup(path);
    const bit = [0o4, 0o2, 0o1][mode] << 6;
    return { value: n.mode & bit ? 1 : 0 };
  }

  readDir(path) {
    const n = this.fs.lookup(path);
    if (n.type !== "dir") throw new FsError(E.NOTDIR);
    const parts = [];
    for (const name of n.children.keys()) parts.push(enc.encode(name), new Uint8Array([0]));
    return { data: concat(parts) };
  }

  resolve(name) {
    if (this.commands.has(name)) return { data: enc.encode(`/usr/bin/${name}`) };
    this.missing.push(name);
    throw new FsError(E.NOENT);
  }

  spawn(spec) {
    const program = spec.program;
    const name = program.startsWith("/usr/bin/") ? program.slice(9) : null;
    const fn = name && this.commands.get(name);
    if (!fn) {
      this.missing.push(program);
      throw new FsError(E.NOENT);
    }
    // The child's descriptors, and the engine's ends of the new pipes.
    const stdio = [];
    const owned = []; // ends the command holds, closed when it ends
    const engineEnds = [];
    for (const { fd, kind, arg } of spec.fds) {
      let h;
      if (kind === FD.FILE) {
        const target = this.handles.get(arg);
        if (!target) throw new FsError(E.BADF);
        if (target instanceof PipeEnd) {
          h = new PipeEnd(target.pipe, target.reader);
          owned.push(h);
        } else h = target;
      } else if (kind === FD.PIPE_IN || kind === FD.PIPE_OUT) {
        const pipe = new Pipe();
        const childReads = kind === FD.PIPE_IN;
        h = new PipeEnd(pipe, childReads);
        owned.push(h);
        engineEnds.push(`${fd}:${this.addHandle(new PipeEnd(pipe, !childReads))}`);
      } else if (kind === FD.DUP) {
        h = stdio[arg];
      } else {
        h = new NullHandle();
      }
      stdio[fd] = h;
    }
    const pid = this.nextPid++;
    const abort = new AbortController();
    const proc = { status: undefined, waiters: [], abort, owned };
    this.procs.set(pid, proc);
    const ctx = new CommandContext(this, spec, stdio, abort);
    Promise.resolve()
      .then(() => fn(ctx))
      .then(
        (status) => this.exit(pid, typeof status === "number" ? status & 255 : 0),
        async (err) => {
          if (err instanceof FsError && err.errno === E.PIPE) return this.exit(pid, -13);
          try {
            await ctx.stderr.write(`${name}: ${err?.message ?? err}\n`);
          } catch {}
          this.exit(pid, 1);
        },
      );
    return { value: pid, data: enc.encode(engineEnds.join(" ")) };
  }

  exit(pid, status) {
    const proc = this.procs.get(pid);
    if (!proc || proc.status !== undefined) return;
    proc.status = status;
    for (const h of proc.owned) h.close();
    for (const w of proc.waiters) w(status);
    proc.waiters = [];
  }

  wait(pid, answer) {
    const proc = this.procs.get(pid);
    if (!proc) return { value: 0 };
    if (proc.status !== undefined) return { value: proc.status };
    proc.waiters.push((status) => answer({ value: status }));
    return undefined;
  }

  signal(pid, sig) {
    const proc = this.procs.get(pid);
    if (!proc || proc.status !== undefined) return;
    proc.abort.abort(sig);
    // A JS command cannot be stopped from outside: it counts as killed now.
    this.exit(pid, -sig);
  }

  close(handle) {
    const h = this.handles.get(handle);
    this.handles.delete(handle);
    h?.close?.();
  }

  read(handle, max, answer) {
    const h = this.handles.get(handle);
    if (!h) throw new FsError(E.BADF);
    if (h.readAsync) {
      const d = h.readAsync(max, (data) => answer({ data }));
      return d === undefined ? undefined : { data: d };
    }
    return { data: h.read(max) };
  }

  write(handle, data) {
    const h = this.handles.get(handle);
    if (!h) throw new FsError(E.BADF);
    return { value: h.write(data) };
  }
}

function concat(parts) {
  const out = new Uint8Array(parts.reduce((n, c) => n + c.length, 0));
  let i = 0;
  for (const c of parts) {
    out.set(c, i);
    i += c.length;
  }
  return out;
}

// --- the module -------------------------------------------------------------------------

class Reader {
  constructor(data) {
    this.data = data;
    this.view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    this.pos = 0;
  }
  u32() {
    const v = this.view.getUint32(this.pos, true);
    this.pos += 4;
    return v;
  }
  bytes() {
    const n = this.u32();
    const b = this.data.slice(this.pos, this.pos + n);
    this.pos += n;
    return b;
  }
}

function writer() {
  const parts = [];
  const w = {
    u32(v) {
      const b = new Uint8Array(4);
      new DataView(b.buffer).setUint32(0, v, true);
      parts.push(b);
      return w;
    },
    bytes(b) {
      b = bytes(b);
      w.u32(b.length);
      parts.push(b);
      return w;
    },
    done: () => concat(parts),
  };
  return w;
}

function decodeSpawn(data) {
  const r = new Reader(data);
  const program = dec.decode(r.bytes());
  const argv = [];
  for (let n = r.u32(); n > 0; n--) argv.push(r.bytes());
  const env = [];
  for (let n = r.u32(); n > 0; n--) env.push([r.bytes(), r.bytes()]);
  const cwd = r.bytes();
  const fds = [];
  for (let n = r.u32(); n > 0; n--) fds.push({ fd: r.u32(), kind: r.u32(), arg: r.u32() });
  return { program, argv, env, cwd, fds };
}

/** Loads the module (bytes, a Response, or a compiled WebAssembly.Module). */
export async function loadPsh(source, { host = new MemHost() } = {}) {
  const psh = new Psh(host);
  const imports = { psh: { host_call: (...a) => psh.hostCall(...a) } };
  let instance;
  if (source instanceof WebAssembly.Module) instance = await WebAssembly.instantiate(source, imports);
  else if (typeof Response !== "undefined" && source instanceof Response)
    ({ instance } = await WebAssembly.instantiateStreaming(source, imports));
  else ({ instance } = await WebAssembly.instantiate(source, imports));
  psh.attach(instance);
  return psh;
}

export class Psh {
  constructor(host) {
    this.host = host;
    this.shells = new Map(); // shell id -> { wake }
    this.panic = null;
  }

  attach(instance) {
    this.x = instance.exports;
    this.memory = this.x.memory;
    if (this.x.psh_abi_version() !== ABI_VERSION)
      throw new Error(`psh.wasm ABI ${this.x.psh_abi_version()}, glue ${ABI_VERSION}`);
    this.x.psh_init();
    this.hasShell = typeof this.x.psh_shell_new === "function";
  }

  mem() {
    return new Uint8Array(this.memory.buffer);
  }

  // Copies bytes into the module's input buffer; returns the length.
  input(data) {
    data = bytes(data);
    const ptr = this.x.psh_in(data.length) >>> 0;
    this.mem().set(data, ptr);
    return data.length;
  }

  output() {
    const ptr = this.x.psh_out_ptr() >>> 0;
    const len = this.x.psh_out_len() >>> 0;
    return this.mem().slice(ptr, ptr + len);
  }

  answer(op, { err = 0, value = 0, data } = {}) {
    const len = this.input(data ?? new Uint8Array(0));
    this.x.psh_complete(op, err, value, len);
  }

  /** `psh --parse` as a function: { ok, json }. */
  parse(source, { extglob = false, compact = true } = {}) {
    const ok = this.x.psh_parse(this.input(source), (extglob ? 1 : 0) | (compact ? 2 : 0));
    return { ok: ok === 1, json: JSON.parse(dec.decode(this.output())) };
  }

  /** `psh --classify` as a function: { ok, json }. */
  classify(source, { compact = true } = {}) {
    const ok = this.x.psh_classify(this.input(source), compact ? 2 : 0);
    return { ok: ok === 1, json: JSON.parse(dec.decode(this.output())) };
  }

  hostCall(kind, op, arg, ptr, len) {
    // wasm i32s arrive signed; these are u32s.
    op >>>= 0;
    arg >>>= 0;
    ptr >>>= 0;
    len >>>= 0;
    const h = this.host;
    const data = () => this.mem().slice(ptr, ptr + len);
    const later = (result) => this.answer(op, result);
    const run = (f) => {
      let r;
      try {
        r = f();
      } catch (e) {
        if (e instanceof FsError) r = { err: e.errno };
        else throw e;
      }
      if (r !== undefined) this.answer(op, r);
      return 0;
    };
    switch (kind) {
      case K.OPEN:
        return run(() => h.open(dec.decode(data()), arg));
      case K.STAT:
        return run(() => h.stat(dec.decode(data()), arg === 1));
      case K.ACCESS:
        return run(() => h.access(dec.decode(data()), arg));
      case K.READ_DIR:
        return run(() => h.readDir(dec.decode(data())));
      case K.CANONICALIZE:
        return run(() => ({ data: enc.encode(h.fs.realpath(dec.decode(data()))) }));
      case K.RESOLVE: {
        const r = new Reader(data());
        const name = dec.decode(r.bytes());
        return run(() => h.resolve(name, dec.decode(r.bytes()), dec.decode(r.bytes())));
      }
      case K.SPAWN:
        return run(() => h.spawn(decodeSpawn(data())));
      case K.SLEEP:
        // setTimeout takes at most 2^31-1 ms; longer sleeps never end in practice.
        if (arg < 0x7fffffff) setTimeout(() => this.answer(op, {}), arg);
        return 0;
      case K.READ:
        return run(() => h.read(arg, len, later));
      case K.WRITE:
        return run(() => h.write(arg, data()));
      case K.CLOSE:
        h.close(arg);
        return 0;
      case K.WAIT:
        return run(() => h.wait(arg, later));
      case K.SIGNAL:
        h.signal(arg, len);
        return 0;
      case K.RELEASE:
        return 0;
      case K.ABANDON:
        h.signal(arg, 15);
        return 0;
      case K.WAKE:
        this.shells.get(arg)?.wake();
        return 0;
      case K.PANIC:
        this.panic = dec.decode(data());
        return 0;
      case K.PID:
        return h.pid;
      default:
        throw new Error(`psh.wasm: unknown host_call kind ${kind}`);
    }
  }

  /**
   * Creates a shell. Options: cwd (default "/"), env (object), stdin (string or bytes),
   * onStdout/onStderr (called with each chunk), maxDepth.
   */
  createShell({ cwd = "/", env = {}, stdin, onStdout, onStderr, maxDepth = 0 } = {}) {
    if (!this.hasShell) throw new Error("this is the parser module: no shell");
    const out = new Capture(onStdout);
    const err = new Capture(onStderr);
    const ids = [
      stdin === undefined ? CLOSED : this.host.addHandle(new Source(stdin)),
      this.host.addHandle(out),
      this.host.addHandle(err),
    ];
    const w = writer().bytes(cwd);
    const pairs = Object.entries(env);
    w.u32(pairs.length);
    for (const [k, v] of pairs) w.bytes(k).bytes(String(v));
    for (const id of ids) w.u32(id);
    w.u32(maxDepth);
    const id = this.x.psh_shell_new(this.input(w.done()));
    if (id === 0) throw new Error("psh_shell_new: bad config");
    return new Shell(this, id, out, err);
  }

  /** Runs one command in a fresh shell: { status, stdout, stderr, stdoutBytes, stderrBytes }. */
  async run(source, opts = {}) {
    const shell = this.createShell(opts);
    try {
      const status = await shell.run(source, opts);
      return shell.result(status);
    } finally {
      shell.free();
    }
  }
}

export class Shell {
  constructor(psh, id, out, err) {
    this.psh = psh;
    this.id = id;
    this.out = out;
    this.err = err;
    this.woken = false;
    this.waiting = null;
    psh.shells.set(id, this);
  }

  wake() {
    this.woken = true;
    if (this.waiting) {
      const w = this.waiting;
      this.waiting = null;
      w();
    }
  }

  /**
   * Runs a script to its end: the exit status. Options: arg0 ("psh"), args ([]),
   * dashC (true), budget (safepoints per step), signal (an AbortSignal: cancels).
   */
  async run(source, { arg0 = "psh", args = [], dashC = true, budget = 10000, signal } = {}) {
    const psh = this.psh;
    const w = writer().bytes(source).bytes(arg0).u32(args.length);
    for (const a of args) w.bytes(a);
    const r = psh.x.psh_shell_start(this.id, psh.input(w.done()), dashC ? 1 : 0);
    if (r !== 0) throw new Error(r === 2 ? "a script is still running" : "psh_shell_start failed");
    signal?.addEventListener("abort", () => psh.x.psh_shell_cancel(this.id), { once: true });
    let steps = 0;
    for (;;) {
      this.woken = false;
      const s = psh.x.psh_shell_step(this.id, budget);
      if (psh.panic) throw new Error(`psh panicked: ${psh.panic}`);
      if (s >= 0) return s;
      if (s === -1) {
        // Blocked: wait for an answer to wake the shell (unless one already did).
        if (!this.woken) await new Promise((resolve) => (this.waiting = resolve));
      } else if (s === -2) {
        // Yielded: let the event loop run now and then.
        if (++steps % 16 === 0) await new Promise((resolve) => setTimeout(resolve, 0));
      } else {
        throw new Error(`psh_shell_step: ${s}`);
      }
    }
  }

  result(status) {
    const stdoutBytes = this.out.bytes();
    const stderrBytes = this.err.bytes();
    return { status, stdoutBytes, stderrBytes, stdout: dec.decode(stdoutBytes), stderr: dec.decode(stderrBytes) };
  }

  /** Clears what stdout and stderr collected. */
  clearOutput() {
    this.out.chunks = [];
    this.err.chunks = [];
  }

  free() {
    this.psh.x.psh_shell_free(this.id);
    this.psh.shells.delete(this.id);
  }
}
