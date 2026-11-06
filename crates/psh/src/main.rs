//! `psh -c CMD [ARGS]`: the standalone shell. A stub until the engine runs commands.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("-c") if args.len() >= 2 => match psh_engine::run(&args[1]) {
            Ok(status) => ExitCode::from(status.0 as u8),
            Err(message) => {
                eprintln!("psh: {message}");
                ExitCode::from(2)
            }
        },
        Some("-c") => {
            eprintln!("psh: -c: option requires an argument");
            ExitCode::from(2)
        }
        _ => {
            eprintln!("usage: psh -c COMMAND [ARGS...]");
            ExitCode::from(2)
        }
    }
}
