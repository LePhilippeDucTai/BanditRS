//! Port of `tests/unit/formatters/test_text.py` (`bandit.formatters.text`).
//! Work package: `docs/plan/wp/WP-12-unit-formatters-text-screen.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::text;
use common::formatters::{TEST_ID, TEST_NAME, TEXT, base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_text.py::TextFormatterTests::test_no_issues`.
#[test]
fn test_no_issues() {
    let mgr = base_manager();
    let mut buf = Vec::new();
    text::report(&mgr, &mut buf, Rank::Low, Rank::Low, 5).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("No issues identified."));
}

/// Port of `tests/unit/formatters/test_text.py::TextFormatterTests::test_report_nobaseline`.
///
/// PARTIAL (WP-12): the upstream test forces `_totals = {loc: 1000, nosec: 50,
/// skipped_tests: 0}` plus every `SEVERITY.*`/`CONFIDENCE.*` counter to 1 and
/// `scores = [{SEVERITY: [0,0,0,1], CONFIDENCE: [0,0,0,1]}]`, then checks the
/// 20 exact substrings of the spec (`"Undefined: 1"`, `"(#nosec): 50"`, ...).
/// Add a way to set the manager totals from the test and assert them all.
#[test]
fn test_report_nobaseline() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));
    mgr.verbose = true;
    mgr.files_list = vec![fname.clone()];
    mgr.scores = vec![banditrs::core::metrics::Scores::default()];
    mgr.skipped = vec![("abc.py".to_string(), "File is bad".to_string())];
    mgr.excluded_files = vec!["def.py".to_string()];

    let mut buf = Vec::new();
    text::report(&mgr, &mut buf, Rank::Low, Rank::Low, 5).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("Run started"));
    assert!(out.contains("Files in scope (1)"));
    assert!(out.contains(&format!("{fname} (score: ")));
    assert!(out.contains(&format!(">> Issue: [{TEST_ID}:{TEST_NAME}] {TEXT}")));
    assert!(out.contains("Severity: Medium   Confidence: Medium"));
    assert!(out.contains("CWE: CWE-605 (https://cwe.mitre.org/data/definitions/605.html)"));
    assert!(out.contains("Files excluded (1):"));
    assert!(out.contains("def.py"));
    assert!(out.contains("Total lines skipped "));
    assert!(out.contains("(#nosec): 0"));
    assert!(out.contains("Total potential issues skipped due to specifically being "));
    assert!(out.contains("disabled (e.g., #nosec BXXX): 0"));
    assert!(out.contains("Total issues (by severity)"));
    assert!(out.contains("Total issues (by confidence)"));
    assert!(out.contains("Files skipped (1)"));
    assert!(out.contains("abc.py (File is bad)"));
}

/// Port of `tests/unit/formatters/test_text.py::TextFormatterTests::test_output_issue`.
#[test]
#[ignore = "WP-12: not ported yet — see docs/plan/wp/WP-12-unit-formatters-text-screen.md"]
fn test_output_issue() {
    unimplemented!(
        "WP-12: port tests/unit/formatters/test_text.py::TextFormatterTests::test_output_issue"
    );
}

/// Port of `tests/unit/formatters/test_text.py::TextFormatterTests::test_report_baseline`.
#[test]
#[ignore = "WP-12: not ported yet — see docs/plan/wp/WP-12-unit-formatters-text-screen.md"]
fn test_report_baseline() {
    unimplemented!(
        "WP-12: port tests/unit/formatters/test_text.py::TextFormatterTests::test_report_baseline"
    );
}
