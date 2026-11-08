// Live examples: every ```sh live block of the guide is an editor whose code runs in
// psh.wasm in the page, in a fresh shell over js/psh.mjs's in-memory host, in an empty
// /work with the environment the build used (crates/psh-docs/src/run.rs). The editor is
// a textarea with transparent text over the highlighted `pre` the page already has
// (render.rs live_block): the textarea does the editing (caret, selection, undo, input
// methods, screen readers) and highlight.js repaints the `pre` under it on every input.
// Without JavaScript (or from file://) the page shows the code and the output and
// classification made at build time.
import { esc, highlight } from "./highlight.js";

const CWD = "/work";
const ENV = { HOME: "/work", PATH: "/usr/bin:/bin" };
// A runaway loop (`while :` typed halfway) is cancelled after this long.
const LIMIT_MS = 5000;
const INDENT = "  ";
const AUTORUN_MS = 600;
const MAC = /Mac|iPhone|iPad/.test(navigator.platform);
const RUN_KEYS = MAC ? "⌘↵" : "Ctrl+↵";

// The glue and the module load on first use: a page that is only read fetches neither.
let libPromise;
function lib() {
  libPromise ??= Promise.all([import("./psh/psh.mjs"), compile(new URL("./psh/psh.wasm", import.meta.url))]).then(
    ([glue, module]) => ({ ...glue, module }),
  );
  return libPromise;
}

async function compile(url) {
  try {
    return await WebAssembly.compileStreaming(fetch(url));
  } catch {
    // A server that does not send application/wasm.
    const r = await fetch(url);
    if (!r.ok) throw new Error(`${url.pathname}: ${r.status}`);
    return WebAssembly.compile(await r.arrayBuffer());
  }
}

const dec = () => new TextDecoder();

/** The Output pane: render.rs output_html. */
function outputHtml(chunks, status) {
  // Consecutive writes to one stream are one run of text, as the build records them.
  const runs = [];
  for (const [err, b] of chunks) {
    const last = runs[runs.length - 1];
    if (last && last[0] === err) last[1].push(b);
    else runs.push([err, [b]]);
  }
  let h = "";
  let nl = true;
  for (const [err, parts] of runs) {
    const n = parts.reduce((k, p) => k + p.length, 0);
    const all = new Uint8Array(n);
    let at = 0;
    for (const p of parts) {
      all.set(p, at);
      at += p.length;
    }
    const text = dec().decode(all);
    nl = text.endsWith("\n");
    const t = esc(text);
    h += err ? `<span class="live-stderr">${t}</span>` : t;
  }
  if (status !== 0) {
    if (!nl) h += "\n";
    h += `<span class="live-exit">(exit status ${status})</span>`;
  }
  return h || `<span class="muted">(no output)</span>`;
}

/** The Classification pane from `psh.classify`'s JSON: run.rs classification_html. */
function classificationHtml(src, c) {
  const yesNo = (b) => (b ? "yes" : "no");
  let h = `<p>Primary category <b>${esc(c.primary)}</b> · read-only: <b>${yesNo(c.read_only)}</b> · output cut short: <b>${yesNo(c.truncated)}</b></p>`;
  if (c.error) return h + `<p class="live-error">Does not parse: ${esc(c.error.message)}</p>`;
  const enc = new TextEncoder();
  const texts = [enc.encode(src), ...c.sources.map((s) => enc.encode(s.text))];
  // Spans are byte offsets into the script, or into `sources[i].text`.
  const textOf = (source, [a, b]) => dec().decode(texts[source === null ? 0 : source + 1].slice(a, b));
  if (c.commands.length) {
    h += `<table class="commands"><thead><tr><th>Command</th><th>Category</th><th>Runs</th></tr></thead><tbody>`;
    for (const x of c.commands) {
      h += `<tr><td><code>${esc(textOf(x.source, x.span))}</code></td><td>${esc(x.categories.join(", "))}</td><td>${esc(x.via)}</td></tr>`;
    }
    h += `</tbody></table>`;
  }
  const paths = (v) => v.map((p) => (p.path === null ? "<i>dynamic</i>" : `<code>${esc(p.path)}</code>`)).join(", ");
  if (c.reads.length) h += `<p>Reads ${paths(c.reads)}</p>`;
  if (c.writes.length) h += `<p>Writes ${paths(c.writes)}</p>`;
  return h;
}

// ---- Editing ----------------------------------------------------------------------

