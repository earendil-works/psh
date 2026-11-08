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

async function editor({ loading } = {}) {
  const el = new Element();
  el.id = "example";
  const code = new Element();
  code.textContent = "echo original";
  const pre = new Element();
  pre.children.set("code", code);
  el.children.set(".live-editor pre", pre);
  for (const k of [".live-note", ".live-run", ".live-reset", ".live-status", '[data-pane="out"]', '[data-pane="classify"]'])
    el.children.set(k, new Element());
  const out = el.children.get('[data-pane="out"]');
  const classification = el.children.get('[data-pane="classify"]');
  const status = el.children.get(".live-status");
  const tabs = ["out", "classify"].map((tab) => {
    const b = new Element();
    b.dataset.tab = tab;
    return b;
  });
  out.dataset.pane = "out";
  classification.dataset.pane = "classify";
  el.querySelectorAll = (k) => k === '[role="tab"]' ? tabs : [out, classification];
  const timers = new Map();
  let timerId = 0, ta;
  const shells = [], classified = [];
  const glue = {
    MemHost: class { fs = { mkdir() {} }; },
    async loadPsh() {
      if (loading) await loading.promise;
      return {
        classify(src) {
          classified.push(src);
          return { json: { primary: src, read_only: false, truncated: false, sources: [], commands: [], reads: [], writes: [] } };
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
  const context = vm.createContext({
    navigator: { platform: "Mac" }, location: { hash: "" }, URL,
    TextEncoder, TextDecoder, AbortController, performance,
    document: {
      querySelectorAll: () => [el],
      createElement(tag) { const e = new Element(); if (tag === "textarea") ta = e; return e; },
    },
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
    el, out, classification, status, shells, classified, tabs,
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
  assert.match(e.classification.innerHTML, /sleep 60/);
  assert.equal(e.status.textContent, "running…");
  assert.equal(e.shells[0].freed, false);
  e.tabs[1].onclick();
  assert.equal(e.classification.hidden, false);
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
  assert.equal(e.out.innerHTML, "");
  assert.match(e.classification.innerHTML, /pending/);
  await e.autorun();
  assert.match(e.classification.innerHTML, /echo new/);
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
  assert.ok(e.shells.every((s) => s.freed));
});

test("execution errors do not prevent classification", async () => {
  const e = await editor();
  e.input("echo new");
  await e.run();
  e.shells[0].done.reject(Error("execution failed"));
  await flush();
  assert.match(e.status.textContent, /could not run: execution failed/);
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
  assert.equal(e.shells.length, 0);
  assert.match(e.classification.innerHTML, /pending/);
  await e.autorun();
  assert.deepEqual(e.classified, ["echo new"]);
  e.shells[0].done.resolve(0);
  await flush();
});
