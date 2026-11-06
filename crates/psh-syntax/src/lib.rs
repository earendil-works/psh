//! The psh parser: lexer, recursive-descent parser, AST with byte spans and a printer.
//!
//! `no_std` + `alloc` with zero dependencies, so the parser runs anywhere the classifier
//! does, including `wasm32-unknown-unknown`. Nothing here may touch the OS.
#![no_std]

extern crate alloc;

use alloc::string::String;

/// A parse error. Placeholder until the parser lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

/// Parses a script. The stub accepts nothing and reports that parsing is not implemented.
pub fn parse(_source: &str) -> Result<(), ParseError> {
    Err(ParseError {
        message: String::from("not implemented"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_reports_not_implemented() {
        assert_eq!(parse("echo hi").unwrap_err().message, "not implemented");
    }
}
