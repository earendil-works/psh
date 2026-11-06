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

use crate::escape::{Dialect, decode};
use crate::exec;
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
                            None => vec![vec![Piece::Text(
                                code.ast.text(part.span).as_bytes().to_vec(),
                            )]],
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
    let step_n: i64 = match step {
        Some(s) => s.parse().ok()?,
        None => 1,
    };
    let step_abs = step_n.unsigned_abs().max(1) as i64;
    if let (Ok(a), Ok(b)) = (start.parse::<i64>(), end.parse::<i64>()) {
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
        let (a, b) = (i64::from(a[0]), i64::from(b[0]));
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
    glob_fields(vp, &mut f).await;
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
    let text = code.ast.text(part.span).as_bytes();
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
            f.quoted(c);
        }
        Part::Single => f.quoted(&text[1..text.len() - 1]),
        Part::AnsiC => {
            let inner = &text[2..text.len() - 1];
            f.quoted(&decode(inner, Dialect::AnsiC).bytes);
        }
        Part::Double(inner) | Part::Locale(inner) => {
            // `"…"` makes a field even when everything inside is empty, except that
            // `"$@"` with no parameters makes none.
            if !inner.iter().all(is_at) {
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
            return Err(Unwind::Exit(1));
        }
        Part::CmdSubst(list) => {
            let (out, st) = exec::command_subst(vp, code, *list).await?;
            f.subst_status = Some(st);
            emit(f, &out, quoted);
        }
        Part::Backtick => {
            let body = psh_syntax::backtick_body(code.ast.text(part.span), quoted);
            let (out, st) = exec::command_subst_text(vp, code, &body, part.span.start).await?;
            f.subst_status = Some(st);
            emit(f, &out, quoted);
        }
        Part::Arith(_) => {
            return unsupported(vp, "arithmetic expansion").await;
        }
        Part::Brace(_) => {
            // Only reached where brace expansion does not apply: the text as is.
            if quoted {
                f.quoted(text);
            } else {
                f.literal(text);
            }
        }
        Part::ProcSubst { .. } => return unsupported(vp, "process substitution").await,
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
            matches!(p.op, ParamOp::None)
                && p.prefix == ParamPrefix::None
                && (p.name == ParamName::Special(b'@') || matches!(p.index, Some(Subscript::At)))
        }
        _ => false,
    }
}

