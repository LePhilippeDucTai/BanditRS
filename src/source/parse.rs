//! Parsing with ruff's parser.

use ruff_python_ast::ModModule;
use ruff_python_parser::{Mode, ParseOptions, Parsed, parse_unchecked};

use crate::ast::PyCompat;

/// A syntax error (bandit reports `syntax error while parsing AST from file`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError(pub String);

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Parse a module; any syntax error (including version-specific ones for
/// the selected compatibility level) is reported.
pub fn parse_module(text: &str, compat: PyCompat) -> Result<Parsed<ModModule>, SyntaxError> {
    let options = ParseOptions::from(Mode::Module).with_target_version(compat.target_version());
    let parsed = parse_unchecked(text, options)
        .try_into_module()
        .expect("module mode always yields a module");
    if let Some(err) = parsed.errors().first() {
        return Err(SyntaxError(err.to_string()));
    }
    if let Some(err) = parsed.unsupported_syntax_errors().first() {
        return Err(SyntaxError(err.to_string()));
    }
    Ok(parsed)
}
