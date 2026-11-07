//! Word expansion: brace expansion, tilde, parameters, command substitution, word
//! splitting on `IFS`, pathname expansion and quote removal (bash manual, "Shell
//! Expansions"), over the parser's word parts.
//!
//! The expander appends to [`Fields`]: quoted text, unquoted literal text and unquoted
//! expansion results are told apart there, because only the last are split, and only
//! unquoted text keeps its pattern chars for globbing.

use std::rc::Rc;

use psh_syntax::{
    Brace, CaseKind, DefaultKind, ParamName, ParamOp, ParamPrefix, Part, ReplaceKind, Subscript,
    Word, WordPart,
};

use crate::exec;
use crate::options::{SetOpt, Shopt};
use crate::pattern::{self, Pattern, ReplaceMode};
use crate::sched::LocalBoxFuture;
use crate::state::{Code, Unwind, Vp, is_name};

/// One field under construction.
#[derive(Clone, Default, Debug)]
pub(crate) struct Field {
    /// The text after quote removal.
    pub text: Vec<u8>,
    /// The same as a pattern: quoted pattern chars escaped with a backslash.
    pub pat: Vec<u8>,
    /// It has unquoted pattern chars.
    pub glob: bool,
    started: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pending {
    None,
    /// IFS whitespace was seen after text: the next text starts a new field.
    Ws,
    /// A non-whitespace IFS char ended the field.
    Delim,
}

/// The fields of the words being expanded.
pub(crate) struct Fields {
    pub out: Vec<Field>,
    cur: Field,
    pending: Pending,
    ifs: Vec<u8>,
    /// Word splitting applies (command words); off for assignments, redirection
    /// targets, here-strings, patterns.
    split: bool,
    /// The status of the last command substitution, for `$?` after an assignment-only
    /// command.
    pub subst_status: Option<i32>,
}

fn is_ifs_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n')
}

impl Fields {
    pub(crate) fn new(vp: &Vp, split: bool) -> Fields {
        let ifs = match vp.var("IFS") {
            Some(v) => v.to_vec(),
            None => b" \t\n".to_vec(),
        };
        Fields {
            out: Vec::new(),
            cur: Field::default(),
            pending: Pending::None,
            ifs,
            split,
            subst_status: None,
        }
    }

    fn push_cur(&mut self) {
        let f = std::mem::take(&mut self.cur);
        self.out.push(f);
    }

    fn begin(&mut self) {
        match self.pending {
            Pending::Ws => {
                self.push_cur();
            }
            Pending::Delim | Pending::None => {}
        }
        self.pending = Pending::None;
    }

    /// Quoted text: no splitting, no globbing; even empty, it makes a field.
    pub(crate) fn quoted(&mut self, s: &[u8]) {
        self.begin();
        self.cur.text.extend_from_slice(s);
        pattern::escape(s, &mut self.cur.pat);
        self.cur.started = true;
    }

    /// Unquoted text from the source: not split, but its pattern chars are live.
    pub(crate) fn literal(&mut self, s: &[u8]) {
        self.begin();
        self.cur.text.extend_from_slice(s);
        self.cur.pat.extend_from_slice(s);
        if pattern::has_glob_chars(s) {
            self.cur.glob = true;
        }
        self.cur.started = true;
    }

    /// The unquoted result of an expansion: split on IFS, pattern chars live.
    pub(crate) fn unquoted(&mut self, s: &[u8]) {
        if !self.split {
            if !s.is_empty() {
                self.literal(s);
            }
            return;
        }
        for &b in s {
            if self.ifs.contains(&b) {
                if is_ifs_ws(b) {
                    if self.pending == Pending::None && self.cur.started {
                        self.pending = Pending::Ws;
                    }
                } else {
                    self.push_cur();
                    self.pending = Pending::Delim;
                }
                continue;
            }
            self.begin();
            self.cur.text.push(b);
            self.cur.pat.push(b);
            if matches!(b, b'*' | b'?' | b'[') {
                self.cur.glob = true;
            }
            self.cur.started = true;
        }
    }

    /// Between the elements of `$@`: a new field.
    fn field_break(&mut self, quoted: bool) {
        if !self.split {
            self.quoted(b" ");
            return;
        }
        if quoted {
            self.begin();
            self.push_cur();
        } else {
            if self.pending == Pending::Ws || (self.pending == Pending::None && self.cur.started) {
                self.push_cur();
            } else {
                self.cur = Field::default();
            }
            self.pending = Pending::None;
        }
    }

    /// Ends a word.
    pub(crate) fn finish_word(&mut self) {
        match self.pending {
            Pending::Ws => self.push_cur(),
            Pending::Delim => self.cur = Field::default(),
            Pending::None => {
                if self.cur.started {
                    self.push_cur();
                } else {
                    self.cur = Field::default();
                }
            }
        }
        self.pending = Pending::None;
    }

    /// All fields joined with spaces (for contexts that want one string).
    pub(crate) fn joined(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (i, f) in self.out.iter().enumerate() {
            if i > 0 {
                out.push(b' ');
            }
            out.extend_from_slice(&f.text);
        }
        out
    }
}

/// A piece of a brace-expanded word: a part of the source, or text a sequence
/// expression produced (unquoted literal).
enum Piece<'a> {
    Part(&'a WordPart),
    Text(Vec<u8>),
}

/// Brace expansion of a word into piece lists (one per resulting word).
fn brace_expand<'a>(code: &'a Code, parts: &'a [WordPart]) -> Vec<Vec<Piece<'a>>> {
    let mut results: Vec<Vec<Piece<'a>>> = vec![Vec::new()];
    for part in parts {
        match &part.kind {
            Part::Brace(b) => {
                let alts: Vec<Vec<Piece<'a>>> = match &**b {
                    Brace::Alts(words) => words
                        .iter()
                        .flat_map(|w| brace_expand(code, &w.parts))
                        .collect(),
                    Brace::Seq { start, end, step } => {
                        let s = code.ast.text(*start);
                        let e = code.ast.text(*end);
                        let st = step.map(|sp| code.ast.text(sp));
                        match seq(s, e, st) {
                            Some(items) => {
                                items.into_iter().map(|t| vec![Piece::Text(t)]).collect()
                            }
                            None => vec![vec![Piece::Text(code.ast.bytes(part.span).to_vec())]],
                        }
                    }
                };
                let mut next = Vec::with_capacity(results.len() * alts.len());
                for prefix in &results {
                    for alt in &alts {
                        let mut v: Vec<Piece<'a>> = prefix.iter().map(clone_piece).collect();
                        v.extend(alt.iter().map(clone_piece));
                        next.push(v);
                    }
                }
                results = next;
            }
            _ => {
                for r in &mut results {
                    r.push(Piece::Part(part));
                }
            }
        }
    }
    results
}

fn clone_piece<'a>(p: &Piece<'a>) -> Piece<'a> {
    match p {
        Piece::Part(p) => Piece::Part(p),
        Piece::Text(t) => Piece::Text(t.clone()),
    }
}