pub(crate) async fn unsupported<T>(vp: &Vp, what: &str) -> Result<T, Unwind> {
    vp.error(&format!("{what}: not supported yet")).await;
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
        b'!' => match vp.last_bg {
            Some(p) => Val::Str(p.to_string().into_bytes()),
            None => Val::Unset,
        },
        b'-' => Val::Str(b"hB".to_vec()),
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
    let name_text = code.ast.text(p.name_span).as_bytes();
    let bad = |vp: &Vp| {
        let msg = format!("{}: bad substitution", code.ast.text(part.span));
        let vp = vp.clone();
        async move {
            vp.error(&msg).await;
            Err::<(), Unwind>(Unwind::Exit(1))
        }
    };
    let mut val = match p.name {
        ParamName::Var => {
            let v = match vp.var(code.ast.text(p.name_span)) {
                Some(v) => Val::Str(v.to_vec()),
                None => Val::Unset,
            };
            match &p.index {
                None => v,
                Some(Subscript::At) => match v {
                    Val::Str(s) => Val::List(vec![s], false),
                    _ => Val::List(Vec::new(), false),
                },
                Some(Subscript::Star) => match v {
                    Val::Str(s) => Val::List(vec![s], true),
                    _ => Val::List(Vec::new(), true),
                },
                Some(Subscript::Expr(w)) => {
                    // Arrays are not there yet; a scalar is element 0.
                    let (idx, _) = expand_single(vp, code, w, Ctx::Word).await?;
                    if idx.trim_ascii() == b"0" {
                        v
                    } else {
                        Val::Unset
                    }
                }
            }
        }
        ParamName::Positional | ParamName::Special(_) => {
            by_name(vp, name_text).unwrap_or(Val::Unset)
        }
    };

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
                    .filter(|(n, v)| n.starts_with(prefix) && v.value.is_some())
                    .map(|(n, _)| n.as_bytes().to_vec())
                    .collect();
                names.sort();
                return emit_val(vp, f, Val::List(names, star), quoted);
            }
            let target = match &val {
                Val::Str(s) => s.clone(),
                Val::Unset => {
                    vp.error(&format!(
                        "{}: invalid indirect expansion",
                        String::from_utf8_lossy(name_text)
                    ))
                    .await;
                    return Err(Unwind::Exit(1));
                }
                Val::List(..) => return bad(vp).await,
            };
            val = match by_name(vp, &target) {
                Some(v) => v,
                None => {
                    vp.error(&format!(
                        "{}: invalid variable name",
                        String::from_utf8_lossy(&target)
                    ))
                    .await;
                    return Err(Unwind::Exit(1));
                }
            };
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
                    if p.name != ParamName::Var || p.index.is_some() {
                        vp.error(&format!(
                            "${}: cannot assign in this way",
                            String::from_utf8_lossy(name_text)
                        ))
                        .await;
                        return Err(Unwind::Exit(1));
                    }
                    let (v, _) = expand_single(vp, code, word, Ctx::Word).await?;
                    let name = code.ast.text(p.name_span);
                    if vp.vars.get(name).is_some_and(|v| v.readonly) {
                        vp.error(&format!("{name}: readonly variable")).await;
                        return Err(Unwind::Exit(1));
                    }
                    vp.set_var(name, v.clone());
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
                    Err(Unwind::Exit(if vp.core.dash_c.get() { 127 } else { 1 }))
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
        ParamOp::Substring { .. } => unsupported(vp, "substring expansion").await,
        ParamOp::Transform(op) => {
            let op = *op;
            match op {
                b'Q' | b'E' | b'U' | b'L' | b'u' => {
                    let val = map_val(val, |s| match op {
                        b'Q' => quote(s),
                        b'E' => decode(s, Dialect::AnsiC).bytes,
                        b'U' => s.to_ascii_uppercase(),
                        b'L' => s.to_ascii_lowercase(),
                        _ => {
                            let mut v = s.to_vec();
                            if let Some(c) = v.first_mut() {
                                c.make_ascii_uppercase();
                            }
                            v
                        }
                    });
                    emit_val(vp, f, val, quoted)
                }
                _ => unsupported(vp, &format!("${{…@{}}}", op as char)).await,
            }
        }
        ParamOp::Names { .. } => bad(vp).await,
    }
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
        match kind {
            CaseKind::UpperFirst | CaseKind::UpperAll => out.extend(ch.to_uppercase()),
            CaseKind::LowerFirst | CaseKind::LowerAll => out.extend(ch.to_lowercase()),
            CaseKind::ToggleFirst | CaseKind::ToggleAll => {
                if ch.is_uppercase() {
                    out.extend(ch.to_lowercase())
                } else {
                    out.extend(ch.to_uppercase())
                }
            }
        }
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

/// Pathname expansion of the fields with unquoted pattern chars.
async fn glob_fields(vp: &mut Vp, f: &mut Fields) {
    if !f.out.iter().any(|x| x.glob) {
        return;
    }
    let fields = std::mem::take(&mut f.out);
    for field in fields {
        if !field.glob {
            f.out.push(field);
            continue;
        }
        let matches = glob(vp, &field.pat).await;
        if matches.is_empty() {
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

/// The words of a declaration builtin (`export a=$x`): arguments that look like
/// assignments are expanded like assignment values, without splitting or globbing.
pub(crate) async fn expand_decl_words(
    vp: &mut Vp,
    code: &Rc<Code>,
    words: &[Word],
) -> Result<Fields, Unwind> {
    let mut f = Fields::new(vp, true);
    for (i, w) in words.iter().enumerate() {
        let assignment = i > 0
            && matches!(w.parts.first(), Some(p) if matches!(p.kind, Part::Lit) && {
                let t = code.ast.text(p.span).as_bytes();
                t.iter().position(|&b| b == b'=').is_some_and(|eq| {
                    let n = t[..eq].strip_suffix(b"+").unwrap_or(&t[..eq]);
                    is_name(n)
                })
            });
        if assignment {
            let (v, st) = expand_single(vp, code, w, Ctx::Assign).await?;
            if st.is_some() {
                f.subst_status = st;
            }
            f.out.push(Field {
                text: v,
                pat: Vec::new(),
                glob: false,
                started: true,
            });
        } else {
            let more = expand_words(vp, code, std::slice::from_ref(w)).await?;
            if more.subst_status.is_some() {
                f.subst_status = more.subst_status;
            }
            f.out.extend(more.out);
        }
    }
    Ok(f)
}
