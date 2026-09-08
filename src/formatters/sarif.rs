//! `sarif` formatter — port of `bandit/formatters/sarif.py`.
//!
//! SARIF 2.1.0 (`$schema` https://json.schemastore.org/sarif-2.1.0.json), tool driver Bandit / organization PyCQA / version + semanticVersion, rules with `help_uri`, `properties.tags = ["security", "external/cwe/cwe-N"]`, `precision = confidence.lower()`, results with `level` (HIGH→error, MEDIUM→warning (omitted as default), LOW→note), region/contextRegion from `get_code`, invocations with endTimeUtc/executionSuccessful and toolConfigurationNotifications for skipped files, `properties.metrics` in insertion order. Key order follows `jschema_to_python` (required first, then alphabetical of the sarif-om attribute names).
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"SARIF output written to file: {}"` (module tag `sarif`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64) -> io::Result<()> {
    todo!("M6: sarif formatter")
}