/// `{x..y[..step]}` with integers or single chars.
fn seq(start: &str, end: &str, step: Option<&str>) -> Option<Vec<Vec<u8>>> {
    // i128, so that steps and ends at the edges of i64 cannot overflow.
    let step_n: i128 = match step {
        // bash leaves a step of i64::MIN (which has no absolute value) unexpanded.
        Some(s) => s.parse::<i64>().ok().filter(|&n| n != i64::MIN)?.into(),
        None => 1,
    };
    let step_abs = step_n.abs().max(1);
    if let (Ok(a), Ok(b)) = (start.parse::<i64>(), end.parse::<i64>()) {
        let (a, b) = (i128::from(a), i128::from(b));
        // Zero padding when either end has a leading zero.
        let pad = |s: &str| {
            let d = s.trim_start_matches('-');
            if d.len() > 1 && d.starts_with('0') {
                s.len()
            } else {
                0
            }
        };
        let width = pad(start).max(pad(end));
        let mut out = Vec::new();
        let mut i = a;
        loop {
            if (a <= b && i > b) || (a > b && i < b) {
                break;
            }
            let s = if width > 0 {
                if i < 0 {
                    format!("-{:0w$}", -i, w = width - 1)
                } else {
                    format!("{:0w$}", i, w = width)
                }
            } else {
                i.to_string()
            };
            out.push(s.into_bytes());
            if out.len() > 1_000_000 {
                return None;
            }
            i = if a <= b { i + step_abs } else { i - step_abs };
        }
        return Some(out);
    }
    let (a, b) = (start.as_bytes(), end.as_bytes());
    if a.len() == 1 && b.len() == 1 && a[0].is_ascii_alphabetic() && b[0].is_ascii_alphabetic() {
        let (a, b) = (i128::from(a[0]), i128::from(b[0]));
        let mut out = Vec::new();
        let mut i = a;
        loop {
            if (a <= b && i > b) || (a > b && i < b) {
                break;
            }
            out.push(vec![i as u8]);
            i = if a <= b { i + step_abs } else { i - step_abs };
        }
        return Some(out);
    }
    None
}

/// Expands command words: brace expansion, all expansions, splitting, globbing.
pub(crate) async fn expand_words(
    vp: &mut Vp,
    code: &Rc<Code>,
    words: &[Word],
) -> Result<Fields, Unwind> {
    let mut f = Fields::new(vp, true);
    for w in words {
        for pieces in brace_expand(code, &w.parts) {
            expand_pieces(vp, code, &pieces, false, &mut f, Ctx::Word).await?;
            f.finish_word();
        }
    }
    glob_fields(vp, &mut f).await?;
    Ok(f)
}

/// Expands a word to one string without splitting or globbing: assignment values,
/// here-strings, `case` words.
pub(crate) async fn expand_single(
    vp: &mut Vp,
    code: &Rc<Code>,
    word: &Word,
    ctx: Ctx,
) -> Result<(Vec<u8>, Option<i32>), Unwind> {
    let mut f = Fields::new(vp, false);
    let pieces: Vec<Piece> = word.parts.iter().map(Piece::Part).collect();
    expand_pieces(vp, code, &pieces, false, &mut f, ctx).await?;
    f.finish_word();
    let st = f.subst_status;
    Ok((f.joined(), st))
}

/// Expands a word to a pattern (quoted pattern chars escaped).
pub(crate) async fn expand_pattern(
    vp: &mut Vp,
    code: &Rc<Code>,
    word: &Word,
) -> Result<Vec<u8>, Unwind> {
    let mut f = Fields::new(vp, false);
    let pieces: Vec<Piece> = word.parts.iter().map(Piece::Part).collect();
    expand_pieces(vp, code, &pieces, false, &mut f, Ctx::Word).await?;
    f.finish_word();
    let mut out = Vec::new();
    for (i, field) in f.out.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        out.extend_from_slice(&field.pat);
    }
    Ok(out)
}

/// A redirection target: brace expansion and globbing apply; it must come out as one
/// word. `Err(None)` is "ambiguous redirect".
pub(crate) async fn expand_target(
    vp: &mut Vp,
    code: &Rc<Code>,
    word: &Word,
) -> Result<Option<Vec<u8>>, Unwind> {
    let f = expand_words(vp, code, std::slice::from_ref(word)).await?;
    if f.out.len() == 1 {
        Ok(Some(
            f.out.into_iter().next().map(|f| f.text).unwrap_or_default(),
        ))
    } else {
        Ok(None)
    }
}

/// The body of an unquoted here-document: expansions, as inside double quotes.
pub(crate) async fn expand_heredoc(
    vp: &mut Vp,
    code: &Rc<Code>,
    body: &Word,
) -> Result<Vec<u8>, Unwind> {
    let mut f = Fields::new(vp, false);
    let pieces: Vec<Piece> = body.parts.iter().map(Piece::Part).collect();
    expand_pieces(vp, code, &pieces, true, &mut f, Ctx::HereDoc).await?;
    f.finish_word();
    Ok(f.joined())
}

/// Where a word is: decides tilde expansion and a few quoting details.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ctx {
    Word,
    /// An assignment value: `~` also after `:`.
    Assign,
    HereDoc,
}

fn expand_pieces<'a>(
    vp: &'a mut Vp,
    code: &'a Rc<Code>,
    pieces: &'a [Piece<'a>],
    quoted: bool,
    f: &'a mut Fields,
    ctx: Ctx,
) -> LocalBoxFuture<'a, Result<(), Unwind>> {
    Box::pin(async move {
        for (i, piece) in pieces.iter().enumerate() {
            match piece {
                Piece::Text(t) => f.literal(t),
                Piece::Part(p) => {
                    expand_part(vp, code, p, quoted, f, ctx, i == 0).await?;
                }
            }
        }
        Ok(())
    })
}

fn expand_parts<'a>(
    vp: &'a mut Vp,
    code: &'a Rc<Code>,
    parts: &'a [WordPart],
    quoted: bool,
    f: &'a mut Fields,
) -> LocalBoxFuture<'a, Result<(), Unwind>> {
    Box::pin(async move {
        for (i, p) in parts.iter().enumerate() {
            expand_part(vp, code, p, quoted, f, Ctx::Word, i == 0 && !quoted).await?;
        }
        Ok(())
    })
}

/// The home directory for `~` and the like.
fn tilde(vp: &Vp, name: &[u8]) -> Option<Vec<u8>> {
    match name {
        b"" => vp.var("HOME").map(<[u8]>::to_vec),
        b"+" => vp.var("PWD").map(<[u8]>::to_vec),
        b"-" => vp.var("OLDPWD").map(<[u8]>::to_vec),
        _ => None,
    }
}

