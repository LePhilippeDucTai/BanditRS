//! Output formatters (port of `bandit/formatters/*.py`). See
//! docs/spec/cli_formatters_tests.md part B for the exact output of each
//! formatter. Status: stubs (M6).
//!
//! Every formatter has the signature
//! `fn report(manager, out, sev_level, conf_level, lines) -> io::Result<()>`
//! (`custom` takes a template instead of `lines`). Formatters accepting a
//! baseline (`IssueList::Baseline`): custom, html, json, screen, text.

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

pub mod csv;
pub mod custom;
pub mod html;
pub mod json;
pub mod sarif;
pub mod screen;
pub mod text;
pub mod xml;
pub mod yaml;

/// Output destination (`-o`): stdout or a file opened at argument-parsing
/// time (`argparse.FileType("w")`).
pub enum Output {
    Stdout,
    File { name: String, file: std::fs::File },
}

impl Output {
    pub fn name(&self) -> &str {
        match self {
            Output::Stdout => "<stdout>",
            Output::File { name, .. } => name,
        }
    }

    pub fn is_stdout(&self) -> bool {
        matches!(self, Output::Stdout)
    }

    pub fn writer(&mut self) -> Box<dyn Write + '_> {
        match self {
            Output::Stdout => Box::new(io::stdout().lock()),
            Output::File { file, .. } => Box::new(io::BufWriter::new(file)),
        }
    }
}

/// Formatter names in `setup.cfg` order.
pub const FORMATTER_NAMES: &[&str] = &["csv", "json", "txt", "xml", "html", "sarif", "screen", "yaml", "custom"];

/// Formatters that accept a baseline (`@accepts_baseline`), in registration order.
pub const BASELINE_FORMATTERS: &[&str] = &["json", "txt", "html", "screen", "custom"];

/// `manager.output_results(lines, sev_level, conf_level, output, format, template)`.
/// Unknown formats fall back to `screen`/`txt` depending on the terminal.
/// TODO(M6): dispatch; wrap errors as
/// `RuntimeError("Unable to output report using '{format}' formatter: {e}")`.
pub fn output_results(
    _manager: &Manager,
    _lines: i64,
    _sev_level: Rank,
    _conf_level: Rank,
    _output: &mut Output,
    _format: &str,
    _template: Option<&str>,
) -> Result<(), String> {
    todo!("M6: output_results")
}

/// `"screen"` when stdout is a tty, `NO_COLOR` is unset and `TERM != "dumb"`, else `"txt"`.
pub fn default_format() -> &'static str {
    use std::io::IsTerminal;
    if io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() && std::env::var("TERM").as_deref() != Ok("dumb") {
        "screen"
    } else {
        "txt"
    }
}
