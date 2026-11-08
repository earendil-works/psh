// Exercise the actual editor module with a small DOM/host double. Deferred shell
// runs model sleeps without requiring a browser, WASM build, or real-time delays.
// node --experimental-vm-modules --test crates/psh-docs/tools/live-test.mjs
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import vm from "node:vm";
import { esc, highlight } from "../assets/highlight.js";

const source = await readFile(new URL("../assets/live.js", import.meta.url), "utf8");
const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

class Element {
  constructor() {
    this.innerHTML = "";
    this.textContent = "";
    this.hidden = false;
    this.attributes = new Map();
    this.listeners = new Map();
    this.children = new Map();
    this.dataset = {};
    const classes = new Set();
    this.classList = { add: (c) => classes.add(c), remove: (c) => classes.delete(c), contains: (c) => classes.has(c) };
  }
  setAttribute(k, v) { this.attributes.set(k, v); }
  addEventListener(k, fn) { this.listeners.set(k, fn); }
  querySelector(k) { assert.ok(this.children.has(k), `unexpected selector: ${k}`); return this.children.get(k); }
  after() {}
}

async function editor({ loading, parseResult } = {}) {
  const el = new Element();
  el.id = "example";
  const code = new Element();
  code.textContent = "echo original";
  const pre = new Element();
  pre.children.set("code", code);
  el.children.set(".live-editor pre", pre);
  for (const k of [".live-note", ".live-run", ".live-reset", ".live-status", '[data-pane="out"]', '[data-pane="classify"]', '[data-pane="ast"]'])
    el.children.set(k, new Element());
  const out = el.children.get('[data-pane="out"]');
  const classification = el.children.get('[data-pane="classify"]');
  const ast = el.children.get('[data-pane="ast"]');
  const status = el.children.get(".live-status");
  const tabs = ["out", "classify", "ast"].map((tab) => {
    const b = new Element();
    b.dataset.tab = tab;
    return b;
  });
  out.dataset.pane = "out";
  classification.dataset.pane = "classify";
  ast.dataset.pane = "ast";
  el.querySelectorAll = (k) => k === '[role="tab"]' ? tabs : [out, classification, ast];
  const timers = new Map();
  let timerId = 0, ta;
  const shells = [], classified = [], parsed = [];
  const glue = {
    MemHost: class { fs = { mkdir() {} }; },
    async loadPsh() {
      if (loading) await loading.promise;
      return {
        classify(src) {
          classified.push(src);
          return { json: { primary: src, read_only: false, truncated: false, sources: [], commands: [], reads: [], writes: [] } };
        },
        parse(src, { outline = false } = {}) {
          parsed.push(src);
          const span = [0, new TextEncoder().encode(src).length];
          if (parseResult) return { json: parseResult };
          if (outline) return { json: { ok: true, rows: [{ depth: 0, role: "", kind: "simple", info: "i<", text: src, span }, { depth: 1, role: "do", kind: "word", info: "", text: "w", span }] } };
          return { json: { ok: true, script: { type: "list", text: src, span } } };
        },
        createShell() {
          const done = deferred();
          const shell = { done, freed: false, run: () => done.promise, free() { this.freed = true; } };
          shells.push(shell);
          return shell;
        },
      };
    },
  };
  const document = {
    querySelectorAll: () => [el],
    createElement(tag) { const e = new Element(); if (tag === "textarea") ta = e; if (tag === "div") marks = e; return e; },
  };
  let marks;
  const context = vm.createContext({
    navigator: { platform: "Mac" }, location: { hash: "" }, URL,
    TextEncoder, TextDecoder, AbortController, performance, document,
    fetch: async () => ({}), WebAssembly: { compileStreaming: async () => ({}) },
    setTimeout(fn, ms) { const id = ++timerId; timers.set(id, { fn, ms }); return id; },
    clearTimeout(id) { timers.delete(id); },
  });
  const moduleOf = async (values) => {
    const m = new vm.SyntheticModule(Object.keys(values), function () {
      for (const [k, v] of Object.entries(values)) this.setExport(k, v);
    }, { context });
    await m.link(() => { throw Error("unexpected import"); });
    await m.evaluate();
    return m;
  };
  const module = new vm.SourceTextModule(source, {
    context,
    initializeImportMeta(meta) { meta.url = "https://example.test/live.js"; },
    importModuleDynamically: () => moduleOf(glue),
  });
  await module.link(() => moduleOf({ esc, highlight }));
  await module.evaluate();
  return {
    el, out, classification, ast, status, shells, classified, parsed, tabs, marks,
    /** A click on the AST pane whose target is inside `selectors` (selector: element). */
    async clickAst(targets) {
      ast.listeners.get("click")({ target: { closest: (k) => targets[k] ?? null } });
      await flush();
    },
    input(src) { ta.value = src; ta.listeners.get("input")(); },
    async run() { el.children.get(".live-run").onclick(); await flush(); },
    async autorun() {
      const [id, timer] = [...timers].find(([, t]) => t.ms === 600);
      timers.delete(id); timer.fn(); await flush();
    },
  };
}

