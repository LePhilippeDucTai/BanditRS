//! `html` formatter — port of `bandit/formatters/html.py`.
//!
//! verbatim templates from `bandit/formatters/html.py` (header_block, report_block, issue_block, code_block, candidate_block, candidate_issue, skipped_block, metrics_block); only the code is HTML-escaped; `issue-sev-{severity.lower()}`; baseline candidates.

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::docs_utils::get_url;
use crate::core::issue::Issue;
use crate::core::manager::{IssueList, Manager};
use crate::pycompat::html::escape;

const HEADER_BLOCK: &str = "\n<!DOCTYPE html>\n<html>\n<head>\n\n<meta charset=\"UTF-8\">\n\n<title>\n    Bandit Report\n</title>\n\n<style>\n\nhtml * {\n    font-family: \"Arial\", sans-serif;\n}\n\npre {\n    font-family: \"Monaco\", monospace;\n}\n\n.bordered-box {\n    border: 1px solid black;\n    padding-top:.5em;\n    padding-bottom:.5em;\n    padding-left:1em;\n}\n\n.metrics-box {\n    font-size: 1.1em;\n    line-height: 130%;\n}\n\n.metrics-title {\n    font-size: 1.5em;\n    font-weight: 500;\n    margin-bottom: .25em;\n}\n\n.issue-description {\n    font-size: 1.3em;\n    font-weight: 500;\n}\n\n.candidate-issues {\n    margin-left: 2em;\n    border-left: solid 1px; LightGray;\n    padding-left: 5%;\n    margin-top: .2em;\n    margin-bottom: .2em;\n}\n\n.issue-block {\n    border: 1px solid LightGray;\n    padding-left: .5em;\n    padding-top: .5em;\n    padding-bottom: .5em;\n    margin-bottom: .5em;\n}\n\n.issue-sev-high {\n    background-color: Pink;\n}\n\n.issue-sev-medium {\n    background-color: NavajoWhite;\n}\n\n.issue-sev-low {\n    background-color: LightCyan;\n}\n\n</style>\n</head>\n";

fn report_block(metrics: &str, skipped: &str, results: &str) -> String {
    format!(
        "\n<body>\n{metrics}\n{skipped}\n\n<br>\n<div id=\"results\">\n    {results}\n</div>\n\n</body>\n</html>\n"
    )
}

#[allow(clippy::too_many_arguments)]
fn issue_block(
    issue_no: usize,
    issue_class: &str,
    test_name: &str,
    test_id: &str,
    test_text: &str,
    severity: &str,
    confidence: &str,
    cwe_link: &str,
    cwe_id: u32,
    path: &str,
    line_number: u32,
    url: &str,
    code: &str,
    candidates: &str,
) -> String {
    format!(
        "\n<div id=\"issue-{issue_no}\">\n<div class=\"issue-block {issue_class}\">\n    <b>{test_name}: </b> {test_text}<br>\n    <b>Test ID:</b> {test_id}<br>\n    <b>Severity: </b>{severity}<br>\n    <b>Confidence: </b>{confidence}<br>\n    <b>CWE: </b><a href=\"{cwe_link}\" target=\"_blank\">CWE-{cwe_id}</a><br>\n    <b>File: </b><a href=\"{path}\" target=\"_blank\">{path}</a><br>\n    <b>Line number: </b>{line_number}<br>\n    <b>More info: </b><a href=\"{url}\" target=\"_blank\">{url}</a><br>\n{code}\n{candidates}\n</div>\n</div>\n"
    )
}

fn code_block(code: &str) -> String {
    format!("\n<div class=\"code\">\n<pre>\n{code}\n</pre>\n</div>\n")
}

fn candidate_block(candidate_list: &str) -> String {
    format!("\n<div class=\"candidates\">\n<br>\n<b>Candidates: </b>\n{candidate_list}\n</div>\n")
}

fn candidate_issue(code: &str) -> String {
    format!(
        "\n<div class=\"candidate\">\n<div class=\"candidate-issues\">\n<pre>{code}</pre>\n</div>\n</div>\n"
    )
}

