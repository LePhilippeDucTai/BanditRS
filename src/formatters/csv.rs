//! `csv` formatter — port of `bandit/formatters/csv.py`.
//!
//! excel dialect (`,` delimiter, `"` quoting only when needed, CRLF); header `filename,test_name,test_id,issue_severity,issue_confidence,issue_cwe,issue_text,line_number,col_offset,end_col_offset,line_range,more_info`; `issue_cwe` = link (empty when NOTSET — deliberate deviation), `line_range` = Python list repr `[4]`, `more_info` = docs URL. No baseline support (dict keys are used).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::docs_utils::get_url;
use crate::core::manager::Manager;
use crate::pycompat::csv::write_row;

const FIELDNAMES: [&str; 12] = [
    "filename",
    "test_name",
    "test_id",
    "issue_severity",
    "issue_confidence",
    "issue_cwe",
    "issue_text",
    "line_number",
    "col_offset",
    "end_col_offset",
    "line_range",
    "more_info",
];

/// `repr(list(range))`: e.g. `[4]`, `[3, 4, 5, 6]`, `[]`.
fn python_list_repr(values: impl Iterator<Item = u32>) -> String {
    let parts: Vec<String> = values.map(|v| v.to_string()).collect();
    format!("[{}]", parts.join(", "))
}

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(manager: &Manager, out: &mut dyn Write, sev_level: Rank, conf_level: Rank, _lines: i64) -> io::Result<()> {
    out.write_all(write_row(&FIELDNAMES).as_bytes())?;
    for issue in manager.get_issue_list(sev_level, conf_level).issues() {
        let row = [
            issue.fname.as_str(),
            issue.test.as_ref(),
            issue.test_id.as_ref(),
            issue.severity.as_str(),
            issue.confidence.as_str(),
            &issue.cwe.link(),
            issue.text.as_str(),
            &issue.lineno.to_string(),
            &issue.col_offset.to_string(),
            &issue.end_col_offset.to_string(),
            &python_list_repr(issue.linerange.iter()),
            &get_url(&issue.test_id),
        ];
        out.write_all(write_row(&row).as_bytes())?;
    }
    Ok(())
}
