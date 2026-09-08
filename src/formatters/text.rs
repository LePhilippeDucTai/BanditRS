//! `text` formatter — port of `bandit/formatters/text.py`.
//!
//! `Run started:{datetime}`, verbose file lists, `Test results:`, issue blocks (`>> Issue: [id:test] text`, `   Severity: X   Confidence: Y`, `   CWE: ...`, `   More Info: url`, `   Location: file:line:col`, tabbed code lines, 50 dashes), `Code scanned:` totals, `Run metrics:`, `Files skipped (N):`; nothing printed in quiet mode with zero results; baseline candidates indented by 10 spaces.
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"Text output written to file: {}"` (module tag `text`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64) -> io::Result<()> {
    todo!("M6: text formatter")
}
