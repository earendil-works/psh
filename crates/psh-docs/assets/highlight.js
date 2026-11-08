// The shell half of crates/psh-docs/src/highlight.rs, for the live examples' editor: the
// same rules and the same roles (tok-function, tok-variable, ...), so code looks the
// same while it is edited as it does in the static page. A test in highlight.rs runs
// this file in node over the guide's shell blocks and compares it with the Rust output.

const KW = new Set(
  "if then else elif fi for while until do done case esac in function select time ! { } [[ ]] coproc".split(" "),
);
// Keywords after which the next word is a command name again.
const KW_CMD = new Set("if then else elif while until do ! { time coproc function".split(" "));
// Longest first.
const OPS = [
  ";;&", "<<<", "<<-", "&>>", ";;", ";&", "||", "&&", "|&", ">>", "<<", "&>", ">&", "<&", "<>",
  ">|", "<(", ">(", "|", "&", ";", "<", ">", "(", ")",
];
const CMD_OPS = new Set(["|", "||", "&&", "&", ";", ";;", ";&", ";;&", "|&", "(", ")", "<(", ">("]);

const isWs = (c) => c === " " || c === "\t" || c === "\n" || c === "\r" || c === "\f";
const special = (c) => isWs(c) || "|&;<>()'\"$`\\".includes(c);
const alpha = (c) => /^[A-Za-z]$/.test(c);
const alnum = (c) => /^[A-Za-z0-9]$/.test(c);
const digit = (c) => /^[0-9]$/.test(c);

