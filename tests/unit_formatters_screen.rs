//! Port of `tests/unit/formatters/test_screen.py` (`bandit.formatters.screen`).
//! Work package: `docs/plan/wp/WP-12-unit-formatters-text-screen.md`.
//!
//! `screen::report_to` renders into an `impl Write` (the real `report`
//! writes to stdout via `report_to` and only adds the `-o` hint on top), so
//! the four upstream tests can assert on the assembled string directly
//! instead of mocking `do_print`.

mod common;

use std::sync::Arc;

use banditrs::constants::Rank;
use banditrs::core::issue::{BaselineIssue, Cwe, Issue};
use banditrs::core::metrics::{FileMetrics, Scores};
use banditrs::formatters::screen;
use banditrs::source::SourceFile;
use common::formatters::{base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_screen.py::_get_issue_instance`
/// (`severity=MEDIUM`, `cwe=123`, `confidence=MEDIUM`, `text="Test issue"`,
/// `fname="code.py"`, `test="bandit_plugin"`, `lineno=1`); `test_id` stays
/// empty and `col_offset` stays at `Issue::new`'s default `0` (Rust has no
/// `-1` sentinel for the unsigned field).
fn get_issue_instance() -> Issue {
    Issue::new(
        Rank::Medium,
        Rank::Medium,
        Cwe(123),
        "Test issue",
        "code.py",
        "bandit_plugin",
        "",
        1,
    )
}

/// Extra (no upstream equivalent): with `-o <file>` the screen formatter only
/// logs `Screen formatter output was not written to file: <name>, consider '-f txt'`.
#[test]
fn extra_output_file_hint() {
    let mut mgr = base_manager();
    mgr.quiet = true; // avoid touching real stdout with ANSI issue output
    let (_lock, entries) = banditrs::log::with_buffer(|| {
        banditrs::log::set_level(banditrs::log::Level::Info);
        screen::report(&mgr, Rank::Low, Rank::Low, -1, Some("report.txt")).unwrap();
    });
    assert!(entries.iter().any(|e| {
        e.message
            .contains("Screen formatter output was not written to file: report.txt")
    }));
}

/// Port of `tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_no_issues`.
#[test]
fn test_no_issues() {
    let mgr = base_manager();
    let mut buf = Vec::new();
    screen::report_to(&mgr, &mut buf, Rank::Low, Rank::Low, 5).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("No issues identified."));
}

/// Port of `tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_output_issue`.
///
/// Adapted like `unit_formatters_text.rs::test_output_issue`: a real
/// `SourceFile` replaces the mocked `Issue.get_code`, and the expected code
/// lines come from `issue.get_code(-1, true)` itself.
#[test]
fn test_output_issue() {
    let mut i = get_issue_instance();
    i.source = Some(Arc::new(SourceFile::new(
        "code.py",
        "import socket\nsocket.bind(('0.0.0.0', 8080))\n",
    )));
    i.linerange = banditrs::core::issue::LineRange::single(1);
    let indent_val = "CCCCCCC";

    let template = |ind: &str, code: &str, color: &str| -> String {
        let mut lines = vec![
            format!(
                "{ind}{color}>> Issue: [{}:{}] {}",
                i.test_id, i.test, i.text
            ),
            format!(
                "{ind}   Severity: {}   Confidence: {}",
                i.severity.capitalize(),
                i.confidence.capitalize()
            ),
            format!("{ind}   CWE: {}", i.cwe),
            format!(
                "{ind}   More Info: {}",
                banditrs::core::docs_utils::get_url(&i.test_id)
            ),
            format!(
                "{ind}   Location: {}:{}:{}{}",
                i.fname,
                i.lineno,
                i.col_offset,
                screen::DEFAULT
            ),
        ];
        if !code.is_empty() {
            for line in code.split('\n') {
                lines.push(format!("{ind}{line}"));
            }
        }
        lines.join("\n")
    };

    let code = i.get_code(-1, true);
    let issue_text = screen::output_issue_str(&i, indent_val, true, true, -1);
    let expected_return = template(indent_val, &code, screen::MEDIUM);
    assert_eq!(expected_return, issue_text);

    let issue_text = screen::output_issue_str(&i, indent_val, true, false, -1);
    let expected_return = template(indent_val, "", screen::MEDIUM);
    assert_eq!(expected_return, issue_text);

    let issue_text = screen::output_issue_str(&i, indent_val, false, true, -1);
    assert!(issue_text.contains(&format!(
        "{indent_val}   Location: code.py::{}",
        screen::DEFAULT
    )));
}

/// Port of `tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_report_nobaseline`.
#[test]
fn test_report_nobaseline() {
    let mut mgr = base_manager();
    mgr.verbose = true;
    mgr.files_list = vec!["binding.py".to_string()];

    let mut score = Scores::default();
    score.note(Rank::Undefined, Rank::Undefined);
    mgr.scores = vec![score];

    mgr.skipped = vec![("abc.py".to_string(), "File is bad".to_string())];
    mgr.excluded_files = vec!["def.py".to_string()];

    let a = get_issue_instance();
    let b = get_issue_instance();
    mgr.results = vec![a.clone(), b.clone()];

    // Note: unlike `text`, the Python `screen._totals` mock omits
    // `skipped_tests` entirely (`{"loc": 1000, "nosec": 50}`); `screen.report`
    // never reads that key, so it stays at its `FileMetrics::default()` of 0.
    mgr.metrics.totals = FileMetrics {
        loc: 1000,
        nosec: 50,
        skipped_tests: 0,
        issues: Some([[1, 1, 1, 1], [1, 1, 1, 1]]),
    };

    let mut buf = Vec::new();
    screen::report_to(&mgr, &mut buf, Rank::Low, Rank::Low, 5).unwrap();
    let out = String::from_utf8(buf).unwrap();

    assert!(out.contains(&screen::output_issue_str(&a, "", true, true, 5)));
    assert!(out.contains(&screen::output_issue_str(&b, "", true, true, 5)));

    assert!(out.contains("Run started"));
    assert!(out.contains(&screen::header("Files in scope (1):")));
    assert!(out.contains("\n\tbinding.py (score: {SEVERITY: 1, CONFIDENCE: 1})"));
    assert!(out.contains(&format!(
        "{}\n\tdef.py",
        screen::header("Files excluded (1):")
    )));
    assert!(out.contains("Total lines of code: 1000\n\tTotal lines skipped (#nosec): 50"));
    assert!(out.contains(
        "Total issues (by severity):\n\t\tUndefined: 1\n\t\tLow: 1\n\t\tMedium: 1\n\t\tHigh: 1"
    ));
    assert!(out.contains(
        "Total issues (by confidence):\n\t\tUndefined: 1\n\t\tLow: 1\n\t\tMedium: 1\n\t\tHigh: 1"
    ));
    assert!(out.contains(&format!(
        "{}\n\tabc.py (File is bad)",
        screen::header("Files skipped (1):")
    )));
}

/// Port of `tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_report_baseline`.
///
/// Adapted like `unit_formatters_text.rs::test_report_baseline`: `results =
/// [a, b1, b2]` (`a` unique, `b1`/`b2` sharing a signature) plus a `baseline`
/// entry matching none of them reproduces the candidate variant
/// `{a: [a], b1: [b1, b2], b2: [b1, b2]}` that Python builds by mocking
/// `get_issue_list`.
#[test]
fn test_report_baseline() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();

    let a = make_issue(&fname, 4, 8, 16);
    let mut b1 = make_issue(&fname, 4, 8, 16);
    b1.text = "same signature".to_string();
    let mut b2 = b1.clone();
    b2.lineno = 9;

    mgr.results = vec![a.clone(), b1.clone(), b2.clone()];
    mgr.baseline = vec![BaselineIssue {
        text: Some("no match at all".to_string()),
        severity: None,
        cwe: Cwe::NOTSET,
        confidence: None,
        fname: None,
        test: None,
        test_id: None,
        code: serde_json::Value::Null,
        line_number: serde_json::Value::Null,
        line_range: serde_json::Value::Null,
        col_offset: serde_json::Value::Null,
        end_col_offset: serde_json::Value::Null,
    }];

    let mut buf = Vec::new();
    screen::report_to(&mgr, &mut buf, Rank::Low, Rank::Low, 5).unwrap();
    let out = String::from_utf8(buf).unwrap();

    let indent_val = " ".repeat(10);
    assert!(out.contains(&screen::output_issue_str(&a, "", true, true, 5)));
    assert!(out.contains(&screen::output_issue_str(&b1, "", false, false, -1)));
    assert!(out.contains("-- Candidate Issues --"));
    assert!(out.contains(&screen::output_issue_str(&b1, &indent_val, true, true, 5)));
    assert!(out.contains(&screen::output_issue_str(&b2, &indent_val, true, true, 5)));
}
