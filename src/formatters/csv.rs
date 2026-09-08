//! `csv` formatter — port of `bandit/formatters/csv.py`.
//!
//! excel dialect (`,` delimiter, `"` quoting only when needed, CRLF); header `filename,test_name,test_id,issue_severity,issue_confidence,issue_cwe,issue_text,line_number,col_offset,end_col_offset,line_range,more_info`; `issue_cwe` = link (empty when NOTSET — deliberate deviation), `line_range` = Python list repr `[4]`, `more_info` = docs URL. No baseline support (dict keys are used).
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"CSV output written to file: {}"` (module tag `csv`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64) -> io::Result<()> {
    todo!("M6: csv formatter")
}