fn skipped_block(files_list: &str) -> String {
    format!(
        "\n<br>\n<div id=\"skipped\">\n<div class=\"bordered-box\">\n<b>Skipped files:</b><br><br>\n{files_list}\n</div>\n</div>\n"
    )
}

fn metrics_block(loc: u64, nosec: u64) -> String {
    format!(
        "\n<div id=\"metrics\">\n    <div class=\"metrics-box bordered-box\">\n        <div class=\"metrics-title\">\n            Metrics:<br>\n        </div>\n        Total lines of code: <span id=\"loc\">{loc}</span><br>\n        Total lines skipped (#nosec): <span id=\"nosec\">{nosec}</span>\n    </div>\n</div>\n\n"
    )
}

/// `html_escape(issue.get_code(lines, True).strip("\n").lstrip(" "))`.
fn safe_code(issue: &Issue, lines: i64) -> String {
    let code = issue.get_code(lines, true);
    let trimmed = code.trim_matches('\n').trim_start_matches(' ');
    escape(trimmed)
}

fn render_issue(index: usize, issue: &Issue, code: &str, candidates: &str) -> String {
    let url = get_url(&issue.test_id);
    issue_block(
        index,
        &format!("issue-sev-{}", issue.severity.lower()),
        &issue.test,
        &issue.test_id,
        &issue.text,
        issue.severity.as_str(),
        issue.confidence.as_str(),
        &issue.cwe.link(),
        issue.cwe.id(),
        &issue.fname,
        issue.lineno,
        &url,
        code,
        candidates,
    )
}

/// Renders the `results` HTML for an already-built [`IssueList`] — split out
/// from `report` so tests can supply a hand-built list (candidates included)
/// without going through `Manager::get_issue_list`'s baseline matching.
pub fn render_issues(issue_list: &IssueList, lines: i64) -> String {
    let mut results_str = String::new();
    match issue_list {
        IssueList::Plain(issues) => {
            for (index, issue) in issues.iter().enumerate() {
                let code = code_block(&safe_code(issue, lines));
                results_str.push_str(&render_issue(index, issue, &code, ""));
            }
        }
        IssueList::Baseline(pairs) => {
            for (index, (issue, candidates)) in pairs.iter().enumerate() {
                if candidates.len() == 1 {
                    let code = code_block(&safe_code(issue, lines));
                    results_str.push_str(&render_issue(index, issue, &code, ""));
                } else {
                    let candidates_str: String = candidates
                        .iter()
                        .map(|c| candidate_issue(&safe_code(c, lines)))
                        .collect();
                    let candidates_html = candidate_block(&candidates_str);
                    results_str.push_str(&render_issue(index, issue, "", &candidates_html));
                }
            }
        }
    }
    results_str
}

/// Assembles the full document (header + metrics + skipped files + results),
/// independent of `Manager` — used by `report` and directly by tests that
/// build their own `IssueList`.
pub fn render_document(
    loc: u64,
    nosec: u64,
    skipped: &[(String, String)],
    results_str: &str,
) -> String {
    let skipped_str: String = skipped
        .iter()
        .map(|(fname, reason)| format!("{fname} <b>reason:</b> {reason}<br>"))
        .collect();
    let skipped_text = if skipped_str.is_empty() {
        String::new()
    } else {
        skipped_block(&skipped_str)
    };
    let metrics_summary = metrics_block(loc, nosec);
    let report_contents = report_block(&metrics_summary, &skipped_text, results_str);
    format!("{HEADER_BLOCK}{report_contents}")
}

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(
    manager: &Manager,
    out: &mut dyn Write,
    sev_level: Rank,
    conf_level: Rank,
    lines: i64,
) -> io::Result<()> {
    let issue_list = manager.get_issue_list(sev_level, conf_level);
    let results_str = render_issues(&issue_list, lines);
    let document = render_document(
        manager.metrics.totals.loc,
        manager.metrics.totals.nosec,
        manager.get_skipped(),
        &results_str,
    );
    out.write_all(document.as_bytes())
}