/// Tilde expansion of unquoted literal text at the start of a word (and after `:` in
/// assignments). `first` is whether the text starts the word.
fn literal_with_tilde(vp: &Vp, text: &[u8], first: bool, ctx: Ctx, f: &mut Fields) {
    if ctx == Ctx::HereDoc {
        f.quoted(text);
        return;
    }
    let mut rest = text;
    let mut at_start = first;
    loop {
        if at_start && rest.first() == Some(&b'~') {
            let end = rest
                .iter()
                .position(|&b| b == b'/' || (ctx == Ctx::Assign && b == b':'))
                .unwrap_or(rest.len());
            // Only a tilde prefix that ends the literal text or a slash counts; a
            // quoted char right after it (`~"x"`) makes it literal, which we cannot
            // see here, so text that ends this part is accepted.
            if let Some(home) = tilde(vp, &rest[1..end]) {
                f.quoted(&home);
                rest = &rest[end..];
            }
        }
        if ctx != Ctx::Assign {
            f.literal(rest);
            return;
        }
        match rest.iter().position(|&b| b == b':') {
            Some(i) => {
                f.literal(&rest[..=i]);
                rest = &rest[i + 1..];
                at_start = true;
            }
            None => {
                f.literal(rest);
                return;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn expand_part(
    vp: &mut Vp,
    code: &Rc<Code>,
    part: &WordPart,
    quoted: bool,
    f: &mut Fields,
    ctx: Ctx,
    first: bool,
) -> Result<(), Unwind> {
    let text = code.ast.bytes(part.span);
    match &part.kind {
        Part::Lit => {
            if quoted {
                f.quoted(text);
            } else if text.first() == Some(&b'~') || ctx == Ctx::Assign {
                literal_with_tilde(vp, text, first, ctx, f);
            } else {
                f.literal(text);
            }
        }
        Part::Escaped => {
            let c = &text[1..];
            // In double quotes (a `${x-\z}` operand) only `$ ` " \ }` and newline are
            // escapes; the backslash of anything else stays.
            if quoted && !matches!(c.first(), Some(b'$' | b'`' | b'"' | b'\\' | b'}' | b'\n')) {
                f.quoted(text);
            } else {
                f.quoted(c);
            }
        }
        Part::Single => f.quoted(&text[1..text.len() - 1]),
        Part::AnsiC => {
            let inner = &text[2..text.len() - 1];
            f.quoted(&psh_syntax::decode_ansi_c(inner));
        }
        Part::Double(inner) | Part::Locale(inner) => {
            // `"…"` makes a field even when everything inside is empty, except that
            // `"$@"` with no parameters makes none.
            if inner.is_empty() || !inner.iter().all(is_at) {
                f.quoted(b"");
            }
            expand_parts(vp, code, inner, true, f).await?;
        }
        Part::Param(p) => param(vp, code, p, part, quoted, f).await?,
        Part::BadSubst => {
            vp.error(&format!(
                "{}: bad substitution",
                String::from_utf8_lossy(text)
            ))
            .await;
            return Err(Unwind::Abort(1));
        }
        Part::CmdSubst(list) => {
            let (out, st) = exec::command_subst(vp, code, *list).await?;
            f.subst_status = Some(st);
            emit(f, &out, quoted);
        }
        Part::Backtick => {
            // Unescaped on the UTF-8 view, then back to the source's bytes.
            let body = psh_syntax::backtick_body(code.ast.text(part.span), quoted);
            let body = code.ast.restore_bytes(&body).into_owned();
            let (out, st) = exec::command_subst_text(vp, code, &body, part.span.start).await?;
            f.subst_status = Some(st);
            emit(f, &out, quoted);
        }
        Part::Arith(w) => {
            let (text, st) = expand_single(vp, code, w, Ctx::Word).await?;
            if st.is_some() {
                f.subst_status = st;
            }
            match crate::arith::eval(vp, &text) {
                Ok(v) => emit(f, v.to_string().as_bytes(), quoted),
                Err(msg) => {
                    // An error in an arithmetic expansion abandons the command line.
                    vp.error(&msg).await;
                    return Err(Unwind::Abort(1));
                }
            }
        }
        Part::Brace(_) => {
            // Only reached where brace expansion does not apply: the text as is.
            if quoted {
                f.quoted(text);
            } else {
                f.literal(text);
            }
        }
        Part::ProcSubst { output, list } => {
            let path = exec::process_subst(vp, code, *list, *output).await?;
            f.literal(&path);
        }
        Part::FunSub { .. } => return unsupported(vp, "${ …; } substitution").await,
        Part::ExtGlob { .. } => return unsupported(vp, "extended glob").await,
        Part::Array(_) => return unsupported(vp, "array assignment").await,
    }
    Ok(())
}

/// `$@`, `${@}`, `${name[@]}`: expansions that may produce no field at all.
fn is_at(p: &WordPart) -> bool {
    match &p.kind {
        Part::Param(p) => {
            let at = p.name == ParamName::Special(b'@') || matches!(p.index, Some(Subscript::At));
            match p.prefix {
                // `"${!a[@]}"`: the keys, also possibly none.
                ParamPrefix::Indirect => {
                    matches!(p.index, Some(Subscript::At)) && matches!(p.op, ParamOp::None)
                }
                ParamPrefix::Length => false,
                ParamPrefix::None => {
                    at && matches!(
                        p.op,
                        ParamOp::None
                            | ParamOp::Substring { .. }
                            | ParamOp::RemovePrefix { .. }
                            | ParamOp::RemoveSuffix { .. }
                            | ParamOp::Replace { .. }
                            | ParamOp::Case { .. }
                            | ParamOp::Transform(_)
                    )
                }
            }
        }
        _ => false,
    }
}

/// The status a fatal expansion error (`${x?}`, `set -u`) ends the shell with: 127
/// under `-c` unless errexit is on, else 1, as bash 5.3 does.
pub(crate) fn expansion_exit_status(vp: &Vp) -> i32 {
    if vp.opts.dash_c && !vp.opts.get(SetOpt::Errexit) {
        127
    } else {
        1
    }
}

/// `set -u`: `NAME: unbound variable`, and the shell ends (D1282).
pub(crate) async fn unbound<T>(vp: &Vp, name: &str) -> Result<T, Unwind> {
    vp.error(&format!("{name}: unbound variable")).await;
    Err(Unwind::Exit(expansion_exit_status(vp)))
}

/// A subscript text from a nameref or `unset 'a[…]'`: expanded like a word (quotes
/// removed, `$x` and `$(…)` expanded), as bash does when it reparses it.
pub(crate) async fn expand_sub_text(vp: &mut Vp, sub: &[u8]) -> Result<Vec<u8>, Unwind> {
    if !sub
        .iter()
        .any(|b| matches!(b, b'$' | b'\'' | b'"' | b'\\' | b'`'))
    {
        return Ok(sub.to_vec());
    }
    let mut src = b": ".to_vec();
    src.extend_from_slice(sub);
    let Ok(ast) = psh_syntax::parse_bytes(&src, psh_syntax::Options::default()) else {
        return Ok(sub.to_vec());
    };
    let Some(id) = ast.cmd_ids().next() else {
        return Ok(sub.to_vec());
    };
    let psh_syntax::CommandKind::Simple(sc) = &ast.cmd(id).kind else {
        return Ok(sub.to_vec());
    };
    let Some(w) = sc.words.get(1).cloned() else {
        return Ok(sub.to_vec());
    };
    let code = Rc::new(Code {
        lines: Rc::new(psh_syntax::LineIndex::from_bytes(ast.source_bytes())),
        ast: std::sync::Arc::new(ast),
        line_base: vp.line.saturating_sub(1),
    });
    Ok(expand_single(vp, &code, &w, Ctx::Word).await?.0)
}

pub(crate) async fn unsupported<T>(vp: &Vp, what: &str) -> Result<T, Unwind> {
    vp.error(&format!("{what}: not supported yet")).await;
    // The script stops: what follows likely depends on it (D1057).
    Err(Unwind::Exit(2))
}

fn emit(f: &mut Fields, s: &[u8], quoted: bool) {
    if quoted {
        f.quoted(s);
    } else {
        f.unquoted(s);
    }
}

/// A parameter's value before its operator.
enum Val {
    Unset,
    Str(Vec<u8>),
    /// `$@` (`star` false) or `$*`.
    List(Vec<Vec<u8>>, bool),
}

fn special(vp: &Vp, c: u8) -> Val {
    match c {
        b'?' => Val::Str(vp.status.to_string().into_bytes()),
        b'#' => Val::Str(vp.args.len().to_string().into_bytes()),
        b'$' => Val::Str(vp.host().pid().to_string().into_bytes()),
        b'!' => match &vp.last_bg {
            Some(job) => Val::Str(job.pid().to_string().into_bytes()),
            None => Val::Unset,
        },
        b'-' => Val::Str(vp.opts.dash()),
        b'0' => Val::Str(vp.arg0.clone()),
        b'@' => Val::List(vp.args.clone(), false),
        b'*' => Val::List(vp.args.clone(), true),
        _ => Val::Unset,
    }
}

fn by_name(vp: &Vp, name: &[u8]) -> Option<Val> {
    if name.iter().all(u8::is_ascii_digit) && !name.is_empty() {
        let n: usize = std::str::from_utf8(name).ok()?.parse().ok()?;
        return Some(if n == 0 {
            Val::Str(vp.arg0.clone())
        } else {
            match vp.args.get(n - 1) {
                Some(v) => Val::Str(v.clone()),
                None => Val::Unset,
            }
        });
    }
    if name.len() == 1 && b"?#$!-@*".contains(&name[0]) {
        return Some(special(vp, name[0]));
    }
    if is_name(name) {
        let name = std::str::from_utf8(name).ok()?;
        return Some(match vp.var(name) {
            Some(v) => Val::Str(v.to_vec()),
            None => Val::Unset,
        });
    }
    None
}

fn ifs_first(vp: &Vp) -> Vec<u8> {
    match vp.var("IFS") {
        Some(v) => v.first().map(|b| vec![*b]).unwrap_or_default(),
        None => b" ".to_vec(),
    }
}

async fn param(
    vp: &mut Vp,
    code: &Rc<Code>,
    p: &psh_syntax::Param,
    part: &WordPart,
    quoted: bool,
    f: &mut Fields,
) -> Result<(), Unwind> {
    // `$!` names the job's first external: let the job start it (D1900).
    if p.name == ParamName::Special(b'!')
        && let Some(job) = vp.last_bg.clone()
    {
        crate::jobs::settle(&vp.core.sched, &job).await;
    }
    let name_text = code.ast.text(p.name_span).as_bytes();
    let bad = |vp: &Vp| {
        let msg = format!("{}: bad substitution", code.ast.text(part.span));
        let vp = vp.clone();
        async move {
            vp.error(&msg).await;
            Err::<(), Unwind>(Unwind::Abort(1))
        }
    };
    let is_list = matches!(p.index, Some(Subscript::At | Subscript::Star));
    // `${!a[@]}`: the keys of an array.
    if p.prefix == ParamPrefix::Indirect && is_list && p.name == ParamName::Var {
        let name = vp.resolve(code.ast.text(p.name_span)).into_owned();
        let keys = vp.vars.get(&name).map(|v| v.keys()).unwrap_or_default();
        let star = matches!(p.index, Some(Subscript::Star));
        return emit_val(vp, f, Val::List(keys, star), quoted);
    }
    // `${x:o:l}` on arrays and `$@` slices by index, so it reads the source itself.
    if let ParamOp::Substring { offset, length } = &p.op
        && p.prefix == ParamPrefix::None
        && (is_list || matches!(p.name, ParamName::Special(b'@' | b'*')))
    {
        let off = arith_word(vp, code, offset).await?;
        let len = match length {
            Some(w) => Some(arith_word(vp, code, w).await?),
            None => None,
        };
        let star = matches!(p.index, Some(Subscript::Star)) || p.name == ParamName::Special(b'*');
        let items = if p.name == ParamName::Var {
            let name = vp.resolve(code.ast.text(p.name_span)).into_owned();
            slice_array(vp, &name, off, len)
        } else {
            let mut all = vec![(0, vp.arg0.clone())];
            all.extend(
                vp.args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (i as i64 + 1, a.clone())),
            );
            slice_items(&all, off, len)
        };
        let items = match items {
            Ok(items) => items,
            Err(msg) => {
                vp.error(&format!("{msg}: substring expression < 0")).await;
                return Err(Unwind::Abort(1));
            }
        };
        return emit_val(vp, f, Val::List(items, star), quoted);
    }
    let mut val = match p.name {
        // `$LINENO`: the line of the command running (in a trap, the line it fired at).
        ParamName::Var if code.ast.text(p.name_span) == "LINENO" && p.index.is_none() => {
            Val::Str(vp.line.to_string().into_bytes())
        }
        ParamName::Var => match vp.target(code.ast.text(p.name_span)) {
            // A nameref to an element (`declare -n r='a[2]'`).
            Ok((name, Some(sub))) if p.index.is_none() => match &sub[..] {
                b"@" | b"*" => {
                    let values = vp.vars.get(&name).map(|v| v.values()).unwrap_or_default();
                    Val::List(values, sub == b"*")
                }
                _ => {
                    let sub = expand_sub_text(vp, &sub).await?;
                    element(vp, &name, &sub).await?
                }
            },
            // A subscript on a reference to an element: nothing (bash).
            Ok((_, Some(_))) => match p.index {
                Some(Subscript::At) => Val::List(Vec::new(), false),
                Some(Subscript::Star) => Val::List(Vec::new(), true),
                _ => Val::Unset,
            },
            Ok((name, None)) => lookup_var(vp, code, &name, &p.index).await?,
            Err(msg) => {
                vp.error(&format!("warning: {msg}")).await;
                Val::Unset
            }
        },
        ParamName::Positional | ParamName::Special(_) => {
            by_name(vp, name_text).unwrap_or(Val::Unset)
        }
    };

    // `set -u` (D1282): an unset parameter is an error unless an operator handles it.
    if matches!(val, Val::Unset)
        && p.prefix != ParamPrefix::Indirect
        && !matches!(p.op, ParamOp::Default { .. } | ParamOp::Names { .. })
        && vp.opts.get(SetOpt::Nounset)
    {
        let mut name = String::new();
        if !p.braced && p.name != ParamName::Var {
            name.push('$');
        }
        name.push_str(&String::from_utf8_lossy(name_text));
        if let Some(Subscript::Expr(w)) = &p.index {
            name.push_str(&format!("[{}]", code.ast.text(w.span)));
        }
        return unbound(vp, &name).await;
    }
    match p.prefix {
        ParamPrefix::None => {}
        ParamPrefix::Length => {
            let utf8 = vp.utf8();
            let n = match &val {
                Val::Unset => 0,
                Val::Str(s) => pattern::units(s, utf8).0.len(),
                Val::List(l, _) => l.len(),
            };
            emit(f, n.to_string().as_bytes(), quoted);
            return Ok(());
        }
        ParamPrefix::Indirect => {
            if let ParamOp::Names { star } = p.op {
                let prefix = code.ast.text(p.name_span);
                let mut names: Vec<Vec<u8>> = vp
                    .vars
                    .iter()
                    .filter(|(n, v)| n.starts_with(prefix) && v.value != crate::vars::Value::Unset)
                    .map(|(n, _)| n.as_bytes().to_vec())
                    .collect();
                names.sort();
                return emit_val(vp, f, Val::List(names, star), quoted);
            }
            // A nameref: `${!ref}` is the name it refers to.
            if p.name == ParamName::Var
                && let Some(v) = vp
                    .vars
                    .get(code.ast.text(p.name_span))
                    .filter(|v| v.nameref)
            {
                // Through a chain of namerefs: the last name.
                let first = v.get().map(<[u8]>::to_vec);
                val = match vp.resolve_checked(code.ast.text(p.name_span)) {
                    Ok(n) if first.is_some() => Val::Str(n.as_bytes().to_vec()),
                    _ => first.map_or(Val::Unset, Val::Str),
                };
            } else {
                let target = match &val {
                    Val::Str(s) => s.clone(),
                    Val::Unset => {
                        vp.error(&format!(
                            "{}: invalid indirect expansion",
                            String::from_utf8_lossy(name_text)
                        ))
                        .await;
                        return Err(Unwind::Abort(1));
                    }
                    Val::List(..) => return bad(vp).await,
                };
                val = match indirect(vp, code, &target).await? {
                    Some(Val::Unset)
                        if vp.opts.get(SetOpt::Nounset)
                            && !matches!(p.op, ParamOp::Default { .. }) =>
                    {
                        return unbound(vp, &format!("!{}", String::from_utf8_lossy(name_text)))
                            .await;
                    }
                    Some(v) => v,
                    None => {
                        vp.error(&format!(
                            "{}: invalid variable name",
                            String::from_utf8_lossy(&target)
                        ))
                        .await;
                        return Err(Unwind::Abort(1));
                    }
                };
            }
        }
    }

    let utf8 = vp.utf8();
    match &p.op {
        ParamOp::None => emit_val(vp, f, val, quoted),
        ParamOp::Default { kind, colon, word } => {
            let is_set = match &val {
                Val::Unset => false,
                Val::Str(s) => !(*colon && s.is_empty()),
                Val::List(l, _) => {
                    !l.is_empty() && !(*colon && l.iter().all(|s| s.is_empty()) && l.len() == 1)
                }
            };
            match kind {
                DefaultKind::Use => {
                    if is_set {
                        emit_val(vp, f, val, quoted)
                    } else {
                        expand_operand(vp, code, word, quoted, f).await
                    }
                }
                DefaultKind::Alternative => {
                    if is_set {
                        expand_operand(vp, code, word, quoted, f).await
                    } else {
                        if quoted {
                            f.quoted(b"");
                        }
                        Ok(())
                    }
                }
                DefaultKind::Assign => {
                    if is_set {
                        return emit_val(vp, f, val, quoted);
                    }
                    if p.name != ParamName::Var
                        || matches!(p.index, Some(Subscript::At | Subscript::Star))
                    {
                        vp.error(&format!(
                            "${}: cannot assign in this way",
                            String::from_utf8_lossy(name_text)
                        ))
                        .await;
                        return Err(Unwind::Abort(1));
                    }
                    let (v, _) = expand_single(vp, code, word, Ctx::Word).await?;
                    let name = code.ast.text(p.name_span);
                    let r = match &p.index {
                        Some(Subscript::Expr(w)) => {
                            let (sub, _) = expand_single(vp, code, w, Ctx::Word).await?;
                            vp.assign_elem(name, &sub, v.clone(), false)
                        }
                        _ => vp.assign(name, v.clone()),
                    };
                    if let Err(msg) = r {
                        vp.error(&msg).await;
                        return Err(Unwind::Abort(1));
                    }
                    // The value as assigned (`-i`, `-u`, … applied).
                    let v = match &p.index {
                        None => vp.var(name).map(<[u8]>::to_vec).unwrap_or(v),
                        _ => v,
                    };
                    emit(f, &v, quoted);
                    Ok(())
                }
                DefaultKind::Error => {
                    if is_set {
                        return emit_val(vp, f, val, quoted);
                    }
                    let (msg, _) = expand_single(vp, code, word, Ctx::Word).await?;
                    let msg = if msg.is_empty() {
                        if *colon {
                            "parameter null or not set".to_string()
                        } else {
                            "parameter not set".to_string()
                        }
                    } else {
                        String::from_utf8_lossy(&msg).into_owned()
                    };
                    vp.error(&format!("{}: {msg}", String::from_utf8_lossy(name_text)))
                        .await;
                    Err(Unwind::Exit(expansion_exit_status(vp)))
                }
            }
        }
        ParamOp::RemovePrefix { longest, pattern } | ParamOp::RemoveSuffix { longest, pattern } => {
            let suffix = matches!(p.op, ParamOp::RemoveSuffix { .. });
            let pat = expand_pattern(vp, code, pattern).await?;
            let val = map_val(val, |s| {
                pattern::remove_affix(s, &pat, suffix, *longest, utf8)
            });
            emit_val(vp, f, val, quoted)
        }
        ParamOp::Replace {
            kind,
            pattern,
            replacement,
        } => {
            let pat = expand_pattern(vp, code, pattern).await?;
            let rep = match replacement {
                Some(w) => expand_single(vp, code, w, Ctx::Word).await?.0,
                None => Vec::new(),
            };
            let mode = match kind {
                ReplaceKind::First => ReplaceMode::First,
                ReplaceKind::All => ReplaceMode::All,
                ReplaceKind::Prefix => ReplaceMode::Prefix,
                ReplaceKind::Suffix => ReplaceMode::Suffix,
            };
            let val = map_val(val, |s| {
                if pat.is_empty() && mode != ReplaceMode::Prefix && mode != ReplaceMode::Suffix {
                    s.to_vec()
                } else {
                    pattern::replace(s, &pat, &rep, mode, utf8)
                }
            });
            emit_val(vp, f, val, quoted)
        }
        ParamOp::Case { kind, pattern } => {
            let pat = match pattern {
                Some(w) => expand_pattern(vp, code, w).await?,
                None => b"?".to_vec(),
            };
            let pat = Pattern::new(&pat, utf8);
            let val = map_val(val, |s| change_case(s, *kind, &pat, utf8));
            emit_val(vp, f, val, quoted)
        }
        ParamOp::Substring { offset, length } => {
            let off = arith_word(vp, code, offset).await?;
            let len = match length {
                Some(w) => Some(arith_word(vp, code, w).await?),
                None => None,
            };
            let mut err = None;
            let val = map_val(val, |s| match substring(s, off, len, utf8) {
                Ok(v) => v,
                Err(e) => {
                    err = Some(e);
                    Vec::new()
                }
            });
            if let Some(e) = err {
                vp.error(&format!("{e}: substring expression < 0")).await;
                return Err(Unwind::Abort(1));
            }
            emit_val(vp, f, val, quoted)
        }
        ParamOp::Transform(op) => {
            let op = *op;
            let any = Pattern::new(b"?", utf8);
            match op {
                b'Q' | b'E' | b'U' | b'L' | b'u' | b'P' => {
                    let val = match (&val, op) {
                        // `@Q` of nothing is nothing, not `''`.
                        (Val::Unset, _) => Val::Unset,
                        _ => map_val(val, |s| match op {
                            b'Q' => quote(s),
                            b'E' => psh_syntax::decode_ansi_c(s),
                            b'U' => change_case(s, CaseKind::UpperAll, &any, utf8),
                            b'L' => change_case(s, CaseKind::LowerAll, &any, utf8),
                            b'P' => prompt(vp, s),
                            _ => change_case(s, CaseKind::UpperFirst, &any, utf8),
                        }),
                    };
                    emit_val(vp, f, val, quoted)
                }
                b'A' | b'a' | b'K' | b'k' => {
                    let val = transform_var(vp, code, p, op, val);
                    emit_val(vp, f, val, quoted)
                }
                _ => bad(vp).await,
            }
        }
        ParamOp::Names { .. } => bad(vp).await,
    }
}

/// `${x@A}`, `@a`, `@K`, `@k`: transformations that need the variable, not only
/// its value.
fn transform_var(vp: &Vp, code: &Rc<Code>, p: &psh_syntax::Param, op: u8, val: Val) -> Val {
    use crate::builtins::declare::{declaration, value_quote};
    let var = match p.name {
        ParamName::Var => vp.var_ref(code.ast.text(p.name_span)).cloned(),
        _ => None,
    };
    let name = match p.name {
        ParamName::Var => vp.resolve(code.ast.text(p.name_span)).into_owned(),
        _ => String::new(),
    };
    let list = matches!(p.index, Some(Subscript::At | Subscript::Star));
    let star = matches!(p.index, Some(Subscript::Star));
    match op {
        b'a' => {
            let flags = var.as_ref().map_or(String::new(), |v| {
                let f = v.flags();
                if f == "-" { String::new() } else { f }
            });
            match val {
                Val::List(items, st) => Val::List(
                    items.iter().map(|_| flags.clone().into_bytes()).collect(),
                    st,
                ),
                Val::Unset if var.is_none() => Val::Unset,
                _ => Val::Str(flags.into_bytes()),
            }
        }
        b'A' => {
            let Some(var) = var else {
                return match val {
                    Val::Str(s) if p.name != ParamName::Var => Val::Str(quote(&s)),
                    _ => Val::Unset,
                };
            };
            if list && var.is_array() {
                let mut d = declaration(&name, &var);
                d.pop();
                return Val::Str(d);
            }
            let Val::Str(s) = val else { return Val::Unset };
            let flags = var.flags();
            let mut out = Vec::new();
            if flags != "-" {
                out.extend_from_slice(format!("declare -{flags} ").as_bytes());
            }
            out.extend_from_slice(name.as_bytes());
            out.push(b'=');
            out.extend_from_slice(&quote(&s));
            Val::Str(out)
        }
        _ => {
            // `@K`: key-value pairs as one quoted text; `@k`: separate words.
            let pairs: Vec<(Vec<u8>, Vec<u8>)> = match (&var, list) {
                (Some(v), true) if v.is_array() => v.keys().into_iter().zip(v.values()).collect(),
                _ => {
                    return match val {
                        Val::Str(s) if op == b'K' => Val::Str(quote(&s)),
                        Val::List(items, st) if op == b'K' => {
                            Val::List(items.iter().map(|s| quote(s)).collect(), st)
                        }
                        other => other,
                    };
                }
            };
            if op == b'k' {
                return Val::List(pairs.into_iter().flat_map(|(k, v)| [k, v]).collect(), star);
            }
            let assoc = var.as_ref().is_some_and(|v| v.is_assoc());
            let mut out = Vec::new();
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                out.extend_from_slice(k);
                out.push(b' ');
                out.extend_from_slice(&value_quote(v));
            }
            if assoc {
                out.push(b' ');
            }
            Val::Str(out)
        }
    }
}

/// `${x@P}`: the prompt escapes of a non-interactive shell.
fn prompt(vp: &Vp, s: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if s[i] != b'\\' || i + 1 >= s.len() {
            out.push(s[i]);
            i += 1;
            continue;
        }
        let c = s[i + 1];
        i += 2;
        match c {
            b'a' => out.push(7),
            b'e' => out.push(0x1b),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b'\\' => out.push(b'\\'),
            b'$' => out.push(b'$'),
            b's' => out.extend_from_slice(vp.msg_name().as_bytes()),
            b'u' => out.extend_from_slice(vp.var("USER").unwrap_or_default()),
            b'w' | b'W' => {
                let pwd = vp.var("PWD").unwrap_or_default().to_vec();
                let home = vp.var("HOME").unwrap_or_default().to_vec();
                let shown = if !home.is_empty() && pwd.starts_with(&home) {
                    let mut v = b"~".to_vec();
                    v.extend_from_slice(&pwd[home.len()..]);
                    v
                } else {
                    pwd
                };
                if c == b'W' {
                    let base = shown
                        .rsplit(|&b| b == b'/')
                        .next()
                        .unwrap_or_default()
                        .to_vec();
                    out.extend_from_slice(if base.is_empty() { &shown } else { &base });
                } else {
                    out.extend_from_slice(&shown);
                }
            }
            b'[' | b']' => {}
            _ => {
                out.push(b'\\');
                out.push(c);
            }
        }
    }
    out
}

/// The value of a variable for `${name}`, `${name[sub]}`, `${name[@]}`.
async fn lookup_var(
    vp: &mut Vp,
    code: &Rc<Code>,
    name: &str,
    index: &Option<Subscript>,
) -> Result<Val, Unwind> {
    match index {
        None => Ok(vp
            .vars
            .value(name)
            .map_or(Val::Unset, |v| Val::Str(v.to_vec()))),
        Some(Subscript::At) | Some(Subscript::Star) => {
            let values = vp.vars.get(name).map(|v| v.values()).unwrap_or_default();
            Ok(Val::List(values, matches!(index, Some(Subscript::Star))))
        }
        Some(Subscript::Expr(w)) => {
            let (sub, _) = expand_single(vp, code, w, Ctx::Word).await?;
            element(vp, name, &sub).await
        }
    }
}

/// `name[sub]` with `sub` expanded: an associative key or an arithmetic index.
async fn element(vp: &mut Vp, name: &str, sub: &[u8]) -> Result<Val, Unwind> {
    match vp.get_elem(name, sub) {
        Ok(v) => Ok(v.map_or(Val::Unset, Val::Str)),
        Err(msg) if msg.ends_with("bad array subscript") => {
            // A reference before the start: an error, but the word goes on empty.
            vp.error(&msg).await;
            Ok(Val::Unset)
        }
        Err(msg) => {
            vp.error(&msg).await;
            Err(Unwind::Abort(1))
        }
    }
}

/// `${!name}`: the value of the variable (or `a[sub]`, `a[@]`, positional) named by
/// `target`; `None` if it is not a valid name.
async fn indirect(vp: &mut Vp, code: &Rc<Code>, target: &[u8]) -> Result<Option<Val>, Unwind> {
    if let Some(open) = target.iter().position(|&b| b == b'[')
        && target.ends_with(b"]")
        && is_name(&target[..open])
    {
        let name = vp
            .resolve(&String::from_utf8_lossy(&target[..open]))
            .into_owned();
        let sub = &target[open + 1..target.len() - 1];
        let _ = code;
        return Ok(Some(match sub {
            b"@" | b"*" => {
                let values = vp.vars.get(&name).map(|v| v.values()).unwrap_or_default();
                Val::List(values, sub == b"*")
            }
            _ => element(vp, &name, sub).await?,
        }));
    }
    if is_name(target) {
        let name = String::from_utf8_lossy(target).into_owned();
        return Ok(Some(
            vp.var(&name).map_or(Val::Unset, |v| Val::Str(v.to_vec())),
        ));
    }
    Ok(by_name(vp, target))
}

/// An arithmetic word (offsets, lengths): expanded, then evaluated.
async fn arith_word(vp: &mut Vp, code: &Rc<Code>, w: &Word) -> Result<i64, Unwind> {
    let (text, _) = expand_single(vp, code, w, Ctx::Word).await?;
    match crate::arith::eval(vp, &text) {
        Ok(v) => Ok(v),
        Err(msg) => {
            vp.error(&msg).await;
            Err(Unwind::Abort(1))
        }
    }
}

/// `${a[@]:off:len}`: elements by index (`off` counts from after the last index when
/// negative), at most `len` of them.
fn slice_array(vp: &Vp, name: &str, off: i64, len: Option<i64>) -> Result<Vec<Vec<u8>>, String> {
    let items: Vec<(i64, Vec<u8>)> = match vp.vars.get(name).map(|v| &v.value) {
        Some(crate::vars::Value::Indexed(m)) => m.iter().map(|(k, v)| (*k, v.clone())).collect(),
        // bash numbers the elements of an associative array from 1, like positional
        // parameters, and offset 0 starts at the first.
        Some(crate::vars::Value::Assoc(a)) => {
            let items: Vec<(i64, Vec<u8>)> = a
                .iter()
                .enumerate()
                .map(|(i, (_, v))| (i as i64 + 1, v.to_vec()))
                .collect();
            return slice_items(&items, if off == 0 { 1 } else { off }, len);
        }
        Some(crate::vars::Value::Str(s)) => vec![(0, s.clone())],
        _ => Vec::new(),
    };
    slice_items(&items, off, len)
}

fn slice_items(
    items: &[(i64, Vec<u8>)],
    off: i64,
    len: Option<i64>,
) -> Result<Vec<Vec<u8>>, String> {
    if let Some(l) = len
        && l < 0
    {
        return Err(l.to_string());
    }
    let start = if off < 0 {
        // One past the last index; bash's arithmetic wraps when that is i64::MAX + 1,
        // and a negative offset then selects nothing.
        let end = items.last().map_or(0, |(k, _)| k.wrapping_add(1));
        if end < 0 {
            return Ok(Vec::new());
        }
        end.saturating_add(off)
    } else {
        off
    };
    if off < 0 && start < 0 {
        return Ok(Vec::new());
    }
    let it = items
        .iter()
        .filter(|(k, _)| *k >= start)
        .map(|(_, v)| v.clone());
    Ok(match len {
        Some(l) => it.take(l as usize).collect(),
        None => it.collect(),
    })
}

/// `${x:off:len}` on a string, by chars (UTF-8 locale) or bytes.
fn substring(s: &[u8], off: i64, len: Option<i64>, utf8: bool) -> Result<Vec<u8>, String> {
    let (u, offs) = pattern::units(s, utf8);
    let n = u.len() as i64;
    let start = if off < 0 { n.saturating_add(off) } else { off };
    if start < 0 || start > n {
        return Ok(Vec::new());
    }
    let end = match len {
        None => n,
        Some(l) if l < 0 => {
            let e = n.saturating_add(l);
            if e < start {
                return Err(l.to_string());
            }
            e
        }
        Some(l) => start.saturating_add(l).min(n),
    };
    Ok(s[offs[start as usize]..offs[end as usize]].to_vec())
}

/// `${x@Q}`: quoted for reuse as shell input.
pub(crate) fn quote(s: &[u8]) -> Vec<u8> {
    let mut out = vec![b'\''];
    for &b in s {
        if b == b'\'' {
            out.extend_from_slice(b"'\\''");
        } else {
            out.push(b);
        }
    }
    out.push(b'\'');
    out
}

fn change_case(s: &[u8], kind: CaseKind, pat: &Pattern, utf8: bool) -> Vec<u8> {
    let text = String::from_utf8_lossy(s);
    let mut out = String::new();
    let all = matches!(
        kind,
        CaseKind::UpperAll | CaseKind::LowerAll | CaseKind::ToggleAll
    );
    for (i, ch) in text.chars().enumerate() {
        let mut buf = [0u8; 4];
        let applies = (all || i == 0) && pat.matches(ch.encode_utf8(&mut buf).as_bytes(), utf8);
        if !applies {
            out.push(ch);
            continue;
        }
        // Like towupper/towlower: only one-char mappings (ß stays ß).
        let one = |m: &mut dyn Iterator<Item = char>| {
            let v: Vec<char> = m.collect();
            if v.len() == 1 { v[0] } else { ch }
        };
        let up = || one(&mut ch.to_uppercase());
        let down = || one(&mut ch.to_lowercase());
        out.push(match kind {
            CaseKind::UpperFirst | CaseKind::UpperAll => up(),
            CaseKind::LowerFirst | CaseKind::LowerAll => down(),
            CaseKind::ToggleFirst | CaseKind::ToggleAll => {
                if ch.is_uppercase() {
                    down()
                } else {
                    up()
                }
            }
        });
    }
    out.into_bytes()
}

fn map_val(val: Val, mut f: impl FnMut(&[u8]) -> Vec<u8>) -> Val {
    match val {
        Val::Unset => Val::Unset,
        Val::Str(s) => Val::Str(f(&s)),
        Val::List(l, star) => Val::List(l.iter().map(|s| f(s)).collect(), star),
    }
}

fn emit_val(vp: &Vp, f: &mut Fields, val: Val, quoted: bool) -> Result<(), Unwind> {
    match val {
        Val::Unset => {
            if quoted {
                f.quoted(b"");
            }
        }
        Val::Str(s) => emit(f, &s, quoted),
        Val::List(items, star) => {
            if star && (quoted || !f.split) {
                let sep = ifs_first(vp);
                let joined = items.join(&sep[..]);
                emit(f, &joined, quoted);
            } else {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.field_break(quoted);
                    }
                    emit(f, item, quoted);
                }
            }
        }
    }
    Ok(())
}