test("classification is available while execution is blocked", async () => {
  const e = await editor();
  e.input("sleep 60");
  await e.autorun();
  assert.deepEqual(e.classified, ["sleep 60"]);
  assert.deepEqual(e.parsed, ["sleep 60"]);
  assert.match(e.ast.innerHTML, /sleep 60/);
  assert.match(e.classification.innerHTML, /sleep 60/);
  assert.equal(e.status.textContent, "running…");
  assert.equal(e.shells[0].freed, false);
  e.tabs[1].onclick();
  assert.equal(e.classification.hidden, false);
  assert.equal(e.out.hidden, true);
  e.tabs[2].onclick();
  assert.equal(e.ast.hidden, false);
  assert.equal(e.classification.hidden, true);
  assert.equal(e.out.hidden, true);
  e.shells[0].done.resolve(143);
  await flush();
  assert.match(e.status.textContent, /exit status 143/);
  assert.match(e.classification.innerHTML, /sleep 60/);
  assert.equal(e.shells[0].freed, true);
});

test("an edit invalidates old results before the autorun debounce fires", async () => {
  const e = await editor();
  e.input("sleep 60");
  await e.run();
  e.input("echo new");
  assert.match(e.classification.innerHTML, /pending/);
  e.shells[0].done.resolve(0);
  await flush();
  assert.equal(e.status.textContent, "edited");
  assert.match(e.ast.innerHTML, /AST pending/);
  assert.equal(e.out.innerHTML, "");
  assert.match(e.classification.innerHTML, /pending/);
  await e.autorun();
  assert.match(e.classification.innerHTML, /echo new/);
  assert.match(e.ast.innerHTML, /echo new/);
  e.shells[1].done.resolve(0);
  await flush();
});

test("an older run finishing cannot overwrite a newer run's results", async () => {
  const e = await editor();
  e.input("echo old");
  await e.run();
  e.input("echo new");
  await e.run();
  e.shells[1].done.resolve(0);
  await flush();
  const status = e.status.textContent;
  const output = e.out.innerHTML;
  e.shells[0].done.resolve(127);
  await flush();
  assert.equal(e.status.textContent, status);
  assert.equal(e.out.innerHTML, output);
  assert.match(e.classification.innerHTML, /echo new/);
  assert.match(e.ast.innerHTML, /echo new/);
  assert.doesNotMatch(e.ast.innerHTML, /echo old/);
  assert.ok(e.shells.every((s) => s.freed));
});

test("execution errors do not prevent classification", async () => {
  const e = await editor();
  e.input("echo new");
  await e.run();
  e.shells[0].done.reject(Error("execution failed"));
  await flush();
  assert.match(e.status.textContent, /could not run: execution failed/);
  assert.match(e.ast.innerHTML, /echo new/);
  assert.match(e.classification.innerHTML, /echo new/);
  assert.equal(e.shells[0].freed, true);
});

test("an edit during module loading cannot publish or execute obsolete input", async () => {
  const loading = deferred();
  const e = await editor({ loading });
  e.input("echo old");
  await e.run();
  e.input("echo new");
  loading.resolve();
  await flush();
  assert.deepEqual(e.classified, []);
  assert.deepEqual(e.parsed, []);
  assert.match(e.ast.innerHTML, /AST pending/);
  assert.equal(e.shells.length, 0);
  assert.match(e.classification.innerHTML, /pending/);
  await e.autorun();
  assert.deepEqual(e.classified, ["echo new"]);
  assert.deepEqual(e.parsed, ["echo new"]);
  e.shells[0].done.resolve(0);
  await flush();
});