/** Replaces the selection with `text`, through the browser's editing so undo works. */
function insert(ta, text) {
  const ok = text === "" ? document.execCommand("delete") : document.execCommand("insertText", false, text);
  if (!ok) {
    ta.setRangeText(text, ta.selectionStart, ta.selectionEnd, "end");
    ta.dispatchEvent(new InputEvent("input", { inputType: "insertText", data: text }));
  }
}

/** Indents (or with `out`, dedents) the lines the selection touches. */
function indentLines(ta, out) {
  const v = ta.value;
  const [s, e] = [ta.selectionStart, ta.selectionEnd];
  const from = v.lastIndexOf("\n", s - 1) + 1;
  let to = v.indexOf("\n", e > s && v[e - 1] === "\n" ? e - 1 : e);
  if (to < 0) to = v.length;
  const lines = v.slice(from, to).split("\n");
  const changed = lines.map((l) => (out ? l.replace(/^( {1,2}|\t)/, "") : l === "" ? l : INDENT + l));
  const text = changed.join("\n");
  if (text === v.slice(from, to)) return;
  const firstDelta = changed[0].length - lines[0].length;
  ta.setSelectionRange(from, to);
  insert(ta, text);
  const ns = Math.max(from, s + firstDelta);
  ta.setSelectionRange(ns, s === e ? ns : e + text.length - (to - from));
}

