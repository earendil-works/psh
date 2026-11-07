//! The hit list's constructs, read from psh-syntax's tree (D1222): the corpus feature
//! names of `research/corpus/results.json` (`pipe`, `heredoc_quoted`, `param_expansion_op`,
//! ...), the names of simple commands written as literals, and constructs the corpus did
//! not measure (`select`, `coproc`, `indirect_expansion`, `nameref`, `extglob`,
//! `regex_match`, `funsub`). A few features the tree does not keep (separators,
//! comments, line continuations) are still found in the text.

use std::collections::BTreeSet;

use psh_syntax::{
    AndOrOp, AssignValue, Ast, BinaryTest, CommandKind, CondExpr, CondKind, ListId, Param, ParamOp,
    ParamPrefix, Part, RedirFd, RedirOp, RedirTarget, Word, WordPart,
};

/// Interpreters that here-docs feed in the corpus's `heredoc_to_interp`.
const INTERPRETERS: &[&str] = &[
    "python", "python3", "node", "sh", "bash", "zsh", "ruby", "perl", "uv", "bun", "deno", "php",
];

#[derive(Default)]
pub struct Found {
    pub features: BTreeSet<&'static str>,
    /// Names of simple commands written as literal words, in order of appearance.
    pub names: Vec<String>,
}

/// Parses `script` and collects its constructs; `None` when psh-syntax rejects it.
pub fn find(script: &str) -> Option<Found> {
    let ast = psh_syntax::parse(script).ok()?;
    let mut w = Walker {
        ast: &ast,
        found: Found::default(),
    };
    let root = ast.list(ast.root());
    if root.items.len() > 1 {
        w.add("multi_statement");
    }
    w.list(ast.root());
    // Not in the tree.
    let text = script;
    if text.lines().any(|l| l.trim_start().starts_with('#')) || text.contains(" #") {
        w.add("comment");
    }
    if text.contains("\\\n") {
        w.add("line_continuation");
    }
    if text.replace(";;", "").contains(';') {
        w.add("semicolon");
    }
    Some(w.found)
}

struct Walker<'a> {
    ast: &'a Ast,
    found: Found,
}

