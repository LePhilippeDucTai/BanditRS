//! Port of `tests/unit/core/test_blacklisting.py` (`blacklisting.report_issue`).
//!
//! Work package: `docs/plan/wp/WP-11-unit-core-issue-blacklisting-docs.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/unit/core/test_blacklisting.py`.

use banditrs::constants::Rank;
use banditrs::core::blacklist::BlacklistEntry;
use banditrs::core::issue::Cwe;
use serde_json::json;

/// Port of `tests/unit/core/test_blacklisting.py::BlacklistingTests::test_report_issue`.
///
/// `blacklisting.report_issue(data, name)` in Python takes a raw `dict`; here
/// the equivalent input is a [`BlacklistEntry`] built from the same fields
/// (`level`, `message`, `id`), and the resulting draft's fields are checked
/// directly in place of `issue.as_dict(with_code=False)`.
#[test]
fn test_report_issue() {
    let entry = BlacklistEntry {
        name: "x".into(),
        id: "B000".into(),
        cwe: Cwe::NOTSET,
        qualnames: vec![],
        message: "test {name}".into(),
        level: "HIGH".into(),
    };

    let issue = entry.report_issue("name");
    assert_eq!(issue.test_id.as_deref(), Some("B000"));
    assert_eq!(issue.severity, Rank::High);
    assert_eq!(issue.cwe.as_dict(), json!({}));
    assert_eq!(issue.confidence, Rank::High);
    assert_eq!(issue.text, "test name");
}

/// Port of `tests/unit/core/test_blacklisting.py::BlacklistingTests::test_report_issue_defaults`.
///
/// Adapted: Python's input `dict` has no `id`/`level` key, and
/// `blacklisting.report_issue` falls back to `data.get("id", "LEGACY")` and
/// `data.get("level", "MEDIUM")`; the equivalent entry produced by the legacy
/// profile conversion carries `id = "LEGACY"` and `level = ""` (an empty
/// string that `Rank::parse` rejects, so `report_issue` falls back to
/// `Rank::Medium`, matching the Python default).
#[test]
fn test_report_issue_defaults() {
    let entry = BlacklistEntry {
        name: "x".into(),
        id: "LEGACY".into(),
        cwe: Cwe::NOTSET,
        qualnames: vec![],
        message: "test {name}".into(),
        level: "".into(),
    };

    let issue = entry.report_issue("name");
    assert_eq!(issue.test_id.as_deref(), Some("LEGACY"));
    assert_eq!(issue.severity, Rank::Medium);
    assert_eq!(issue.cwe.as_dict(), json!({}));
    assert_eq!(issue.confidence, Rank::High);
    assert_eq!(issue.text, "test name");
}