/// The word of `${x:-word}` and friends, in the quoting context of the expansion.
async fn expand_operand(
    vp: &mut Vp,
    code: &Rc<Code>,
    word: &Word,
    quoted: bool,
    f: &mut Fields,
) -> Result<(), Unwind> {
    if quoted && word.parts.is_empty() {
        f.quoted(b"");
        return Ok(());
    }
    expand_parts(vp, code, &word.parts, quoted, f).await
}

/// Pathname expansion of the fields with unquoted pattern chars; `set -f`, `nullglob`
/// and `failglob` (D1280).
async fn glob_fields(vp: &mut Vp, f: &mut Fields) -> Result<(), Unwind> {
    if !f.out.iter().any(|x| x.glob) || vp.opts.get(SetOpt::Noglob) {
        return Ok(());
    }
    let fields = std::mem::take(&mut f.out);
    for field in fields {
        if !field.glob {
            f.out.push(field);
            continue;
        }
        let matches = glob(vp, &field.pat).await;
        if matches.is_empty() && is_pattern(&field.pat) {
            if vp.opts.shopt(Shopt::Failglob) {
                vp.error(&format!(
                    "no match: {}",
                    String::from_utf8_lossy(&field.text)
                ))
                .await;
                return Err(Unwind::Abort(1));
            }
            if !vp.opts.shopt(Shopt::Nullglob) {
                f.out.push(field);
            }
        } else if matches.is_empty() {
            f.out.push(field);
        } else {
            for m in matches {
                f.out.push(Field {
                    pat: Vec::new(),
                    text: m,
                    glob: false,
                    started: true,
                });
            }
        }
    }
    Ok(())
}

