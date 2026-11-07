//! Development tool: a toy runner for checking alias expansion against bash. Runs a
//! script chunk by chunk, executing only `shopt -s expand_aliases`, `alias N=V`,
//! `unalias N`, `echo` (literal words), `true`/`false`, `if`, `{ }`, `( )`, `&&`, `||`.
//! Anything else prints `?NAME`. Usage: `aliasrun FILE`.

use psh_syntax::*;

struct Run<'a> {
    ast: &'a Ast,
    aliases: &'a mut Aliases,
    expand: &'a mut bool,
}

impl Run<'_> {
    fn list(&mut self, id: ListId) -> bool {
        let mut st = true;
        for ao in &self.ast.list(id).items {
            st = self.pipeline(&ao.first);
            for (op, p) in &ao.rest {
                let go = match op {
                    AndOrOp::And => st,
                    AndOrOp::Or => !st,
                };
                if go {
                    st = self.pipeline(p);
                }
            }
        }
        st
    }

    fn pipeline(&mut self, p: &Pipeline) -> bool {
        let mut st = true;
        for s in &p.stages {
            st = self.cmd(s.cmd);
        }
        if p.negated() { !st } else { st }
    }

    fn words(&self, ws: &[Word]) -> Vec<String> {
        ws.iter()
            .map(|w| literal_text(self.ast, w).unwrap_or_else(|| "<dyn>".into()))
            .collect()
    }

    fn cmd(&mut self, id: CmdId) -> bool {
        let ast = self.ast;
        match &ast.cmd(id).kind {
            CommandKind::Simple(s) => {
                let w = self.words(&s.words);
                let Some(name) = w.first() else { return true };
                match name.as_str() {
                    "echo" => println!("{}", w[1..].join(" ")),
                    "true" | ":" => {}
                    "false" => return false,
                    "shopt" => {
                        if w.get(1).map(String::as_str) == Some("-s")
                            && w.get(2).map(String::as_str) == Some("expand_aliases")
                        {
                            *self.expand = true;
                        }
                    }
                    "alias" => {
                        for a in &w[1..] {
                            if let Some((n, v)) = a.split_once('=') {
                                self.aliases.set(n.as_bytes(), v.as_bytes());
                            }
                        }
                    }
                    "unalias" => {
                        for a in &w[1..] {
                            self.aliases.remove(a.as_bytes());
                        }
                    }
                    other => {
                        println!("?{other}");
                        return false;
                    }
                }
                true
            }
            CommandKind::Brace(l) | CommandKind::Subshell(l) => self.list(*l),
            CommandKind::If(i) => {
                for b in &i.branches {
                    if self.list(b.cond) {
                        return self.list(b.body);
                    }
                }
                match i.else_body {
                    Some(e) => self.list(e),
                    None => true,
                }
            }
            _ => {
                println!("?compound");
                true
            }
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(path).unwrap();
    let mut aliases = Aliases::new();
    let mut expand = false;
    let mut p = Parser::new(src.as_str(), Options::default());
    loop {
        p.set_aliases(if expand { Some(&aliases) } else { None });
        let Some(chunk) = p.next_chunk() else { break };
        match chunk {
            Ok(ast) => {
                let root = ast.root();
                Run {
                    ast: &ast,
                    aliases: &mut aliases,
                    expand: &mut expand,
                }
                .list(root);
            }
            Err(e) => {
                println!("ERROR {}", e.message);
                break;
            }
        }
    }
}
