//! The declaration builtins: `declare`/`typeset`, `local`, `export`, `readonly`
//! (D1262). Their `name=value` and `name=(…)` arguments are assignments, expanded by the
//! interpreter without word splitting, so they come in as [`DeclArg`]s rather than
//! plain words.

use crate::arrays::ArrayItem;
use crate::state::{R, Vp, is_name};
use crate::vars::{Case, Value, Var};

use super::out;

/// An argument of a declaration builtin.
#[derive(Clone, Debug)]
pub(crate) enum DeclArg {
    /// An option or a name.
    Word(Vec<u8>),
    /// `name=value`, `name+=value`, `name[sub]=value`.
    Assign {
        name: Vec<u8>,
        index: Option<Vec<u8>>,
        append: bool,
        value: Vec<u8>,
    },
    /// `name=(…)`, `name+=(…)`.
    Array {
        name: Vec<u8>,
        append: bool,
        items: Vec<ArrayItem>,
    },
}

impl DeclArg {
    /// Reads a plain word as a declaration argument (`declare "a=1"`; also the path
    /// for `builtin declare …` and the like).
    pub(crate) fn from_word(w: Vec<u8>) -> DeclArg {
        if w.first().is_some_and(|&b| b == b'-' || b == b'+') {
            return DeclArg::Word(w);
        }
        let Some(eq) = w.iter().position(|&b| b == b'=') else {
            return DeclArg::Word(w);
        };
        let (lhs, append) = match w[..eq].strip_suffix(b"+") {
            Some(l) => (l, true),
            None => (&w[..eq], false),
        };
        let (name, index) = match lhs.iter().position(|&b| b == b'[') {
            Some(i) if lhs.ends_with(b"]") => {
                (lhs[..i].to_vec(), Some(lhs[i + 1..lhs.len() - 1].to_vec()))
            }
            _ => (lhs.to_vec(), None),
        };
        DeclArg::Assign {
            name,
            index,
            append,
            value: w[eq + 1..].to_vec(),
        }
    }

    fn name(&self) -> &[u8] {
        match self {
            DeclArg::Word(w) => w,
            DeclArg::Assign { name, .. } | DeclArg::Array { name, .. } => name,
        }
    }

    /// The argument as written, for messages.
    fn shown(&self) -> String {
        match self {
            DeclArg::Word(w) => String::from_utf8_lossy(w).into_owned(),
            DeclArg::Assign {
                name,
                index,
                append,
                value,
            } => {
                let mut s = String::from_utf8_lossy(name).into_owned();
                if let Some(i) = index {
                    s.push('[');
                    s.push_str(&String::from_utf8_lossy(i));
                    s.push(']');
                    // bash names the element only.
                    return s;
                }
                if *append {
                    s.push('+');
                }
                s.push('=');
                s.push_str(&String::from_utf8_lossy(value));
                s
            }
            DeclArg::Array { name, .. } => String::from_utf8_lossy(name).into_owned(),
        }
    }
}

/// Which builtin runs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    Declare,
    Typeset,
    Local,
    Export,
    Readonly,
}

