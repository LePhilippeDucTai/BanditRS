//! `json` formatter — port of `bandit/formatters/json.py`.
//!
//! `{"errors": [{filename, reason}], "generated_at": "%Y-%m-%dT%H:%M:%SZ", "metrics": metrics.data, "results": [issue dicts + more_info (+ candidates when >1)]}` sorted by `filename` (or `test_name` for `-a vuln`), `json.dumps(sort_keys=True, indent=2)` with ensure_ascii escaping, no trailing newline.
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"JSON output written to file: {}"` (module tag `json`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64) -> io::Result<()> {
    todo!("M6: json formatter")
}
