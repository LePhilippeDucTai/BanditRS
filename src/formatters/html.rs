//! `html` formatter — port of `bandit/formatters/html.py`.
//!
//! verbatim templates from `bandit/formatters/html.py` (header_block, report_block, issue_block, code_block, candidate_block, candidate_issue, skipped_block, metrics_block); only the code is HTML-escaped; `issue-sev-{severity.lower()}`; baseline candidates.
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"HTML output written to file: {}"` (module tag `html`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64) -> io::Result<()> {
    todo!("M6: html formatter")
}