/// Whether a glob pattern can match anything but itself: an unescaped `*` or `?`, or a
/// `[` with a `]` after it (a lone `[` is an ordinary char). For `nullglob`/`failglob`.
fn is_pattern(pat: &[u8]) -> bool {
    let mut i = 0;
    while i < pat.len() {
        match pat[i] {
            b'\\' => i += 2,
            b'*' | b'?' => return true,
            b'[' if pat[i + 1..].contains(&b']') => return true,
            _ => i += 1,
        }
    }
    false
}

/// Unescapes a pattern component that has no pattern chars.
fn unescape(p: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(p.len());
    let mut i = 0;
    while i < p.len() {
        if p[i] == b'\\' && i + 1 < p.len() {
            out.push(p[i + 1]);
            i += 2;
        } else {
            out.push(p[i]);
            i += 1;
        }
    }
    out
}

/// The paths matching a pattern, sorted; empty if none.
pub(crate) async fn glob(vp: &Vp, pat: &[u8]) -> Vec<Vec<u8>> {
    let utf8 = vp.utf8();
    let absolute = pat.first() == Some(&b'/');
    let comps: Vec<&[u8]> = pat
        .split(|&b| b == b'/')
        .filter(|c| !c.is_empty())
        .collect();
    // (shown path, absolute path)
    let mut cur: Vec<(Vec<u8>, Vec<u8>)> = vec![if absolute {
        (b"/".to_vec(), b"/".to_vec())
    } else {
        (Vec::new(), vp.cwd.clone())
    }];
    for (i, comp) in comps.iter().enumerate() {
        let last = i + 1 == comps.len();
        let mut next = Vec::new();
        let join = |a: &[u8], b: &[u8]| {
            let mut v = a.to_vec();
            if !v.is_empty() && v.last() != Some(&b'/') {
                v.push(b'/');
            }
            v.extend_from_slice(b);
            v
        };
        if !pattern::has_glob_chars(comp) {
            let name = unescape(comp);
            for (shown, abs) in &cur {
                let p = join(abs, &name);
                if last
                    || vp
                        .host()
                        .stat(&p, true)
                        .await
                        .is_ok_and(|m| m.kind == crate::host::FileKind::Dir)
                {
                    next.push((join(shown, &name), p));
                }
            }
            if last {
                // The final literal component must exist.
                let mut kept = Vec::new();
                for (s, p) in next {
                    if vp.host().stat(&p, false).await.is_ok() {
                        kept.push((s, p));
                    }
                }
                next = kept;
            }
        } else {
            let p = Pattern::new(comp, utf8);
            let dot_ok = comp.first() == Some(&b'.');
            for (shown, abs) in &cur {
                let Ok(mut entries) = vp.host().read_dir(abs).await else {
                    continue;
                };
                entries.sort_by(|a, b| a.name.cmp(&b.name));
                for e in entries {
                    if e.name.first() == Some(&b'.') && !dot_ok {
                        continue;
                    }
                    if !p.matches(&e.name, utf8) {
                        continue;
                    }
                    let full = join(abs, &e.name);
                    if !last
                        && !vp
                            .host()
                            .stat(&full, true)
                            .await
                            .is_ok_and(|m| m.kind == crate::host::FileKind::Dir)
                    {
                        continue;
                    }
                    next.push((join(shown, &e.name), full));
                }
            }
        }
        cur = next;
        if cur.is_empty() {
            return Vec::new();
        }
    }
    let mut out: Vec<Vec<u8>> = cur.into_iter().map(|(s, _)| s).collect();
    if pat.ends_with(b"/") {
        for o in &mut out {
            o.push(b'/');
        }
    }
    out.sort();
    out
}

