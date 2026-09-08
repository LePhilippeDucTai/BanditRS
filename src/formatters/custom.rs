//! `custom` formatter — port of `bandit/formatters/custom.py`.
//!
//! `--msg-template` rendering with Python `str.format` semantics: tags abspath, relpath, line, col, end_col, test_id, severity, msg, confidence, range, cwe; default template `{abspath}:{line}: {test_id}[bandit]: {severity}: {msg}`; validation with `SafeMapper(line=0)` (only `line` is an int during validation) → `Template is not in valid format: {err}` + exit 2; no tags → `No tags were found in the template. Are you missing '{}'?` + exit 2; unknown tags → warning `Tag '%s' was not recognized and will be skipped, did you mean to use '%s'?` and emitted as the bare tag name; one line per issue (`template + "\n"`).
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"Result output written to file: {}"` (module tag `custom`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64, _template: Option<&str>) -> io::Result<()> {
    todo!("M6: custom formatter")
}
