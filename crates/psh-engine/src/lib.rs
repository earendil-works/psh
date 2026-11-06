//! The psh interpreter: a synchronous, sans-IO state machine stepped by its embedder.
//! Every side effect goes through a `Host`; no async runtime, only `std::task::Waker`.

/// The status a script finished with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitStatus(pub i32);

/// Runs a command string. The stub parses it and fails with status 2 ("not implemented").
pub fn run(source: &str) -> Result<ExitStatus, String> {
    match psh_syntax::parse(source) {
        Ok(()) => Ok(ExitStatus(0)),
        Err(err) => Err(err.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_reports_not_implemented() {
        assert_eq!(run("echo hi").unwrap_err(), "not implemented");
    }
}
