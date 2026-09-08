//! Port of `tests/unit/formatters/test_csv.py` (`bandit.formatters.csv`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::csv;
use common::formatters::{base_manager, make_issue, tmp_name};

/// Minimal RFC 4180 splitter (no embedded quotes/commas in these fixtures):
/// good enough to stand in for `csv.DictReader`.
fn split_row(row: &str) -> Vec<&str> {
    row.split(',').collect()
}

/// Port of `tests/unit/formatters/test_csv.py::CsvFormatterTests::test_report`.
///
/// Reads the row back through `csv.DictReader` semantics (header → field
/// names, next row → values) and asserts each field individually, as the
/// Python test does.
#[test]
fn test_report() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));

    let mut buf = Vec::new();
    csv::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let text = String::from_utf8(buf).unwrap();
    let mut lines = text.split("\r\n");
    let header: Vec<&str> = split_row(lines.next().unwrap());
    assert_eq!(
        header,
        vec![
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
        ]
    );
    let row = split_row(lines.next().unwrap());
    let data: std::collections::HashMap<&str, &str> = header.into_iter().zip(row).collect();

    assert_eq!(data["filename"], fname);
    assert_eq!(data["issue_severity"], "MEDIUM");
    assert_eq!(data["issue_confidence"], "MEDIUM");
    assert_eq!(data["issue_text"], "Possible binding to all interfaces.");
    assert_eq!(data["line_number"], "4");
    assert_eq!(data["line_range"], "[4]");
    assert_eq!(data["test_name"], "hardcoded_bind_all_interfaces");
    assert!(!data["more_info"].is_empty());
    assert_eq!(data["col_offset"], "8");
    assert_eq!(data["end_col_offset"], "16");
}
