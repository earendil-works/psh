// Live examples: every ```sh live block of the guide is an editor whose code runs in
// psh.wasm in the page, in a fresh shell over js/psh.mjs's in-memory host, in an empty
// /work with the environment the build used (crates/psh-docs/src/run.rs). The editor is
// a textarea with transparent text over the highlighted `pre` the page already has
// (render.rs live_block): the textarea does the editing (caret, selection, undo, input
// methods, screen readers) and highlight.js repaints the `pre` under it on every input.
// Without JavaScript (or from file://) the page shows the code and the output and
// classification and AST made at build time.
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

// ---- The AST pane -------------------------------------------------------------------
// The tree is psh_syntax::outline: one row per node with its byte span (run.rs ast_html
// renders the same at build time). Hovering a row marks its span in the code; the caret
// in the code selects the deepest row there; rows fold; JSON shows `psh --parse` itself.

const enc = new TextEncoder();

function viewBar(view) {
  const b = (v, label) => `<button data-view="${v}" aria-pressed="${view === v}">${label}</button>`;
  return `<div class="ast-bar">${b("tree", "Tree")}${b("json", "JSON")}</div>`;
}

/** The rows as the tree: run.rs tree_html. */
function treeHtml(rows) {
  if (!rows.length) return `<p class="muted">(no commands)</p>`;
  let h = `<div class="ast-tree">`;
  rows.forEach((r, i) => {
    const open = i + 1 < rows.length && rows[i + 1].depth > r.depth ? ` data-open="true"` : "";
    h += `<div class="ast-row" data-depth="${r.depth}"${open} data-span="${r.span[0]} ${r.span[1]}" style="--depth:${r.depth}"><span class="ast-twisty"></span>`;
    if (r.role) h += ` <span class="ast-role">${esc(r.role)}</span>`;
    h += ` <span class="ast-kind">${esc(r.kind)}</span>`;
    if (r.info) h += ` <span class="ast-info">${esc(r.info)}</span>`;
    if (r.text) h += ` <code class="ast-text">${esc(r.text)}</code>`;
    h += `</div>`;
  });
  return h + `</div>`;
}

/** `psh --parse`'s JSON, indented and highlighted. */
function jsonHtml(result) {
  const json = JSON.stringify(result, null, 2);
  // Split on JSON tokens before escaping, so source text cannot introduce markup.
  return json.split(/("(?:\\.|[^"\\])*"|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|\b(?:true|false|null)\b)/g)
    .map((s, i) => {
      if (i % 2 === 0) return esc(s);
      const role = s[0] === '"' ? "string" : /^(true|false|null)$/.test(s) ? "keyword" : "number";
      return `<span class="tok-${role}">${esc(s)}</span>`;
    }).join("");
}

/** The AST pane for `view` from `psh.parse`'s result (outline rows for the tree). */
function astHtml(view, r) {
  let body;
  if (view === "json") body = `<pre class="ast-json">${jsonHtml(r)}</pre>`;
  else if (!r.ok) body = `<p class="live-error">Does not parse: ${esc(r.error.message)} (line ${r.error.line}, column ${r.error.column})</p>`;
  else body = treeHtml(r.rows);
  return viewBar(view) + body;
}

/** Fills the AST pane for `src`, which is the code the rows' spans point into. */
function showAst(ex, psh, src) {
  const r = psh.parse(src, { outline: ex.view === "tree" }).json;
  ex.pane("ast").innerHTML = astHtml(ex.view, r);
  ex.astSrc = src;
  ex.active = null;
  syncCaret(ex);
}

/** The parser for the JSON view, shared by all examples (runs make their own). */
let analyzerPromise;
function analyzer() {
  analyzerPromise ??= lib().then(({ loadPsh, MemHost, module }) => loadPsh(module, { host: new MemHost() }));
  return analyzerPromise;
}

const rowsOf = (ex) => [...ex.pane("ast").querySelectorAll(".ast-row")];
const depthOf = (row) => Number(row.dataset.depth);
const spanOf = (row) => row.dataset.span.split(" ").map(Number);

/** Whether the AST pane shows a tree for the code in the editor. */
function treeIsCurrent(ex) {
  return !ex.pane("ast").hidden && ex.view === "tree" && ex.astSrc === ex.ta.value;
}

/** Marks the span of `row` (none: nothing) in the code. */
function markRow(ex, row) {
  if (!row || !treeIsCurrent(ex)) {
    ex.marks.innerHTML = "";
    return;
  }
  const bytes = enc.encode(ex.astSrc);
  const [a, b] = spanOf(row).map((n) => dec().decode(bytes.subarray(0, n)).length);
  const v = ex.astSrc;
  const cls = a === b ? ` class="empty"` : "";
  ex.marks.innerHTML = `${esc(v.slice(0, a))}<mark${cls}>${esc(v.slice(a, b))}</mark>${esc(v.slice(b))}` +
    (v === "" || v.endsWith("\n") ? " " : "");
}

/** Shows only the rows whose ancestors are all open. */
function applyFolds(ex) {
  let closedAt = Infinity;
  for (const row of rowsOf(ex)) {
    const d = depthOf(row);
    if (d > closedAt) {
      row.hidden = true;
      continue;
    }
    row.hidden = false;
    closedAt = row.dataset.open === "false" ? d : Infinity;
  }
}

function setOpen(ex, row, open) {
  if (row.dataset.open === undefined || row.dataset.open === String(open)) return;
  row.dataset.open = String(open);
  applyFolds(ex);
}

