//! Port of `tests/unit/formatters/test_text.py` (`bandit.formatters.text`).
//! Work package: `docs/plan/wp/WP-12-unit-formatters-text-screen.md`.

mod common;

use std::sync::Arc;

use banditrs::constants::Rank;
use banditrs::core::issue::{BaselineIssue, Cwe, Issue};
use banditrs::core::metrics::{FileMetrics, Scores};
use banditrs::formatters::text;
use banditrs::source::SourceFile;
use common::formatters::{base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_text.py::_get_issue_instance` (module
/// level helper, same defaults: `severity=MEDIUM`, `cwe=Cwe.MULTIPLE_BINDS`,
/// `confidence=MEDIUM`, `text="Test issue"`, `fname="code.py"`,
/// `test="bandit_plugin"`, `lineno=1`). `test_id` is left empty as in the
/// Python `Issue.__init__` default; Rust's `col_offset` has no `-1` sentinel
/// (the field is unsigned), so it stays at its `Issue::new` default of `0`.
fn get_issue_instance() -> Issue {
    Issue::new(
        Rank::Medium,
        Rank::Medium,
        Cwe::MULTIPLE_BINDS,
        "Test issue",
        "code.py",
        "bandit_plugin",
        "",
        1,
    )
}

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
/// PARTIAL (WP-12) resolved: the upstream test forces
/// `_totals = {loc: 1000, nosec: 50, skipped_tests: 0}` plus every
/// `SEVERITY.*`/`CONFIDENCE.*` counter to 1 (`scores = [{SEVERITY: [0,0,0,1],
/// CONFIDENCE: [0,0,0,1]}]`) and checks the 20 exact substrings of
/// `test_report_nobaseline` plus the `_output_issue_str` calls it expects
/// (`assert_has_calls`, adapted here to checking the assembled string
/// literally contains each call's real output).
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

    mgr.metrics.totals = FileMetrics {
        loc: 1000,
        nosec: 50,
        skipped_tests: 0,
        issues: Some([[1, 1, 1, 1], [1, 1, 1, 1]]),
    };

    let mut buf = Vec::new();
    text::report(&mgr, &mut buf, Rank::Low, Rank::Low, 5).unwrap();
    let out = String::from_utf8(buf).unwrap();

    let expected_items = [
        "Run started",
        "Files in scope (1)",
        "binding.py (score: ",
        "CONFIDENCE: 1",
        "SEVERITY: 1",
        &format!("CWE: {}", Cwe::MULTIPLE_BINDS),
        "Files excluded (1):",
        "def.py",
        "Undefined: 1",
        "Low: 1",
        "Medium: 1",
        "High: 1",
        "Total lines skipped ",
        "(#nosec): 50",
        "Total potential issues skipped due to specifically being ",
        "disabled (e.g., #nosec BXXX): 0",
        "Total issues (by severity)",
        "Total issues (by confidence)",
        "Files skipped (1)",
        "abc.py (File is bad)",
    ];
    for item in expected_items {
        assert!(out.contains(item), "missing {item:?} in:\n{out}");
    }

    // `output_str.assert_has_calls([mock.call(issue_a, "", lines=5), ...])`:
    // check the real (unmocked) rendering of each call is present verbatim.
    assert!(out.contains(&text::output_issue_str(&a, "", true, true, 5)));
    assert!(out.contains(&text::output_issue_str(&b, "", true, true, 5)));
}

/// Port of `tests/unit/formatters/test_text.py::TextFormatterTests::test_output_issue`.
///
/// Adapted: Python mocks `Issue.get_code` to return `"DDDDDDD"`; Rust has no
/// mocking, so a real `SourceFile` is attached to the issue and the expected
/// code lines are taken from `issue.get_code(-1, true)` itself (split on
/// `\n`, each prefixed by `indent`), exactly as the spec in
/// `docs/plan/wp/WP-12-unit-formatters-text-screen.md` describes.
#[test]
fn test_output_issue() {
    let mut i = get_issue_instance();
    i.source = Some(Arc::new(SourceFile::new(
        "code.py",
        "import socket\nsocket.bind(('0.0.0.0', 8080))\n",
    )));
    i.linerange = banditrs::core::issue::LineRange::single(1);
    let indent_val = "CCCCCCC";

    let template = |ind: &str, code: &str| -> String {
        let mut lines = vec![
            format!("{ind}>> Issue: [{}:{}] {}", i.test_id, i.test, i.text),
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
                "{ind}   Location: {}:{}:{}",
                i.fname, i.lineno, i.col_offset
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
    let issue_text = text::output_issue_str(&i, indent_val, true, true, -1);
    let expected_return = template(indent_val, &code);
    assert_eq!(expected_return, issue_text);

    let issue_text = text::output_issue_str(&i, indent_val, true, false, -1);
    let expected_return = template(indent_val, "");
    assert_eq!(expected_return, issue_text);

    // `issue.lineno = ""` / `issue.col_offset = ""` in Python: Rust's fields
    // are unsigned, so `show_lineno = false` alone already yields the same
    // empty location fields the mutation was simulating.
    let issue_text = text::output_issue_str(&i, indent_val, false, true, -1);
    assert!(issue_text.contains(&format!("{indent_val}   Location: code.py::")));
}

/// Port of `tests/unit/formatters/test_text.py::TextFormatterTests::test_report_baseline`.
///
/// Adapted: Python mocks `BanditManager.get_issue_list` to return an
/// `OrderedDict([(issue_a, [issue_x]), (issue_b, [issue_y, issue_z])])` built
/// from unrelated issues. Rust's `get_issue_list` is not mockable, so per the
/// spec the same shape is produced from real signature matching: `results =
/// [a, b1, b2]` where `a` has a unique signature and `b1`/`b2` share one, plus
/// a `baseline` entry matching none of them, so `get_issue_list` returns the
/// candidate variant `{a: [a], b1: [b1, b2], b2: [b1, b2]}`.
#[test]
fn test_report_baseline() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();

    let a = make_issue(&fname, 4, 8, 16);
    let mut b1 = make_issue(&fname, 4, 8, 16);
    b1.text = "same signature".to_string();
    let mut b2 = b1.clone();
    b2.lineno = 9; // location differs, signature (fname/text/severity/...) doesn't

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
    text::report(&mgr, &mut buf, Rank::Low, Rank::Low, 5).unwrap();
    let out = String::from_utf8(buf).unwrap();

    let indent_val = " ".repeat(10);
    assert!(out.contains(&text::output_issue_str(&a, "", true, true, 5)));
    assert!(out.contains(&text::output_issue_str(&b1, "", false, false, -1)));
    assert!(out.contains("-- Candidate Issues --"));
    assert!(out.contains(&text::output_issue_str(&b1, &indent_val, true, true, 5)));
    assert!(out.contains(&text::output_issue_str(&b2, &indent_val, true, true, 5)));
}
