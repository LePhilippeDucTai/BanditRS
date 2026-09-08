//! Port of `tests/unit/formatters/test_csv.py` (`bandit.formatters.csv`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::csv;
use common::formatters::{base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_csv.py::CsvFormatterTests::test_report`.
///
/// PARTIAL (WP-13): read the row back through a CSV reader (`DictReader`
/// semantics: header → fields) and assert each field individually:
/// `filename`, `issue_severity == "MEDIUM"`, `issue_confidence == "MEDIUM"`,
/// `issue_text`, `line_number == "4"`, `line_range == "[4]"`, `test_name`,
/// `more_info` non-empty, `col_offset == "8"`, `end_col_offset == "16"`.
#[test]
fn test_report() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));

    let mut buf = Vec::new();
    csv::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let text = String::from_utf8(buf).unwrap();
    let mut lines = text.split("\r\n");
    let header = lines.next().unwrap();
    assert_eq!(
        header,
        "filename,test_name,test_id,issue_severity,issue_confidence,issue_cwe,issue_text,line_number,col_offset,end_col_offset,line_range,more_info"
    );
    let row = lines.next().unwrap();
    assert!(row.starts_with(&fname));
    assert!(row.contains(",4,8,16,[4],"), "row was: {row}");
    assert!(!row.ends_with(','));
}
