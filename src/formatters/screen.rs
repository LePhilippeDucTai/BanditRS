//! `screen` formatter — port of `bandit/formatters/screen.py`.
//!
//! same layout as text with ANSI colours (`\x1b[95m` header, `\x1b[94m` LOW, `\x1b[93m` MEDIUM, `\x1b[91m` HIGH, `\x1b[0m` reset); no `Total potential issues skipped` line; always printed to stdout (never to `-o`, which only logs a hint).
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"Screen output written to file: {}"` (module tag `screen`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64) -> io::Result<()> {
    todo!("M6: screen formatter")
}
