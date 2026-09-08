//! `xml` formatter — port of `bandit/formatters/xml.py`.
//!
//! JUnit-like: `<?xml version='1.0' encoding='utf-8'?>` + `<testsuite name="bandit" tests="N">` + per issue `<testcase classname=fname name=test><error more_info=url type=severity message=text>Test ID: .. Severity: .. Confidence: ..\nCWE: ..\n{text}\nLocation {fname}:{lineno}</error></testcase>`; ElementTree escaping (attributes escape `&<>"` and \r \n \t as `&#13;`/`&#10;`/`&#09;`; text escapes `&<>`); written as bytes.
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"XML output written to file: {}"` (module tag `xml`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64) -> io::Result<()> {
    todo!("M6: xml formatter")
}
