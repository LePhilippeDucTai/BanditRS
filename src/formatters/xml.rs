//! `xml` formatter — port of `bandit/formatters/xml.py`.
//!
//! JUnit-like: `<?xml version='1.0' encoding='utf-8'?>` + `<testsuite name="bandit" tests="N">` + per issue `<testcase classname=fname name=test><error more_info=url type=severity message=text>Test ID: .. Severity: .. Confidence: ..\nCWE: ..\n{text}\nLocation {fname}:{lineno}</error></testcase>`; ElementTree escaping (attributes escape `&<>"` and \r \n \t as `&#13;`/`&#10;`/`&#09;`; text escapes `&<>`); written as bytes.

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::docs_utils::get_url;
use crate::core::manager::Manager;
use crate::pycompat::xml::{escape_attrib, escape_cdata};

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(manager: &Manager, out: &mut dyn Write, sev_level: Rank, conf_level: Rank, _lines: i64) -> io::Result<()> {
    let issues: Vec<_> = manager.get_issue_list(sev_level, conf_level).issues().collect();

    out.write_all(b"<?xml version='1.0' encoding='utf-8'?>\n")?;
    if issues.is_empty() {
        write!(out, "<testsuite name=\"bandit\" tests=\"0\" />")?;
        return Ok(());
    }
    write!(out, "<testsuite name=\"bandit\" tests=\"{}\">", issues.len())?;
    for issue in &issues {
        write!(
            out,
            "<testcase classname=\"{}\" name=\"{}\">",
            escape_attrib(&issue.fname),
            escape_attrib(&issue.test)
        )?;
        let text = format!(
            "Test ID: {} Severity: {} Confidence: {}\nCWE: {}\n{}\nLocation {}:{}",
            issue.test_id, issue.severity, issue.confidence, issue.cwe, issue.text, issue.fname, issue.lineno
        );
        write!(
            out,
            "<error more_info=\"{}\" type=\"{}\" message=\"{}\">{}</error>",
            escape_attrib(&get_url(&issue.test_id)),
            escape_attrib(issue.severity.as_str()),
            escape_attrib(&issue.text),
            escape_cdata(&text)
        )?;
        out.write_all(b"</testcase>")?;
    }
    out.write_all(b"</testsuite>")?;
    Ok(())
}