/** Makes `row` the current row: highlighted, its ancestors open, scrolled into view. */
function setActive(ex, row) {
  ex.active?.classList.remove("is-active");
  ex.active = row;
  if (!row) return markRow(ex, null);
  row.classList.add("is-active");
  if (row.hidden) {
    let d = depthOf(row);
    for (let p = row.previousElementSibling; p && d > 0; p = p.previousElementSibling) {
      if (depthOf(p) < d) {
        p.dataset.open = "true";
        d = depthOf(p);
      }
    }
    applyFolds(ex);
  }
  // Scroll the pane only, never the page under the reader's caret, and not under the
  // sticky Tree/JSON bar.
  const pane = ex.pane("ast");
  const top = row.offsetTop;
  const bar = pane.querySelector(".ast-bar")?.offsetHeight ?? 0;
  if (top - bar < pane.scrollTop) pane.scrollTop = top - bar;
  else if (top + row.offsetHeight > pane.scrollTop + pane.clientHeight) pane.scrollTop = top + row.offsetHeight - pane.clientHeight;
  markRow(ex, row);
}

/** Selects the deepest row at the caret in the code. */
function syncCaret(ex) {
  if (!treeIsCurrent(ex)) return markRow(ex, null);
  if (!ex.caretSeen) return markRow(ex, ex.active);
  const at = enc.encode(ex.ta.value.slice(0, ex.ta.selectionStart)).length;
  let inside = null;
  let touching = null;
  // Children follow their parent, so the last row that holds the caret is the deepest.
  for (const row of rowsOf(ex)) {
    const [a, b] = spanOf(row);
    if (a <= at && at < b) inside = row;
    else if (a <= at && at === b) touching = row;
  }
  setActive(ex, inside ?? touching);
}

async function setView(ex, view) {
  if (ex.view === view) return;
  ex.view = view;
  const src = ex.astSrc;
  // Pending (the code was edited): the next run fills the pane in this view.
  if (src === null || src !== ex.ta.value) return;
  const psh = await analyzer();
  if (ex.astSrc === src && ex.ta.value === src) showAst(ex, psh, src);
}

function setupAst(ex) {
  const pane = ex.pane("ast");
  pane.addEventListener("click", (ev) => {
    const button = ev.target.closest("button[data-view]");
    if (button) return void setView(ex, button.dataset.view).catch(() => {});
    const row = ev.target.closest(".ast-row");
    if (!row) return;
    if (ev.target.closest(".ast-twisty")) setOpen(ex, row, row.dataset.open === "false");
    setActive(ex, row);
  });
  pane.addEventListener("pointerover", (ev) => {
    const row = ev.target.closest(".ast-row");
    if (row) markRow(ex, row);
  });
  pane.addEventListener("pointerleave", () => markRow(ex, ex.active));
  // The ARIA tree keys, on the focused pane: up and down move, right opens or goes to
  // the first child, left closes or goes to the parent.
  pane.addEventListener("keydown", (ev) => {
    if (ev.target !== pane || ev.altKey || ev.ctrlKey || ev.metaKey || !treeIsCurrent(ex)) return;
    const rows = rowsOf(ex).filter((r) => !r.hidden);
    if (!rows.length) return;
    const row = ex.active && !ex.active.hidden ? ex.active : null;
    const i = row ? rows.indexOf(row) : -1;
    let next;
    if (ev.key === "ArrowDown") next = rows[Math.min(i + 1, rows.length - 1)];
    else if (ev.key === "ArrowUp") next = rows[Math.max(i - 1, 0)];
    else if (ev.key === "Home") next = rows[0];
    else if (ev.key === "End") next = rows[rows.length - 1];
    else if (ev.key === "ArrowRight" && row) {
      if (row.dataset.open === "false") setOpen(ex, row, true);
      next = row.dataset.open === "true" ? rows[i + 1] : row;
    } else if (ev.key === "ArrowLeft" && row) {
      if (row.dataset.open === "true") {
        setOpen(ex, row, false);
        next = row;
      } else next = rows.slice(0, i).reverse().find((r) => depthOf(r) < depthOf(row)) ?? row;
    } else return;
    ev.preventDefault();
    setActive(ex, next);
  });
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
    // Analysis needs only the text, not the outcome of a possibly slow run.
    pane("classify").innerHTML = classificationHtml(src, psh.classify(src).json);
    showAst(ex, psh, src);
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
  // The Markdown block's final newline would be an empty last line in the editor.
  const original = code.textContent.replace(/\n$/, "");
  const ex = {
    el,
    code,
    original,
    runs: 0,
    timer: 0,
    pane: (n) => el.querySelector(`[data-pane="${n}"]`),
    // The AST pane: its view, the code its rows were made from (the build's tree at
    // first, null while an edit is pending), the current row, and whether the reader
    // has put the caret in the code (before that, the caret selects no row).
    view: "tree",
    astSrc: original,
    active: null,
    caretSeen: false,
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
  // Under the textarea, over the code: the span of the AST row in focus.
  const marks = (ex.marks = document.createElement("div"));
  marks.className = "live-marks";
  marks.setAttribute("aria-hidden", "true");
  pre.after(marks, ta);
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
  // Where the caret is, in the AST.
  const caret = () => {
    if (document.activeElement === ta) ex.caretSeen = true;
    syncCaret(ex);
  };
  for (const k of ["selectionchange", "keyup", "pointerup"]) ta.addEventListener(k, caret);
  setupAst(ex);
  ta.addEventListener("input", () => {
    // Invalidate an older run immediately, not only when the autorun timer fires.
    ++ex.runs;
    el.querySelector('[data-pane="classify"]').innerHTML = '<p class="muted">Classification pending…</p>';
    el.querySelector('[data-pane="ast"]').innerHTML = '<p class="muted">AST pending…</p>';
    ex.astSrc = null;
    ex.active = null;
    marks.innerHTML = "";
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
    syncCaret(ex);
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