/// Assignment values, for exec.
pub(crate) async fn assign_value(
    vp: &mut Vp,
    code: &Rc<Code>,
    word: &Word,
) -> Result<(Vec<u8>, Option<i32>), Unwind> {
    expand_single(vp, code, word, Ctx::Assign).await
}

/// The elements of a compound assignment `(…)`: `[key]=value` elements are expanded
/// like assignment values; plain elements like command words (split, globbed), each
/// field an element.
pub(crate) async fn expand_array_lit(
    vp: &mut Vp,
    code: &Rc<Code>,
    lit: &psh_syntax::ArrayLit,
) -> Result<(Vec<crate::arrays::ArrayItem>, Option<i32>), Unwind> {
    let mut items = Vec::new();
    let mut status = None;
    for e in &lit.elems {
        match &e.key {
            Some(k) => {
                let (key, st1) = expand_single(vp, code, k, Ctx::Word).await?;
                let (value, st2) = expand_single(vp, code, &e.value, Ctx::Assign).await?;
                status = st2.or(st1).or(status);
                items.push(crate::arrays::ArrayItem {
                    key: Some(key),
                    value,
                    append: e.append,
                });
            }
            None => {
                let f = expand_words(vp, code, std::slice::from_ref(&e.value)).await?;
                status = f.subst_status.or(status);
                for field in f.out {
                    items.push(crate::arrays::ArrayItem {
                        key: None,
                        value: field.text,
                        append: false,
                    });
                }
            }
        }
    }
    Ok((items, status))
}

