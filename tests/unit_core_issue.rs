//! Port of `tests/unit/core/test_issue.py` (`Issue`, `Cwe`).
//!
//! Work package: `docs/plan/wp/WP-11-unit-core-issue-blacklisting-docs.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/unit/core/test_issue.py`.

use banditrs::constants::Rank;
use banditrs::core::issue::{Cwe, Issue};
use banditrs::source::SourceFile;
use std::sync::Arc;

/// `_get_issue_instance` from `test_issue.py`.
fn get_issue_instance(severity: Rank, confidence: Rank) -> Issue {
    let mut new_issue = Issue::new(
        severity,
        confidence,
        Cwe::MULTIPLE_BINDS,
        "Test issue",
        "code.py",
        "bandit_plugin",
        "B999",
        1,
    );
    new_issue.col_offset = 8;
    new_issue.end_col_offset = 16;
    new_issue
}

/// Port of `tests/unit/core/test_issue.py::IssueTests::test_get_code` (adapted, see WP).
///
/// Adapted: Python mocks `linecache.getline` to return the control-byte string
/// `b"\x08\x30"`; here we attach a real `SourceFile` carrying the equivalent
/// bytes (`"\u{8}0\n"`) as the issue's `source` so `get_code` reads it without
/// touching a shared cache. The assertion stays equivalent: `get_code` must not
/// panic and must contain the decoded `"0"`.
#[test]
fn test_get_code() {
    let mut new_issue = Issue::new(
        Rank::Medium,
        Rank::Undefined,
        Cwe::MULTIPLE_BINDS,
        "Test issue",
        "code.py",
        "bandit_plugin",
        "B999",
        1,
    );
    new_issue.source = Some(Arc::new(SourceFile::new("code.py", "\u{8}0\n")));
    new_issue.linerange = banditrs::core::issue::LineRange::single(1);
    let code = new_issue.get_code(-1, false);
    assert!(code.contains('0'));
}

/// Port of `tests/unit/core/test_issue.py::IssueTests::test_issue_as_dict`.
#[test]
fn test_issue_as_dict() {
    let test_issue = get_issue_instance(Rank::Medium, Rank::Medium);
    let d = test_issue.as_dict(false, -1);
    assert_eq!(d["filename"], "code.py");
    assert_eq!(d["test_name"], "bandit_plugin");
    assert_eq!(d["test_id"], "B999");
    assert_eq!(d["issue_severity"], "MEDIUM");
    assert_eq!(
        d["issue_cwe"],
        serde_json::json!({
            "id": 605,
            "link": "https://cwe.mitre.org/data/definitions/605.html",
        })
    );
    assert_eq!(d["issue_confidence"], "MEDIUM");
    assert_eq!(d["issue_text"], "Test issue");
    assert_eq!(d["line_number"], 1);
    assert_eq!(d["line_range"], serde_json::json!([]));
    assert_eq!(d["col_offset"], 8);
    assert_eq!(d["end_col_offset"], 16);
    assert!(d.get("code").is_none());
}

/// Port of `tests/unit/core/test_issue.py::IssueTests::test_issue_create`.
#[test]
fn test_issue_create() {
    let new_issue = get_issue_instance(Rank::Medium, Rank::Medium);
    let _: Issue = new_issue;
}

/// Port of `tests/unit/core/test_issue.py::IssueTests::test_issue_filter_confidence`.
#[test]
fn test_issue_filter_confidence() {
    let levels = [Rank::Low, Rank::Medium, Rank::High];
    let issues: Vec<Issue> = levels
        .iter()
        .map(|&level| get_issue_instance(Rank::High, level))
        .collect();

    for &level in &levels {
        let rank = Rank::ALL.iter().position(|r| *r == level).unwrap();
        for i in &issues {
            let test = Rank::ALL.iter().position(|r| *r == i.confidence).unwrap();
            let result = i.filter(Rank::Undefined, level);
            assert_eq!(test >= rank, result);
        }
    }
}

/// Port of `tests/unit/core/test_issue.py::IssueTests::test_issue_filter_severity`.
#[test]
fn test_issue_filter_severity() {
    let levels = [Rank::Low, Rank::Medium, Rank::High];
    let issues: Vec<Issue> = levels
        .iter()
        .map(|&level| get_issue_instance(level, Rank::High))
        .collect();

    for &level in &levels {
        let rank = Rank::ALL.iter().position(|r| *r == level).unwrap();
        for i in &issues {
            let test = Rank::ALL.iter().position(|r| *r == i.severity).unwrap();
            let result = i.filter(level, Rank::Undefined);
            assert_eq!(test >= rank, result);
        }
    }
}

/// Port of `tests/unit/core/test_issue.py::IssueTests::test_issue_str`.
#[test]
fn test_issue_str() {
    let test_issue = get_issue_instance(Rank::Medium, Rank::Medium);
    let expect = "Issue: 'Test issue' from B999:bandit_plugin: CWE: CWE-605 \
        (https://cwe.mitre.org/data/definitions/605.html), Severity: MEDIUM \
        Confidence: MEDIUM at code.py:1:8";
    assert_eq!(test_issue.to_string(), expect);
}

/// Port of `tests/unit/core/test_issue.py::IssueTests::test_matches_issue`.
#[test]
fn test_matches_issue() {
    let issue_a = get_issue_instance(Rank::Medium, Rank::Medium);
    let issue_b = get_issue_instance(Rank::High, Rank::Medium);
    let issue_c = get_issue_instance(Rank::Medium, Rank::Low);

    let mut issue_d = get_issue_instance(Rank::Medium, Rank::Medium);
    issue_d.text = "ABCD".to_string();

    let mut issue_e = get_issue_instance(Rank::Medium, Rank::Medium);
    issue_e.fname = "file1.py".to_string();

    let issue_f = issue_a.clone();

    let mut issue_g = get_issue_instance(Rank::Medium, Rank::Medium);
    issue_g.test = "ZZZZ".into();

    let mut issue_h = issue_a.clone();
    issue_h.lineno = 12345;

    // positive tests
    assert!(issue_a.same_signature(&issue_a));
    assert!(issue_a.same_signature(&issue_f));
    assert!(issue_f.same_signature(&issue_a));

    // severity doesn't match
    assert!(!issue_a.same_signature(&issue_b));

    // confidence doesn't match
    assert!(!issue_a.same_signature(&issue_c));

    // text doesn't match
    assert!(!issue_a.same_signature(&issue_d));

    // filename doesn't match
    assert!(!issue_a.same_signature(&issue_e));

    // plugin name doesn't match
    assert!(!issue_a.same_signature(&issue_g));

    // line number doesn't match but should pass because we don't test that
    assert!(issue_a.same_signature(&issue_h));
}