export function esc(s) {
  return s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

/**
 * Splits shell `code` into `[role, text]` tokens (role null for plain text); joined, the
 * texts are `code` again. A block whose lines start with `$ ` is a session.
 */
export function tokens(code) {
  if (!code.split("\n").some((l) => l.startsWith("$ "))) return new Sh(code).run();
  const out = [];
  for (const line of code.match(/[^\n]*\n|[^\n]+$/g) ?? []) {
    if (line.startsWith("$ ")) {
      out.push(["punctuation", "$"], [null, " "], ...new Sh(line.slice(2)).run());
    } else out.push([null, line]);
  }
  return out;
}

class Sh {
  constructor(code) {
    this.c = Array.from(code);
    this.i = 0;
    this.out = [];
    this.cmd = true;
    this.loopVar = false;
    this.expectIn = false;
    this.heredocs = [];
    this.delimNext = null;
  }

  text(a, b) {
    return this.c.slice(a, b).join("");
  }

  push(role, a, b) {
    this.out.push([role, this.text(a, b)]);
  }

  wordStart() {
    if (this.i === 0) return true;
    const p = this.c[this.i - 1];
    return isWs(p) || "|&;<>()".includes(p);
  }

  run() {
    const c = this.c;
    while (this.i < c.length) {
      const start = this.i;
      const ch = c[start];
      const atWord = this.wordStart();
      if (ch === "\n") {
        this.i++;
        this.push(null, start, this.i);
        this.cmd = true;
        this.expectIn = false;
        this.bodies();
      } else if (ch === " " || ch === "\t" || ch === "\r") {
        while (this.i < c.length && (c[this.i] === " " || c[this.i] === "\t" || c[this.i] === "\r")) this.i++;
        this.push(null, start, this.i);
      } else if (ch === "#" && atWord) {
        while (this.i < c.length && c[this.i] !== "\n") this.i++;
        this.push("comment", start, this.i);
      } else if (ch === "\\") {
        this.i = Math.min(this.i + 2, c.length);
        this.push(null, start, this.i);
        if (atWord) this.cmd = false;
      } else if (ch === "'" || (ch === "$" && c[start + 1] === "'")) {
        this.single(ch === "$");
        this.afterWord(atWord);
      } else if (ch === '"') {
        this.double();
        this.afterWord(atWord);
      } else if (ch === "$") {
        this.dollar(atWord);
      } else if (ch === "`") {
        this.i++;
        this.push("punctuation", start, this.i);
        this.cmd = true;
      } else {
        const op = this.op();
        if (op !== null) {
          this.i += Array.from(op).length;
          this.push("punctuation", start, this.i);
          if (op === "<<") this.delimNext = false;
          else if (op === "<<-") this.delimNext = true;
          else if (CMD_OPS.has(op)) {
            this.cmd = true;
            this.expectIn = false;
          }
        } else this.word(atWord);
      }
    }
    return this.out;
  }

  op() {
    return OPS.find((op) => Array.from(op).every((o, k) => this.c[this.i + k] === o)) ?? null;
  }

  afterWord(atWord) {
    if (!atWord) return;
    if (this.delimNext !== null) {
      const strip = this.delimNext;
      this.delimNext = null;
      const last = this.out.length ? this.out[this.out.length - 1][1] : "";
      this.heredocs.push([last.replace(/^['"]+|['"]+$/g, ""), strip]);
    }
    this.cmd = false;
    this.loopVar = false;
  }

  single(ansi) {
    const c = this.c;
    const start = this.i;
    this.i += ansi ? 2 : 1;
    while (this.i < c.length && c[this.i] !== "'") {
      if (ansi && c[this.i] === "\\") this.i++;
      this.i++;
    }
    this.i = Math.min(this.i + 1, c.length);
    this.push("string", start, this.i);
  }

  double() {
    const c = this.c;
    let seg = this.i;
    this.i++;
    while (this.i < c.length && c[this.i] !== '"') {
      if (c[this.i] === "\\") this.i += 2;
      else if (c[this.i] === "$") {
        const a = this.i;
        const b = this.varEnd(a);
        if (b > a + 1) {
          this.push("string", seg, a);
          this.push("variable", a, b);
          seg = b;
          this.i = b;
        } else this.i++;
      } else this.i++;
    }
    this.i = Math.min(this.i + 1, c.length);
    if (this.i > seg) this.push("string", seg, this.i);
  }

  varEnd(a) {
    const c = this.c;
    const x = c[a + 1];
    if (x === "{") {
      let depth = 0;
      let j = a + 1;
      while (j < c.length) {
        if (c[j] === "{") depth++;
        else if (c[j] === "}") {
          depth--;
          if (depth === 0) return j + 1;
        } else if (c[j] === "\n") return j;
        j++;
      }
      return j;
    }
    if (x !== undefined && (alpha(x) || x === "_")) {
      let j = a + 1;
      while (j < c.length && (alnum(c[j]) || c[j] === "_")) j++;
      return j;
    }
    if (x !== undefined && (digit(x) || "?@*#$!-".includes(x))) return a + 2;
    return a + 1;
  }

  dollar(atWord) {
    const c = this.c;
    const start = this.i;
    if (c[start + 1] === "(") {
      const arith = c[start + 2] === "(";
      this.i += arith ? 3 : 2;
      this.push("punctuation", start, this.i);
      this.cmd = !arith;
      return;
    }
    const end = this.varEnd(start);
    this.i = end;
    this.push(end > start + 1 ? "variable" : null, start, end);
    this.afterWord(atWord);
  }

  word(atWord) {
    const c = this.c;
    const start = this.i;
    while (this.i < c.length && !special(c[this.i])) this.i++;
    if (this.i === start) this.i++;
    const w = this.text(start, this.i);
    if (!atWord) return this.push(null, start, this.i);
    if (this.delimNext !== null) {
      this.push("string", start, this.i);
      this.heredocs.push([w, this.delimNext]);
      this.delimNext = null;
      this.cmd = false;
      return;
    }
    if (this.loopVar) {
      this.loopVar = false;
      this.expectIn = true;
      return this.push("variable", start, this.i);
    }
    if ((KW.has(w) && this.cmd) || w === "]]" || (w === "in" && this.expectIn)) {
      this.push("keyword", start, this.i);
      this.cmd = KW_CMD.has(w);
      this.expectIn = w === "case";
      this.loopVar = w === "for" || w === "select";
      return;
    }
    if (this.cmd) {
      const eq = w.indexOf("=");
      if (eq >= 0) {
        const name = w.slice(0, eq).replace(/\++$/, "");
        if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
          const k = Array.from(w.slice(0, eq + 1)).length;
          this.push("variable", start, start + k);
          if (start + k < this.i) this.push(null, start + k, this.i);
          return;
        }
      }
      this.cmd = false;
      return this.push("function", start, this.i);
    }
    this.push(/^[0-9]+$/.test(w) ? "number" : null, start, this.i);
  }

  bodies() {
    const c = this.c;
    const docs = this.heredocs;
    this.heredocs = [];
    for (const [delim, strip] of docs) {
      const start = this.i;
      while (this.i < c.length) {
        const ls = this.i;
        while (this.i < c.length && c[this.i] !== "\n") this.i++;
        let line = this.text(ls, this.i);
        if (this.i < c.length) this.i++;
        if (strip) line = line.replace(/^\t+/, "");
        if (line === delim) break;
      }
      if (this.i > start) this.push("string", start, this.i);
    }
  }
}

/** The highlighted HTML of shell `code` (highlight.rs `highlight("sh", code)`). */
export function highlight(code) {
  return tokens(code)
    .map(([role, s]) => (role ? `<span class="tok-${role}">${esc(s)}</span>` : esc(s)))
    .join("");
}