/// Whether a word starts like an assignment: `name=`, `name+=` or `name[`.
fn assignment_like(code: &Code, w: &Word) -> bool {
    let Some(p) = w.parts.first() else {
        return false;
    };
    if !matches!(p.kind, Part::Lit) {
        return false;
    }
    let t = code.ast.bytes(p.span);
    let end = t
        .iter()
        .position(|&b| b == b'=' || b == b'[' || b == b'+')
        .unwrap_or(t.len());
    end > 0
        && end < t.len()
        && is_name(&t[..end])
        && (t[end] != b'+' || t.get(end + 1) == Some(&b'='))
}

/// The arguments of a declaration builtin (`declare a=(…) b=$x c`): assignments are
/// expanded like assignment values (no splitting), `name=(…)` into array items, the
/// rest like command words. Returns the builtin's name, its arguments and the status of
/// the last command substitution.
pub(crate) async fn expand_decl_args(
    vp: &mut Vp,
    code: &Rc<Code>,
    words: &[Word],
) -> Result<(Vec<u8>, Vec<crate::builtins::declare::DeclArg>, Option<i32>), Unwind> {
    use crate::builtins::declare::DeclArg;
    let mut args = Vec::new();
    let mut status = None;
    let name = expand_single(vp, code, &words[0], Ctx::Word).await?.0;
    for w in &words[1..] {
        if let Some((i, Part::Array(lit))) =
            w.parts.iter().enumerate().find_map(|(i, p)| match &p.kind {
                Part::Array(l) => Some((i, Part::Array(l.clone()))),
                _ => None,
            })
        {
            // `name=(…)` / `name+=(…)`: the text before the parentheses names it.
            let mut lhs = Vec::new();
            for p in &w.parts[..i] {
                lhs.extend_from_slice(code.ast.bytes(p.span));
            }
            let lhs = lhs.strip_suffix(b"=").unwrap_or(&lhs).to_vec();
            let (lhs, append) = match lhs.strip_suffix(b"+") {
                Some(l) => (l.to_vec(), true),
                None => (lhs, false),
            };
            let (items, st) = expand_array_lit(vp, code, &lit).await?;
            status = st.or(status);
            args.push(DeclArg::Array {
                name: lhs,
                append,
                items,
            });
        } else if assignment_like(code, w) {
            let (v, st) = expand_single(vp, code, w, Ctx::Assign).await?;
            status = st.or(status);
            args.push(DeclArg::from_word(v));
        } else {
            let f = expand_words(vp, code, std::slice::from_ref(w)).await?;
            status = f.subst_status.or(status);
            args.extend(f.out.into_iter().map(|f| DeclArg::from_word(f.text)));
        }
    }
    Ok((name, args, status))
}
