//! Port of `tests/unit/formatters/test_html.py` (`bandit.formatters.html`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::core::issue::{Cwe, Issue, LineRange};
use banditrs::core::manager::IssueList;
use banditrs::formatters::html;
use banditrs::source::SourceFile;
use common::formatters::{TEST_NAME, TEXT, base_manager, make_issue, tmp_name};
use scraper::{Html, Selector};
use std::sync::Arc;

/// Port of `tests/unit/formatters/test_html.py::HtmlFormatterTests::test_report_with_skipped`.
///
/// Parses the document (`scraper`, dev-dependency) and asserts exactly one
/// `div#skipped` whose text contains `abc.py` and `File is bad`.
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
    assert!(out.contains("span id=\"loc\""));
    assert!(out.contains("span id=\"nosec\""));

    let doc = Html::parse_document(&out);
    let sel = Selector::parse("div#skipped").unwrap();
    let matches: Vec<_> = doc.select(&sel).collect();
    assert_eq!(matches.len(), 1);
    let text: String = matches[0].text().collect();
    assert!(text.contains("abc.py"));
    assert!(text.contains("File is bad"));
}

/// One `Issue`, `lineno = 1`, whose (real, non-mocked) source line 1 reads
/// `some code` — stands in for the Python test's `Issue.get_code` mock.
fn issue_instance(severity: Rank, cwe: u32, confidence: Rank) -> Issue {
    let mut issue = Issue::new(
        severity,
        confidence,
        Cwe(cwe),
        "Test issue",
        "code.py",
        "bandit_plugin",
        "B001",
        1,
    );
    issue.linerange = LineRange::single(1);
    issue.source = Some(Arc::new(SourceFile::new("code.py", "some code\n")));
    issue
}

/// Port of `tests/unit/formatters/test_html.py::HtmlFormatterTests::test_report_contents`.
///
/// Adaptation: `IssueList` is a real enum, not a Python `OrderedDict` mock —
/// the fixture (`issue_a -> [x, y]`, `issue_b -> [x]`, `issue_c -> [y]`) is
/// built directly as `IssueList::Baseline(..)` instead of going through
/// `Manager::get_issue_list`'s baseline matching (see WP-13's "alternative
/// propre": `html::render_issues`/`render_document` take the list/metrics
/// directly). `Rank` is an enum (no free-form `"CCCCCCC"` confidence like the
/// Python test forces); the low issue's confidence stays `Medium` and the
/// assertion below checks the template's `MEDIUM` string instead.
#[test]
fn test_report_contents() {
    let mut issue_a = issue_instance(Rank::Low, 123, Rank::Medium);
    issue_a.fname = "abc.py".to_string();
    issue_a.test = "AAAAAAA".into();
    issue_a.text = "BBBBBBB".to_string();

    let issue_b = issue_instance(Rank::Medium, 123, Rank::Medium);
    let issue_c = issue_instance(Rank::High, 123, Rank::Medium);
    let issue_x = issue_instance(Rank::Medium, 123, Rank::Medium);
    let issue_y = issue_instance(Rank::Medium, 123, Rank::Medium);

    let issue_list = IssueList::Baseline(vec![
        (&issue_a, vec![&issue_x, &issue_y]),
        (&issue_b, vec![&issue_x]),
        (&issue_c, vec![&issue_y]),
    ]);

    let results_str = html::render_issues(&issue_list, -1);
    let document = html::render_document(1000, 50, &[], &results_str);

    let doc = Html::parse_document(&document);
    let text_of = |sel: &str| -> String {
        let selector = Selector::parse(sel).unwrap();
        doc.select(&selector)
            .next()
            .map(|e| e.text().collect())
            .unwrap_or_default()
    };
    assert_eq!(text_of("span#loc"), "1000");
    assert_eq!(text_of("span#nosec"), "50");

    let count = |sel: &str| -> usize {
        let selector = Selector::parse(sel).unwrap();
        doc.select(&selector).count()
    };
    assert_eq!(count("div#issue-0 div.issue-sev-low"), 1);
    assert_eq!(count("div#issue-0 div.candidates"), 1);
    assert_eq!(count("div#issue-0 div.candidate"), 2);
    assert_eq!(count("div#issue-0 div.code"), 0);

    assert_eq!(count("div#issue-1 div.issue-sev-medium"), 1);
    assert_eq!(count("div#issue-1 div.candidates"), 0);
    assert_eq!(count("div#issue-1 div.candidate"), 0);
    assert_eq!(count("div#issue-1 div.code"), 1);

    assert_eq!(count("div#issue-2 div.issue-sev-high"), 1);
    assert_eq!(count("div#issue-2 div.code"), 1);

    let selector = Selector::parse("div#issue-0 div.candidate").unwrap();
    let first_candidate: String = doc.select(&selector).next().unwrap().text().collect();
    assert!(first_candidate.contains("some code"));

    let selector = Selector::parse("div#issue-1 div.code").unwrap();
    let issue1_code: String = doc.select(&selector).next().unwrap().text().collect();
    assert!(issue1_code.contains("some code"));

    let issue0_text = text_of("div#issue-0");
    assert!(issue0_text.contains("AAAAAAA:"));
    assert!(issue0_text.contains("BBBBBBB"));
    assert!(issue0_text.contains("abc.py"));
    assert!(issue0_text.contains("Line number: 1"));
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