test("AST is a tree of outline rows and escapes script markup", async () => {
  const e = await editor();
  const src = 'echo "<script>μ & \\"quoted\\"</script>"';
  e.input(src);
  await e.run();
  const html = e.ast.innerHTML;
  const n = new TextEncoder().encode(src).length;
  assert.ok(html.startsWith('<div class="ast-bar"><button data-view="tree" aria-pressed="true">Tree</button><button data-view="json" aria-pressed="false">JSON</button></div><div class="ast-tree">'), html);
  assert.ok(html.includes(`<div class="ast-row" data-depth="0" data-open="true" data-span="0 ${n}" style="--depth:0"><span class="ast-twisty"></span> <span class="ast-kind">simple</span> <span class="ast-info">i&lt;</span> <code class="ast-text">echo "&lt;script&gt;μ &amp;`), html);
  assert.ok(html.includes(`<div class="ast-row" data-depth="1" data-span="0 ${n}" style="--depth:1"><span class="ast-twisty"></span> <span class="ast-role">do</span> <span class="ast-kind">word</span> <code class="ast-text">w</code></div></div>`), html);
  assert.doesNotMatch(html, /<script>/);
  // Without a caret in the code, no span is marked.
  assert.equal(e.marks.innerHTML, "");
  e.shells[0].done.resolve(0);
  await flush();
});

test("the JSON view shows psh --parse, highlighted, and stays for later runs", async () => {
  const e = await editor();
  const src = 'echo "<script>μ"';
  e.input(src);
  await e.run();
  e.shells[0].done.resolve(0);
  await flush();
  await e.clickAst({ "button[data-view]": { dataset: { view: "json" } } });
  let html = e.ast.innerHTML;
  assert.match(html, /aria-pressed="true">JSON/);
  assert.match(html, /\n  <span class="tok-string">"ok"<\/span>: <span class="tok-keyword">true<\/span>/);
  assert.doesNotMatch(html, /<script>/);
  const plain = html.replace(/^.*?<pre class="ast-json">/s, "").replace(/<[^>]*>/g, "").replaceAll("&lt;", "<").replaceAll("&gt;", ">").replaceAll("&amp;", "&");
  assert.equal(JSON.parse(plain).script.text, src);
  e.input("echo new");
  await e.autorun();
  html = e.ast.innerHTML;
  assert.match(html, /aria-pressed="true">JSON/);
  assert.match(html, /echo new/);
  await e.clickAst({ "button[data-view]": { dataset: { view: "tree" } } });
  assert.match(e.ast.innerHTML, /class="ast-tree"/);
  e.shells[1].done.resolve(0);
  await flush();
});

test("the JSON view of the build's tree waits for an edit in flight", async () => {
  const e = await editor();
  e.input("echo pending");
  await e.clickAst({ "button[data-view]": { dataset: { view: "json" } } });
  assert.match(e.ast.innerHTML, /AST pending/);
  await e.autorun();
  assert.match(e.ast.innerHTML, /ast-json/);
  e.shells[0].done.resolve(0);
  await flush();
});

test("AST shows parse errors before execution finishes", async () => {
  const parseResult = { ok: false, error: { message: "unexpected end of file", span: [2, 2], line: 1, column: 3, incomplete: true } };
  const e = await editor({ parseResult });
  e.input("if");
  await e.run();
  assert.match(e.ast.innerHTML, /<p class="live-error">Does not parse: unexpected end of file \(line 1, column 3\)<\/p>/);
  assert.equal(e.status.textContent, "running…");
  e.shells[0].done.resolve(2);
  await flush();
});

test("keyboard navigation includes the AST tab and wraps", async () => {
  const e = await editor();
  let focused;
  e.tabs.forEach((t, i) => t.focus = () => { focused = i; });
  const key = (i, key) => e.tabs[i].onkeydown({ key, preventDefault() {} });
  key(1, "ArrowRight");
  assert.equal(focused, 2);
  assert.equal(e.ast.hidden, false);
  assert.equal(e.tabs[2].attributes.get("aria-selected"), "true");
  assert.equal(e.tabs[2].tabIndex, 0);
  assert.equal(e.tabs[1].tabIndex, -1);
  key(2, "ArrowRight");
  assert.equal(focused, 0);
  assert.equal(e.ast.hidden, true);
  key(0, "End");
  assert.equal(focused, 2);
  key(2, "Home");
  assert.equal(focused, 0);
});
