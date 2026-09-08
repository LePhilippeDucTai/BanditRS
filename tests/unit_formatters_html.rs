//! Port of `tests/unit/formatters/test_html.py` (`bandit.formatters.html`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::html;
use common::formatters::{TEST_NAME, TEXT, base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_html.py::HtmlFormatterTests::test_report_with_skipped`.
///
/// PARTIAL (WP-13): parse the document (`scraper`, dev-dependency) and assert
/// exactly one `div#skipped` whose text contains `abc.py` and `File is bad`.
#[test]
fn test_report_with_skipped() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));
    mgr.skipped = vec![("abc.py".to_string(), "File is bad".to_string())];

    let mut buf = Vec::new();
    html::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("id=\"issue-0\""));
    assert!(out.contains("issue-sev-medium"));
    assert!(out.contains(&format!("<b>{TEST_NAME}: </b> {TEXT}")));
    assert!(out.contains("<div id=\"skipped\">"));
    assert!(out.contains("abc.py"));
    assert!(out.contains("File is bad"));
    assert!(out.contains("span id=\"loc\""));
    assert!(out.contains("span id=\"nosec\""));
}

/// Port of `tests/unit/formatters/test_html.py::HtmlFormatterTests::test_report_contents`.
#[test]
#[ignore = "WP-13: not ported yet — see docs/plan/wp/WP-13-unit-formatters-structured.md"]
fn test_report_contents() {
    unimplemented!(
        "WP-13: port tests/unit/formatters/test_html.py::HtmlFormatterTests::test_report_contents"
    );
}

/// Port of `tests/unit/formatters/test_html.py::HtmlFormatterTests::test_escaping`:
/// only the code block is HTML-escaped.
#[test]
fn test_escaping() {
    let mut mgr = base_manager();
    let (tmp, fname) = tmp_name();
    std::fs::write(tmp.path(), "if a < b:\n    <tag in code>\n    pass\n").unwrap();
    let mut issue = make_issue(&fname, 2, 4, 20);
    issue.source = Some(std::sync::Arc::new(banditrs::source::SourceFile::new(
        &fname,
        "if a < b:\n    <tag in code>\n    pass\n",
    )));
    mgr.results.push(issue);

    let mut buf = Vec::new();
    html::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(!out.contains("<tag in code>"));
    assert!(out.contains("&lt;tag in code&gt;"));
}
