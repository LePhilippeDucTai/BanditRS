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
pub const FORMATTER_NAMES: &[&str] = &[
    "csv", "json", "txt", "xml", "html", "sarif", "screen", "yaml", "custom",
];

/// Formatters that accept a baseline (`@accepts_baseline`), in registration order.
pub const BASELINE_FORMATTERS: &[&str] = &["json", "txt", "html", "screen", "custom"];

/// `manager.output_results(lines, sev_level, conf_level, output, format, template)`.
/// Unknown formats fall back to `screen`/`txt` depending on the terminal.
/// Errors are wrapped as `RuntimeError("Unable to output report using '{format}' formatter: {e}")`.
pub fn output_results(
    manager: &Manager,
    lines: i64,
    sev_level: Rank,
    conf_level: Rank,
    output: &mut Output,
    format: &str,
    template: Option<&str>,
) -> Result<(), String> {
    let format = if FORMATTER_NAMES.contains(&format) {
        format
    } else {
        default_format()
    };
    let name = output.name().to_string();
    let is_stdout = output.is_stdout();

    let result: io::Result<()> = match format {
        "csv" => csv::report(
            manager,
            output.writer().as_mut(),
            sev_level,
            conf_level,
            lines,
        ),
        "json" => json::report(
            manager,
            output.writer().as_mut(),
            sev_level,
            conf_level,
            lines,
        ),
        "txt" => text::report(
            manager,
            output.writer().as_mut(),
            sev_level,
            conf_level,
            lines,
        ),
        "xml" => xml::report(
            manager,
            output.writer().as_mut(),
            sev_level,
            conf_level,
            lines,
        ),
        "html" => html::report(
            manager,
            output.writer().as_mut(),
            sev_level,
            conf_level,
            lines,
        ),
        "sarif" => sarif::report(
            manager,
            output.writer().as_mut(),
            sev_level,
            conf_level,
            lines,
        ),
        "yaml" => yaml::report(
            manager,
            output.writer().as_mut(),
            sev_level,
            conf_level,
            lines,
        ),
        "custom" => custom::report(
            manager,
            output.writer().as_mut(),
            sev_level,
            conf_level,
            template,
        ),
        "screen" => screen::report(
            manager,
            sev_level,
            conf_level,
            lines,
            if is_stdout { None } else { Some(&name) },
        ),
        _ => unreachable!("format validated against FORMATTER_NAMES/default_format above"),
    };
    result.map_err(|e| format!("Unable to output report using '{format}' formatter: {e}"))?;

    if !is_stdout {
        match format {
            "csv" => crate::log_info!("csv", "CSV output written to file: {}", name),
            "json" => crate::log_info!("json", "JSON output written to file: {}", name),
            "txt" => crate::log_info!("text", "Text output written to file: {}", name),
            "xml" => crate::log_info!("xml", "XML output written to file: {}", name),
            "html" => crate::log_info!("html", "HTML output written to file: {}", name),
            "sarif" => crate::log_info!("sarif", "SARIF output written to file: {}", name),
            "yaml" => crate::log_info!("yaml", "YAML output written to file: {}", name),
            "custom" => crate::log_info!("custom", "Result written to file: {}", name),
            "screen" => {}
            _ => unreachable!(),
        }
    }
    Ok(())
}

/// `"screen"` when stdout is a tty, `NO_COLOR` is unset and `TERM != "dumb"`, else `"txt"`.
pub fn default_format() -> &'static str {
    use std::io::IsTerminal;
    if io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").as_deref() != Ok("dumb")
    {
        "screen"
    } else {
        "txt"
    }
}