impl Which {
    pub(crate) fn from_name(name: &[u8]) -> Option<Which> {
        Some(match name {
            b"declare" => Which::Declare,
            b"typeset" => Which::Typeset,
            b"local" => Which::Local,
            b"export" => Which::Export,
            b"readonly" => Which::Readonly,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Which::Declare => "declare",
            Which::Typeset => "typeset",
            Which::Local => "local",
            Which::Export => "export",
            Which::Readonly => "readonly",
        }
    }
}

/// Attributes to set (`-x`) or clear (`+x`).
#[derive(Default)]
struct Opts {
    indexed: bool,
    assoc: bool,
    integer: Option<bool>,
    lower: Option<bool>,
    upper: Option<bool>,
    nameref: Option<bool>,
    readonly: bool,
    export: Option<bool>,
    global: bool,
    print: bool,
    funcs: bool,
    func_names: bool,
    /// Any attribute given (filters `declare -p` without names).
    any: bool,
}

/// The entry point for the five builtins with already expanded arguments.
pub(crate) async fn run(vp: &mut Vp, which: Which, args: Vec<DeclArg>) -> R {
    let bname = which.name();
    if which == Which::Local && vp.frames.is_empty() {
        vp.error("local: can only be used in a function").await;
        return Ok(1);
    }
    let mut o = Opts::default();
    match which {
        Which::Export => o.export = Some(true),
        Which::Readonly => o.readonly = true,
        _ => {}
    }
    let mut i = 0;
    while i < args.len() {
        let DeclArg::Word(w) = &args[i] else { break };
        if w.len() < 2 || (w[0] != b'-' && w[0] != b'+') {
            break;
        }
        if w == b"--" {
            i += 1;
            break;
        }
        let on = w[0] == b'-';
        for &c in &w[1..] {
            let ok = match (which, c) {
                (_, b'p') => {
                    o.print = true;
                    true
                }
                (Which::Export, b'n') => {
                    o.export = Some(!on);
                    true
                }
                (Which::Export | Which::Readonly, b'f') => {
                    o.funcs = true;
                    true
                }
                (Which::Export | Which::Readonly, b'a' | b'A') => {
                    if c == b'a' {
                        o.indexed = on;
                    } else {
                        o.assoc = on;
                    }
                    true
                }
                (Which::Export | Which::Readonly, _) => false,
                (_, b'a') => {
                    o.indexed |= on;
                    true
                }
                (_, b'A') => {
                    o.assoc |= on;
                    true
                }
                (_, b'i') => {
                    o.integer = Some(on);
                    true
                }
                (_, b'l') => {
                    o.lower = Some(on);
                    true
                }
                (_, b'u') => {
                    o.upper = Some(on);
                    true
                }
                (_, b'n') => {
                    o.nameref = Some(on);
                    true
                }
                (_, b'r') => {
                    o.readonly |= on;
                    true
                }
                (_, b'x') => {
                    o.export = Some(on);
                    true
                }
                (_, b't') => true,
                (Which::Local, b'g' | b'f' | b'F') => false,
                (_, b'g') => {
                    o.global = true;
                    true
                }
                (_, b'f') => {
                    o.funcs = true;
                    true
                }
                (_, b'F') => {
                    o.funcs = true;
                    o.func_names = true;
                    true
                }
                _ => false,
            };
            if !ok {
                vp.error(&format!("{bname}: -{}: invalid option", c as char))
                    .await;
                vp.error_raw(usage(which).as_bytes()).await;
                return Ok(2);
            }
            if c != b'p' && c != b'f' && c != b'F' && c != b'g' {
                o.any = true;
            }
        }
        i += 1;
    }
    let rest = &args[i..];
    if o.funcs && which != Which::Export && which != Which::Readonly {
        // With attributes to set (`declare -fx f`), nothing is printed.
        if o.any && !o.print {
            let missing = rest
                .iter()
                .any(|a| !vp.funcs.contains_key(&*String::from_utf8_lossy(a.name())));
            return Ok(i32::from(missing));
        }
        return functions(vp, bname, rest, o.func_names).await;
    }
    if rest.is_empty() {
        return print_all(vp, which, &o).await;
    }
    if o.print {
        return print_named(vp, bname, rest).await;
    }
    // In a function, declare and typeset (and local) make locals; -g and export /
    // readonly do not.
    let local = !vp.frames.is_empty()
        && !o.global
        && matches!(which, Which::Declare | Which::Typeset | Which::Local);
    let mut status = 0;
    for a in rest {
        let raw = a.name();
        if !is_name(raw) {
            vp.error(&format!("{bname}: `{}': not a valid identifier", a.shown()))
                .await;
            status = 1;
            continue;
        }
        let name = String::from_utf8_lossy(raw).into_owned();
        if matches!(which, Which::Export | Which::Readonly)
            && matches!(a, DeclArg::Assign { index: Some(_), .. })
        {
            vp.error(&format!("{bname}: `{}': not a valid identifier", a.shown()))
                .await;
            status = 1;
            continue;
        }
        if o.funcs {
            // `export -f` / `readonly -f`: functions are not exported to children here.
            if !vp.funcs.contains_key(&name) {
                vp.error(&format!("{bname}: {name}: not a function")).await;
                status = 1;
            }
            continue;
        }
        if let Err(msg) = declare_one(vp, &name, a, &o, local) {
            vp.error(&format!("{bname}: {msg}")).await;
            status = 1;
        }
    }
    Ok(status)
}

fn usage(which: Which) -> String {
    match which {
        Which::Declare => "declare: usage: declare [-aAfFgiIlnrtux] [name[=value] ...] or declare -p [-aAfFilnrtux] [name ...]\n".into(),
        Which::Typeset => "typeset: usage: typeset [-aAfFgiIlnrtux] name[=value] ... or typeset -p [-aAfFilnrtux] [name ...]\n".into(),
        Which::Local => "local: usage: local [option] name[=value] ...\n".into(),
        Which::Export => "export: usage: export [-fn] [name[=value] ...] or export -p [-f]\n".into(),
        Which::Readonly => "readonly: usage: readonly [-aAf] [name[=value] ...] or readonly -p\n".into(),
    }
}

/// Declares one name with the options, and assigns its value if given.
fn declare_one(vp: &mut Vp, name: &str, a: &DeclArg, o: &Opts, local: bool) -> Result<(), String> {
    if local {
        // A readonly variable cannot be shadowed either.
        if vp.vars.get(name).is_some_and(|v| v.readonly) {
            return Err(format!("{name}: readonly variable"));
        }
        let fresh = vp
            .frames
            .last()
            .is_some_and(|f| !f.saved.iter().any(|(s, _)| s == name));
        if fresh {
            vp.make_local(name);
            // A new local starts without a value, but keeps nothing of the outer one
            // except (bash) the attributes it is given now.
            vp.vars.insert(name, Var::default());
        }
    }
    let target = if o.nameref.is_some() {
        name.to_string()
    } else {
        vp.resolve(name).into_owned()
    };
    let has_value = !matches!(a, DeclArg::Word(_));
    {
        let var = vp.vars.entry(&target);
        if var.readonly && (has_value || o.indexed || o.assoc || o.integer.is_some()) {
            return Err(format!("{target}: readonly variable"));
        }
        if o.assoc && matches!(var.value, Value::Indexed(_)) {
            return Err(format!(
                "{target}: cannot convert indexed to associative array"
            ));
        }
        if o.indexed && matches!(var.value, Value::Assoc(_)) {
            return Err(format!(
                "{target}: cannot convert associative to indexed array"
            ));
        }
        if o.assoc && !var.is_assoc() {
            var.value = match std::mem::take(&mut var.value) {
                Value::Str(s) => {
                    let mut a = crate::vars::Assoc::default();
                    a.insert(b"0".to_vec(), s);
                    Value::Assoc(a)
                }
                _ => Value::Assoc(Default::default()),
            };
        }
        if o.indexed && !var.is_array() {
            crate::arrays::to_indexed(&mut var.value);
        }
        if let Some(i) = o.integer {
            var.integer = i;
        }
        if o.lower == Some(true) {
            var.case = Case::Lower;
        } else if o.upper == Some(true) {
            var.case = Case::Upper;
        } else if (o.lower == Some(false) && var.case == Case::Lower)
            || (o.upper == Some(false) && var.case == Case::Upper)
        {
            var.case = Case::None;
        }
        if let Some(n) = o.nameref {
            // The value it will refer to must be a name (or `name[sub]`).
            let target_name = match a {
                DeclArg::Assign {
                    index: None, value, ..
                } => Some(value.clone()),
                _ => var.get().map(<[u8]>::to_vec),
            };
            if n && let Some(t) = target_name
                && !t.is_empty()
                && !is_name(t.split(|&b| b == b'[').next().unwrap_or_default())
            {
                return Err(format!(
                    "`{}': invalid variable name for name reference",
                    String::from_utf8_lossy(&t)
                ));
            }
            var.nameref = n;
        }
        if let Some(x) = o.export {
            var.exported = x;
        }
    }
    match a {
        DeclArg::Word(_) => {}
        DeclArg::Assign {
            index: None,
            append,
            value,
            ..
        } => {
            if o.nameref == Some(true) {
                let v = value.clone();
                if !v.is_empty() && !is_name(v.split(|&b| b == b'[').next().unwrap_or_default()) {
                    return Err(format!(
                        "`{}': invalid variable name for name reference",
                        String::from_utf8_lossy(&v)
                    ));
                }
                vp.vars.entry(&target).set(v);
            } else {
                let v = vp.prepare_value(&target, value.clone(), *append)?;
                vp.vars.entry(&target).set(v);
            }
        }
        DeclArg::Assign {
            index: Some(i),
            append,
            value,
            ..
        } => {
            vp.assign_elem(&target, i, value.clone(), *append)?;
        }
        DeclArg::Array { append, items, .. } => {
            vp.assign_array(&target, items.clone(), *append)?;
        }
    }
    if o.readonly {
        vp.vars.entry(&target).readonly = true;
    }
    Ok(())
}

/// `declare -f` / `-F`: function definitions or names.
async fn functions(vp: &mut Vp, bname: &str, names: &[DeclArg], names_only: bool) -> R {
    let mut buf = Vec::new();
    let mut status = 0;
    let print = |vp: &Vp, name: &str, buf: &mut Vec<u8>| {
        let Some(f) = vp.funcs.get(name) else {
            return false;
        };
        if names_only {
            buf.extend_from_slice(name.as_bytes());
        } else {
            let text = psh_syntax::print_function_bash(&f.code.ast, f.def);
            buf.extend_from_slice(&f.code.ast.restore_bytes(&text));
        }
        buf.push(b'\n');
        true
    };
    if names.is_empty() {
        let mut all: Vec<&String> = vp.funcs.keys().collect();
        all.sort();
        for n in all {
            if names_only {
                buf.extend_from_slice(b"declare -f ");
                buf.extend_from_slice(n.as_bytes());
                buf.push(b'\n');
            } else {
                print(vp, n, &mut buf);
            }
        }
    } else {
        for a in names {
            let n = String::from_utf8_lossy(a.name()).into_owned();
            if !print(vp, &n, &mut buf) {
                status = 1;
            }
        }
    }
    let st = out(vp, bname, &buf).await?;
    Ok(if st != 0 { st } else { status })
}

/// `declare -p NAME…`.
async fn print_named(vp: &mut Vp, bname: &str, names: &[DeclArg]) -> R {
    let mut buf = Vec::new();
    let mut status = 0;
    for a in names {
        let n = String::from_utf8_lossy(a.name()).into_owned();
        match vp.vars.get(&n) {
            Some(v) => buf.extend_from_slice(&declaration(&n, v)),
            None => {
                vp.error(&format!("{bname}: {n}: not found")).await;
                status = 1;
            }
        }
    }
    let st = out(vp, bname, &buf).await?;
    Ok(if st != 0 { st } else { status })
}

/// Without names: every variable (with the attributes asked for), as declarations.
async fn print_all(vp: &mut Vp, which: Which, o: &Opts) -> R {
    let mut vars: Vec<(&String, &Var)> = vp
        .vars
        .iter()
        .filter(|(_, v)| match which {
            Which::Export => v.exported,
            Which::Readonly => v.readonly,
            _ => {
                !o.any
                    || (o.indexed && matches!(v.value, Value::Indexed(_)))
                    || (o.assoc && v.is_assoc())
                    || (o.integer == Some(true) && v.integer)
                    || (o.readonly && v.readonly)
                    || (o.export == Some(true) && v.exported)
                    || (o.nameref == Some(true) && v.nameref)
                    || (o.lower == Some(true) && v.case == Case::Lower)
                    || (o.upper == Some(true) && v.case == Case::Upper)
            }
        })
        .collect();
    vars.sort_by(|a, b| a.0.cmp(b.0));
    let mut buf = Vec::new();
    for (n, v) in vars {
        if !o.print && !matches!(which, Which::Export | Which::Readonly) {
            // `declare` alone prints `name=value` lines, like `set`.
            if let Some(val) = v.get().filter(|_| !v.is_array()) {
                buf.extend_from_slice(n.as_bytes());
                buf.push(b'=');
                buf.extend_from_slice(&super::shell_quote(val));
                buf.push(b'\n');
                continue;
            }
        }
        buf.extend_from_slice(&declaration(n, v));
    }
    out(vp, which.name(), &buf).await
}

/// `declare -FLAGS name=value` as `declare -p` prints it.
pub(crate) fn declaration(name: &str, v: &Var) -> Vec<u8> {
    let mut out = format!("declare -{} {name}", v.flags()).into_bytes();
    match &v.value {
        Value::Unset => {}
        Value::Str(s) => {
            out.push(b'=');
            out.extend_from_slice(&value_quote(s));
        }
        Value::Indexed(m) => {
            out.extend_from_slice(b"=(");
            for (i, (k, val)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                out.extend_from_slice(format!("[{k}]=").as_bytes());
                out.extend_from_slice(&value_quote(val));
            }
            out.push(b')');
        }
        Value::Assoc(a) => {
            out.extend_from_slice(b"=(");
            for (k, val) in a.iter() {
                out.push(b'[');
                out.extend_from_slice(&key_quote(k));
                out.extend_from_slice(b"]=");
                out.extend_from_slice(&value_quote(val));
                out.push(b' ');
            }
            out.push(b')');
        }
    }
    out.push(b'\n');
    out
}

/// A value as `declare -p` quotes it: `"…"` with `" \ $` and backquote escaped, or
/// `$'…'` when it has control characters.
pub(crate) fn value_quote(v: &[u8]) -> Vec<u8> {
    if v.iter().any(|b| b.is_ascii_control()) {
        return ansi_c_quote(v);
    }
    let mut out = vec![b'"'];
    for &b in v {
        if matches!(b, b'"' | b'\\' | b'$' | b'`') {
            out.push(b'\\');
        }
        out.push(b);
    }
    out.push(b'"');
    out
}

/// An associative key: bare when it has no special chars, else like a value.
fn key_quote(k: &[u8]) -> Vec<u8> {
    let plain = !k.is_empty()
        && k.iter().all(|&b| {
            !b.is_ascii_control()
                && !matches!(
                    b,
                    b' ' | b'\t'
                        | b'"'
                        | b'\''
                        | b'\\'
                        | b'$'
                        | b'`'
                        | b'['
                        | b']'
                        | b'*'
                        | b'?'
                        | b'('
                        | b')'
                        | b'<'
                        | b'>'
                        | b'&'
                        | b'|'
                        | b';'
                        | b'{'
                        | b'}'
                        | b'#'
                        | b'~'
                        | b'!'
                        | b'='
                )
        });
    if plain { k.to_vec() } else { value_quote(k) }
}

fn ansi_c_quote(v: &[u8]) -> Vec<u8> {
    let mut out = b"$'".to_vec();
    for &b in v {
        match b {
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\t' => out.extend_from_slice(b"\\t"),
            b'\r' => out.extend_from_slice(b"\\r"),
            0x07 => out.extend_from_slice(b"\\a"),
            0x08 => out.extend_from_slice(b"\\b"),
            0x0b => out.extend_from_slice(b"\\v"),
            0x0c => out.extend_from_slice(b"\\f"),
            0x1b => out.extend_from_slice(b"\\E"),
            b'\'' => out.extend_from_slice(b"\\'"),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b if b.is_ascii_control() => out.extend_from_slice(format!("\\{b:03o}").as_bytes()),
            b => out.push(b),
        }
    }
    out.push(b'\'');
    out
}
