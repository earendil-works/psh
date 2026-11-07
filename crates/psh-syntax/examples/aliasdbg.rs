fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut table = psh_syntax::Aliases::new();
    for a in &args[1..] {
        let (n, v) = a.split_once('=').unwrap();
        table.set(n.as_bytes(), v.as_bytes());
    }
    let mut p = psh_syntax::Parser::new(args[0].as_str(), Default::default());
    p.set_aliases(Some(&table));
    while let Some(c) = p.next_chunk() {
        match c {
            Ok(ast) => {
                println!(
                    "{} | script_len {} | {:?}",
                    psh_syntax::dump(&ast),
                    ast.script_len(),
                    ast.alias_uses()
                );
                let l = ast.list(ast.root());
                println!(
                    "  root {:?} items {:?}",
                    l.span,
                    l.items.iter().map(|i| i.span).collect::<Vec<_>>()
                );
            }
            Err(e) => println!("ERR {e:?}"),
        }
    }
}
