use std::io::Read;
fn main() {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let local = tokio::task::LocalSet::new();
    let code = local.block_on(&rt, async move {
        let list = match deno_task_shell::parser::parse(&text) {
            Ok(l) => l,
            Err(e) => { eprintln!("parse error: {e}"); return 2; }
        };
        let env = std::env::vars_os().collect();
        deno_task_shell::execute(list, env, std::env::current_dir().unwrap(), Default::default(), Default::default()).await
    });
    std::process::exit(code);
}
