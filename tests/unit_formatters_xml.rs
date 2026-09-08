//! Port of `tests/unit/formatters/test_xml.py` (`bandit.formatters.xml`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::xml;
use common::formatters::{TEST_NAME, TEXT, base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_xml.py::XmlFormatterTests::test_report`.
///
/// PARTIAL (WP-13): parse the document (`roxmltree`, dev-dependency) and
/// assert `testsuite/testcase@classname == tmp`, `testcase@name == test`,
/// `testcase/error@message == text`, `error@more_info` present — instead of
/// substring checks.
#[test]
fn test_report() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));

    let mut buf = Vec::new();
    xml::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(text.starts_with("<?xml version='1.0' encoding='utf-8'?>\n"));
    assert!(text.contains(&format!("classname=\"{fname}\"")));
    assert!(text.contains(&format!("name=\"{TEST_NAME}\"")));
    assert!(text.contains(&format!("message=\"{TEXT}\"")));
    assert!(text.contains("more_info="));
}
