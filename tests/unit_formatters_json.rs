//! Port of `tests/unit/formatters/test_json.py` (`bandit.formatters.json`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::core::issue::BaselineIssue;
use banditrs::formatters::json;
use common::formatters::{TEST_NAME, TEXT, base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_json.py::JsonFormatterTests::test_report`.
///
/// Python mocks `get_issue_list` to return `OrderedDict([(issue, [c1, c2])])`;
/// here two issues sharing a signature plus an unrelated baseline entry make
/// each of them "unmatched" with two candidates, which exercises the same
/// baseline branch of the formatter.
#[test]
fn test_report() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 1, 8, 16));
    mgr.results.push(make_issue(&fname, 2, 8, 16));
    let mut other = make_issue(&fname, 99, 8, 16);
    other.text = "different".to_string();
    mgr.baseline = vec![BaselineIssue::from_dict(&other.as_dict(true, -1)).unwrap()];

    let mut buf = Vec::new();
    json::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();

    assert!(v["generated_at"].is_string() && !v["generated_at"].as_str().unwrap().is_empty());
    let r0 = &v["results"][0];
    assert_eq!(r0["filename"], fname);
    assert_eq!(r0["issue_severity"], "MEDIUM");
    assert_eq!(r0["issue_confidence"], "MEDIUM");
    assert_eq!(r0["issue_text"], TEXT);
    assert!(r0["line_number"] == 1 || r0["line_number"] == 2);
    assert_eq!(
        r0["line_range"],
        serde_json::json!([r0["line_number"].as_u64().unwrap()])
    );
    assert_eq!(r0["test_name"], TEST_NAME);
    assert!(r0.get("candidates").is_some());
    assert!(r0["more_info"].as_str().is_some_and(|s| !s.is_empty()));
}