impl Walker<'_> {
    fn add(&mut self, f: &'static str) {
        self.found.features.insert(f);
    }

    fn list(&mut self, id: ListId) {
        let list = self.ast.list(id).clone();
        for (i, item) in list.items.iter().enumerate() {
            if item.background {
                self.add("background");
            }
            for (op, _) in &item.rest {
                self.add(match op {
                    AndOrOp::And => "and",
                    AndOrOp::Or => "or",
                });
            }
            let pipelines = std::iter::once(&item.first).chain(item.rest.iter().map(|(_, p)| p));
            for (j, p) in pipelines.enumerate() {
                if p.stages.len() > 1 {
                    self.add("pipe");
                }
                if p.bangs > 0 {
                    self.add("negation");
                }
                for (k, stage) in p.stages.iter().enumerate() {
                    let first_of_script = id == self.ast.root() && i == 0 && j == 0 && k == 0;
                    self.cmd(stage.cmd, first_of_script);
                }
            }
        }
    }

    fn cmd(&mut self, id: psh_syntax::CmdId, first_of_script: bool) {
        let cmd = self.ast.cmd(id).clone();
        let mut name: Option<String> = None;
        match &cmd.kind {
            CommandKind::Simple(s) => {
                for a in &s.assigns {
                    if a.index.is_some() {
                        self.add("array");
                    }
                    match &a.value {
                        AssignValue::Scalar(w) => self.word(w),
                        AssignValue::Array(arr) => {
                            self.add("array");
                            for e in &arr.elems {
                                if let Some(k) = &e.key {
                                    self.word(k);
                                }
                                self.word(&e.value);
                            }
                        }
                    }
                }
                if !s.assigns.is_empty() {
                    self.add(if s.words.is_empty() {
                        "assignment"
                    } else {
                        "env_prefix"
                    });
                }
                if let Some(first) = s.words.first() {
                    name = psh_syntax::literal_text(self.ast, first);
                }
                let args: Vec<String> = s
                    .words
                    .iter()
                    .skip(1)
                    .filter_map(|w| psh_syntax::literal_text(self.ast, w))
                    .collect();
                if let Some(n) = name.as_deref() {
                    self.simple_name(n, &args, first_of_script);
                }
                for w in &s.words {
                    self.word(w);
                }
            }
            CommandKind::Brace(l) => {
                self.add("brace_group");
                self.list(*l);
            }
            CommandKind::Subshell(l) => {
                self.add("subshell");
                self.list(*l);
            }
            CommandKind::If(c) => {
                self.add("if");
                for b in &c.branches {
                    self.list(b.cond);
                    self.list(b.body);
                }
                if let Some(e) = c.else_body {
                    self.list(e);
                }
            }
            CommandKind::While(c) => {
                self.add("while");
                if c.until {
                    self.add("until");
                }
                self.list(c.cond);
                self.list(c.body);
            }
            CommandKind::For(c) | CommandKind::Select(c) => {
                self.add(if matches!(cmd.kind, CommandKind::Select(_)) {
                    "select"
                } else {
                    "for"
                });
                for w in c.words.iter().flatten() {
                    self.word(w);
                }
                self.list(c.body);
            }
            CommandKind::ArithFor(c) => {
                self.add("for_c");
                self.list(c.body);
            }
            CommandKind::Case(c) => {
                self.add("case");
                self.word(&c.word);
                for item in &c.items {
                    for p in &item.patterns {
                        self.word(p);
                    }
                    if let Some(b) = item.body {
                        self.list(b);
                    }
                }
            }
            CommandKind::Arith(w) => {
                self.add("arith_cmd");
                self.word(w);
            }
            CommandKind::Cond(c) => {
                self.add("dbl_bracket");
                self.cond(c);
            }
            CommandKind::Function(f) => {
                self.add("function");
                self.cmd(f.body, false);
            }
            CommandKind::Coproc(c) => {
                self.add("coproc");
                self.cmd(c.body, false);
            }
        }
        for r in &cmd.redirects {
            let fd2 = matches!(r.fd, Some(RedirFd::Number(2)));
            let target = match &r.target {
                RedirTarget::Word(w) => {
                    self.word(w);
                    psh_syntax::literal_text(self.ast, w)
                }
                RedirTarget::HereDoc(h) => {
                    let doc = self.ast.heredoc(*h).clone();
                    self.add("heredoc");
                    self.add(if doc.quoted {
                        "heredoc_quoted"
                    } else {
                        "heredoc_unquoted"
                    });
                    if doc.strip_tabs {
                        self.add("heredoc_dash");
                    }
                    match name.as_deref() {
                        Some("cat") => self.add("heredoc_to_cat"),
                        Some(n) if INTERPRETERS.contains(&n) => self.add("heredoc_to_interp"),
                        Some(_) => self.add("heredoc_to_other"),
                        None => {}
                    }
                    if !doc.quoted {
                        self.word(&doc.body);
                    }
                    None
                }
            };
            match r.op {
                RedirOp::Write | RedirOp::Clobber => {
                    if target.as_deref() == Some("/dev/null") {
                        self.devnull(fd2);
                    } else {
                        self.add("redir_out");
                        self.add("redir_to_file");
                    }
                }
                RedirOp::Append => {
                    self.add("redir_append");
                    self.add("redir_to_file");
                }
                RedirOp::WriteBoth | RedirOp::AppendBoth => {
                    if target.as_deref() == Some("/dev/null") {
                        self.add("redir_devnull");
                    } else {
                        self.add("redir_amp_out");
                        self.add("redir_to_file");
                    }
                }
                RedirOp::DupRead | RedirOp::DupWrite => self.add("redir_fd_dup"),
                RedirOp::Read | RedirOp::ReadWrite => self.add("redir_in"),
                RedirOp::HereString => self.add("herestring"),
                RedirOp::HereDoc | RedirOp::HereDocStrip => {}
            }
            if target.as_deref().is_some_and(|t| t.starts_with("/tmp")) {
                self.add("redir_tmp");
            }
        }
        if let Some(n) = name {
            self.found.names.push(n);
        }
    }

    fn devnull(&mut self, fd2: bool) {
        self.add("redir_devnull");
        if fd2 {
            self.add("redir_stderr_devnull");
        }
    }

    fn simple_name(&mut self, name: &str, args: &[String], first_of_script: bool) {
        if first_of_script && name == "cd" {
            self.add("cd_prefix");
        }
        match name {
            "[" | "test" => self.add("single_bracket"),
            "declare" | "local" | "export" | "readonly" | "typeset" => {
                self.add("declaration");
                if args.iter().any(|a| a.starts_with('-') && a.contains('n')) && name != "export" {
                    self.add("nameref");
                }
            }
            "set" => {
                for (i, a) in args.iter().enumerate() {
                    if a.starts_with('-') && !a.starts_with("--") {
                        if a.contains('e') {
                            self.add("set_e");
                        }
                        if a.contains('u') {
                            self.add("set_u");
                        }
                        if a.contains('x') {
                            self.add("set_x");
                        }
                    }
                    // `-o NAME`, also at the end of combined flags (`-euo pipefail`).
                    if (a.starts_with('-') || a.starts_with('+')) && a.len() > 1 && a.ends_with('o')
                    {
                        match args.get(i + 1).map(String::as_str) {
                            Some("pipefail") => self.add("set_pipefail"),
                            Some("errexit") => self.add("set_e"),
                            Some("nounset") => self.add("set_u"),
                            Some("xtrace") => self.add("set_x"),
                            Some("posix") => self.add("posix_mode"),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn cond(&mut self, c: &CondExpr) {
        match &c.kind {
            CondKind::Unary { arg, .. } => self.word(arg),
            CondKind::Binary { op, lhs, rhs, .. } => {
                if *op == BinaryTest::StringMatchesRegex {
                    self.add("regex_match");
                }
                self.word(lhs);
                self.word(rhs);
            }
            CondKind::Word(w) => self.word(w),
            CondKind::Not(e) | CondKind::Group(e) => self.cond(e),
            CondKind::And(a, b) | CondKind::Or(a, b) => {
                self.cond(a);
                self.cond(b);
            }
        }
    }

    fn word(&mut self, w: &Word) {
        self.parts(&w.parts, false);
    }

    fn parts(&mut self, parts: &[WordPart], quoted: bool) {
        for p in parts {
            match &p.kind {
                Part::Lit => {
                    let t = self.ast.text(p.span);
                    if !quoted && t != "[" && t != "]" && t.contains(['*', '?', '[']) {
                        self.add("glob_word");
                    }
                }
                Part::Escaped | Part::BadSubst => {}
                Part::Single => self.add("single_quoted"),
                Part::AnsiC => self.add("ansi_c_string"),
                Part::Double(inner) | Part::Locale(inner) => {
                    self.add("double_quoted");
                    self.parts(inner, true);
                }
                Part::Param(param) => self.param(param),
                Part::CmdSubst(l) => {
                    self.add("cmd_subst");
                    self.list(*l);
                }
                Part::FunSub { list, .. } => {
                    self.add("funsub");
                    self.list(*list);
                }
                Part::Backtick => self.add("backtick"),
                Part::Arith(w) => {
                    self.add("arith_expansion");
                    self.word(w);
                }
                Part::ProcSubst { list, .. } => {
                    self.add("proc_subst");
                    self.list(*list);
                }
                Part::Brace(_) => self.add("brace_expansion"),
                Part::ExtGlob { alts, .. } => {
                    self.add("extglob");
                    for a in alts {
                        self.word(a);
                    }
                }
                Part::Array(arr) => {
                    self.add("array");
                    for e in &arr.elems {
                        self.word(&e.value);
                    }
                }
            }
        }
    }

    fn param(&mut self, p: &Param) {
        if !p.braced {
            self.add("var_ref");
            return;
        }
        self.add("param_expansion");
        if p.prefix == ParamPrefix::Indirect {
            self.add("indirect_expansion");
        }
        if p.index.is_some() {
            self.add("array_ref");
        }
        if p.prefix != ParamPrefix::None || !matches!(p.op, ParamOp::None) {
            self.add("param_expansion_op");
        }
        match &p.op {
            ParamOp::Default { word, .. } => self.word(word),
            ParamOp::RemovePrefix { pattern, .. } | ParamOp::RemoveSuffix { pattern, .. } => {
                self.word(pattern)
            }
            ParamOp::Replace {
                pattern,
                replacement,
                ..
            } => {
                self.word(pattern);
                if let Some(r) = replacement {
                    self.word(r);
                }
            }
            ParamOp::Case {
                pattern: Some(pat), ..
            } => self.word(pat),
            ParamOp::Substring { offset, length } => {
                self.word(offset);
                if let Some(l) = length {
                    self.word(l);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feats(s: &str) -> Vec<&'static str> {
        find(s).unwrap().features.into_iter().collect()
    }

    #[test]
    fn agent_shapes() {
        let f = feats("cd /x && cat <<'EOF' | grep -n a 2>/dev/null\nbody\nEOF\n");
        for want in [
            "cd_prefix",
            "and",
            "heredoc",
            "heredoc_quoted",
            "heredoc_to_cat",
            "pipe",
            "redir_devnull",
            "redir_stderr_devnull",
        ] {
            assert!(f.contains(&want), "{want} missing from {f:?}");
        }
        assert!(!f.contains(&"heredoc_unquoted"));
        let f = feats("x=${a:-b} y=${#a} ${a[0]} $((1+2)) $(echo) && sleep 1 &\n");
        for want in [
            "param_expansion_op",
            "array_ref",
            "arith_expansion",
            "cmd_subst",
            "background",
            "env_prefix",
        ] {
            assert!(f.contains(&want), "{want} missing from {f:?}");
        }
        assert!(!feats("echo ${a}").contains(&"param_expansion_op"));
        assert!(feats("select x in a b; do :; done").contains(&"select"));
        assert!(feats("[[ $a =~ b ]]").contains(&"regex_match"));
        assert!(feats("set -euo pipefail").contains(&"set_pipefail"));
        assert!(feats("x=1").contains(&"assignment"));
        // Quoted globs are not globs.
        assert!(!feats("echo '*.txt' \"*\"").contains(&"glob_word"));
        assert!(feats("echo *.txt").contains(&"glob_word"));
    }

    #[test]
    fn names_in_nested_commands() {
        let f = find("if true; then x=$(trap 'a' EXIT; read v); fi | sort").unwrap();
        assert_eq!(f.names, vec!["true", "trap", "read", "sort"]);
        assert!(find("if then fi").is_none());
    }
}
