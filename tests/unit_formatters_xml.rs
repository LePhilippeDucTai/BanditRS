//! Port of `tests/unit/formatters/test_xml.py` (`bandit.formatters.xml`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::xml;
use common::formatters::{TEST_NAME, TEXT, base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_xml.py::XmlFormatterTests::test_report`.
///
/// Parses the document with `roxmltree` and walks `testsuite/testcase/error`
/// like the Python test's `_xml_to_dict`, instead of substring checks.
#[test]
fn test_report() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));

    let mut buf = Vec::new();
    xml::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(text.starts_with("<?xml version='1.0' encoding='utf-8'?>\n"));

    let doc = roxmltree::Document::parse(&text).unwrap();
    let testcase = doc
        .descendants()
        .find(|n| n.has_tag_name("testcase"))
        .unwrap();
    assert_eq!(testcase.attribute("classname"), Some(fname.as_str()));
    assert_eq!(testcase.attribute("name"), Some(TEST_NAME));
    let error = testcase
        .children()
        .find(|n| n.has_tag_name("error"))
        .unwrap();
    assert_eq!(error.attribute("message"), Some(TEXT));
    assert!(error.attribute("more_info").is_some());
}