function onKey(ev, ex) {
  const ta = ex.ta;
  if (ev.isComposing) return;
  const v = ta.value;
  const [s, e] = [ta.selectionStart, ta.selectionEnd];
  if (ev.key === "Enter" && (ev.metaKey || ev.ctrlKey)) {
    ev.preventDefault();
    run(ex);
  } else if (ev.key === "Escape") {
    // Tab indents in the editor; Escape gives the keyboard back to the page, on the
    // result tabs right after the editor.
    ex.el.querySelector('[role="tab"][aria-selected="true"]').focus();
  } else if (ev.key === "Tab" && !ev.altKey && !ev.ctrlKey && !ev.metaKey) {
    ev.preventDefault();
    if (ev.shiftKey || v.slice(s, e).includes("\n")) indentLines(ta, ev.shiftKey);
    else insert(ta, INDENT);
  } else if (ev.key === "Enter" && !ev.altKey) {
    // The new line starts at the indentation of this one, one step deeper after `do`,
    // `then`, `{` or `(`.
    ev.preventDefault();
    const from = v.lastIndexOf("\n", s - 1) + 1;
    const line = v.slice(from, s);
    const indent = /^[ \t]*/.exec(line)[0];
    const deeper = /(\bdo|\bthen|\belse|[{(])\s*$/.test(line);
    insert(ta, `\n${indent}${deeper ? INDENT : ""}`);
  } else if (ev.key === "Backspace" && s === e && s > 0 && !ev.altKey && !ev.metaKey && !ev.ctrlKey) {
    // In the indentation, Backspace removes one step of it.
    const from = v.lastIndexOf("\n", s - 1) + 1;
    const before = v.slice(from, s);
    if (before.length >= 2 && /^ +$/.test(before)) {
      ev.preventDefault();
      const n = before.length % 2 || 2;
      ta.setSelectionRange(s - n, s);
      insert(ta, "");
    }
  }
}

// ---- Running ----------------------------------------------------------------------

async function run(ex) {
  const { el, ta } = ex;
  clearTimeout(ex.timer);
  const id = ++ex.runs;
  const src = ta.value;
  const pane = (n) => el.querySelector(`[data-pane="${n}"]`);
  const setStatus = (t) => (el.querySelector(".live-status").textContent = t);
  setStatus("running…");
  el.classList.add("is-running");
  try {
    const { loadPsh, MemHost, module } = await lib();
    if (id !== ex.runs) return;
    const host = new MemHost();
    host.fs.mkdir(CWD);
    const psh = await loadPsh(module, { host });
    if (id !== ex.runs) return;
    // Classification needs only the text, not the outcome of a possibly slow run.
    pane("classify").innerHTML = classificationHtml(src, psh.classify(src).json);
    const chunks = [];
    const shell = psh.createShell({
      cwd: CWD,
      env: ENV,
      onStdout: (b) => chunks.push([false, b]),
      onStderr: (b) => chunks.push([true, b]),
    });
    const stop = new AbortController();
    const timer = setTimeout(() => stop.abort(), LIMIT_MS);
    const t0 = performance.now();
    let status;
    try {
      status = await shell.run(src, { signal: stop.signal });
    } finally {
      clearTimeout(timer);
      shell.free();
    }
    const ms = performance.now() - t0;
    // Edited again while this ran: the newer run fills the panes.
    if (id !== ex.runs) return;
    let out = outputHtml(chunks, status);
    if (stop.signal.aborted) out += `\n<span class="live-exit">(stopped after ${LIMIT_MS / 1000} s)</span>`;
    pane("out").innerHTML = out;
    setStatus(`exit status ${status} · ${ms.toFixed(1)} ms`);
  } catch (e) {
    if (id === ex.runs) setStatus(`could not run: ${e.message}`);
  } finally {
    if (id === ex.runs) el.classList.remove("is-running");
  }
}

// ---- Setup ------------------------------------------------------------------------

function repaint(ex) {
  const v = ex.ta.value;
  // A pre drops a final newline; the textarea shows the empty line after it.
  ex.code.innerHTML = highlight(v) + (v === "" || v.endsWith("\n") ? " " : "");
}

function setup(el, n) {
  const pre = el.querySelector(".live-editor pre");
  const code = pre.querySelector("code");
  const ex = {
    el,
    code,
    // The Markdown block's final newline would be an empty last line in the editor.
    original: code.textContent.replace(/\n$/, ""),
    runs: 0,
    timer: 0,
  };
  const hint = document.createElement("span");
  hint.className = "live-keys";
  hint.id = `${el.id}-keys`;
  hint.innerHTML = `<kbd>${RUN_KEYS}</kbd> run · <kbd>Tab</kbd> indent · <kbd>Esc</kbd> leave`;
  const note = el.querySelector(".live-note");
  note.textContent = "editable · runs in psh.wasm in your browser";
  note.after(hint);

  const ta = (ex.ta = document.createElement("textarea"));
  ta.className = "live-src";
  ta.value = ex.original;
  ta.spellcheck = false;
  for (const [k, v] of Object.entries({ autocapitalize: "off", autocomplete: "off", autocorrect: "off", wrap: "soft" }))
    ta.setAttribute(k, v);
  ta.setAttribute("aria-label", `Example ${n}: shell, editable`);
  ta.setAttribute("aria-describedby", hint.id);
  pre.setAttribute("aria-hidden", "true");
  pre.after(ta);
  repaint(ex);
  el.classList.add("is-live");

  const runBtn = el.querySelector(".live-run");
  runBtn.innerHTML = `Run <kbd>${RUN_KEYS}</kbd>`;
  runBtn.setAttribute("aria-keyshortcuts", MAC ? "Meta+Enter" : "Control+Enter");
  runBtn.hidden = false;
  runBtn.onclick = () => run(ex);
  const reset = el.querySelector(".live-reset");
  reset.onclick = () => {
    // Through the editing commands, so that undo brings the edits back.
    ta.focus();
    ta.select();
    insert(ta, ex.original);
  };

  ta.addEventListener("keydown", (ev) => onKey(ev, ex));
  ta.addEventListener("input", () => {
    // Invalidate an older run immediately, not only when the autorun timer fires.
    ++ex.runs;
    el.querySelector('[data-pane="classify"]').innerHTML = '<p class="muted">Classification pending…</p>';
    repaint(ex);
    reset.hidden = ta.value === ex.original;
    el.querySelector(".live-status").textContent = "edited";
    clearTimeout(ex.timer);
    ex.timer = setTimeout(() => run(ex), AUTORUN_MS);
  });
  // The textarea never scrolls: the pre under it sizes the editor.
  ta.addEventListener("scroll", () => (ta.scrollTop = ta.scrollLeft = 0));
  // Start loading psh.wasm when the reader is about to use it.
  const warm = () => lib().catch(() => {});
  ta.addEventListener("focus", warm, { once: true });
  runBtn.addEventListener("pointerenter", warm, { once: true });

  // Tabs: arrow keys move between them (the ARIA tab pattern).
  const tabs = [...el.querySelectorAll('[role="tab"]')];
  const select = (b) => {
    for (const t of tabs) {
      t.setAttribute("aria-selected", String(t === b));
      t.tabIndex = t === b ? 0 : -1;
    }
    for (const p of el.querySelectorAll(".live-pane")) p.hidden = p.dataset.pane !== b.dataset.tab;
  };
  for (const [i, b] of tabs.entries()) {
    b.hidden = false;
    b.onclick = () => select(b);
    b.onkeydown = (ev) => {
      const j = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: tabs.length - 1 }[ev.key];
      if (j === undefined) return;
      ev.preventDefault();
      const t = tabs[(j + tabs.length) % tabs.length];
      select(t);
      t.focus();
    };
  }
  return ex;
}

const examples = [...document.querySelectorAll(".live")].map((el, i) => setup(el, i + 1));

// `#run` in the URL runs every example on load (for screenshots and links).
if (location.hash === "#run") for (const ex of examples) run(ex);
