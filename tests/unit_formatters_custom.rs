//! Port of `tests/unit/formatters/test_custom.py` (`bandit.formatters.custom`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::custom;
use common::formatters::{TEXT, base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_custom.py::CustomFormatterTests::test_report`
/// (`col_offset = 30`, `end_col_offset = 38`,
/// template `"{line},{col},{end_col},{severity},{msg}"`).
#[test]
fn test_report() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 30, 38));

    let mut buf = Vec::new();
    custom::report(
        &mgr,
        &mut buf,
        Rank::Low,
        Rank::Low,
        Some("{line},{col},{end_col},{severity},{msg}"),
    )
    .unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert_eq!(text, format!("4,30,38,MEDIUM,{TEXT}\n"));
}
