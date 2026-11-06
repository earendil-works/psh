//! The psh parser: lexer, recursive-descent parser, AST with byte spans and a printer.
//!
//! `no_std` + `alloc` with zero dependencies, so the parser runs anywhere the classifier
//! does, including `wasm32-unknown-unknown`. Nothing here may touch the OS.
//!
//! ```
//! let ast = psh_syntax::parse("echo hi | wc -l").unwrap();
//! let list = ast.list(ast.root());
//! assert_eq!(list.items.len(), 1);
//! assert_eq!(list.items[0].first.stages.len(), 2);
//! ```
#![no_std]

extern crate alloc;

mod ast;
mod brace;
mod dump;
mod error;
mod parser;
mod printer;
mod span;
mod words;

pub use ast::*;
pub use dump::dump;
pub use error::ParseError;
pub use parser::{Options, Parser, parse, parse_with};
pub use printer::print;
pub use span::{LineCol, LineIndex, Span};
pub use words::{backtick_body, literal_text};
