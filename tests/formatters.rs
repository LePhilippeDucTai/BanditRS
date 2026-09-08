//! Ports of `tests/unit/formatters/*.py` (docs/spec/cli_formatters_tests.md §C.7):
//! a `Manager` populated with one issue (`fname = <tmpfile>`, lineno 4,
//! linerange [4], test `hardcoded_bind_all_interfaces`, text
//! `Possible binding to all interfaces.`, severity/confidence MEDIUM, CWE
//! `MULTIPLE_BINDS` (605), col 8, end col 16) is rendered by each formatter
//! and the output parsed back.

use banditrs::constants::Rank;
use banditrs::core::config::BanditConfig;
use banditrs::core::issue::{BaselineIssue, Cwe, Issue, LineRange};
use banditrs::core::manager::{AggType, Manager};
use banditrs::core::test_set::TestSet;
use banditrs::formatters::{csv, custom, html, json, sarif, screen, text, xml, yaml};

const TEXT: &str = "Possible binding to all interfaces.";
const TEST_NAME: &str = "hardcoded_bind_all_interfaces";
const TEST_ID: &str = "B104";

fn base_manager() -> Manager {
    let config = BanditConfig::default();
    let profile = config.default_profile();
    let test_set = TestSet::new(&config, &profile);
    Manager::new(config, AggType::File, test_set)
}

fn make_issue(fname: &str, lineno: u32, col: u32, end_col: u32) -> Issue {
    let mut issue = Issue::new(
        Rank::Medium,
        Rank::Medium,
        Cwe::MULTIPLE_BINDS,
        TEXT,
        fname,
        TEST_NAME,
        TEST_ID,
        lineno,
    );
    issue.linerange = LineRange::single(lineno);
    issue.col_offset = col;
    issue.end_col_offset = end_col;
    issue
}

fn tmp_name() -> (tempfile::NamedTempFile, String) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let name = tmp.path().to_string_lossy().into_owned();
    (tmp, name)
}

#[test]
fn json_report_with_baseline_candidates() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 1, 8, 16));
    mgr.results.push(make_issue(&fname, 2, 8, 16));
    // A baseline entry that matches neither issue makes both "unmatched",
    // and since they share a signature each has 2 candidates.
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
    assert_eq!(r0["test_name"], TEST_NAME);
    assert!(r0.get("candidates").is_some());
    assert!(r0["more_info"].as_str().is_some_and(|s| !s.is_empty()));
}

#[test]
fn yaml_report_roundtrips_same_fields() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));

    let mut buf = Vec::new();
    yaml::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(text.contains("generated_at:"));
    assert!(text.contains(&format!("filename: {fname}")) || text.contains("filename:"));
    assert!(text.contains("issue_severity: MEDIUM"));
    assert!(text.contains("issue_confidence: MEDIUM"));
    assert!(text.contains("line_number: 4"));
}

#[test]
fn csv_report_has_expected_columns() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));

    let mut buf = Vec::new();
    csv::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let text = String::from_utf8(buf).unwrap();
    let mut lines = text.split("\r\n");
    let header = lines.next().unwrap();
    assert_eq!(
        header,
        "filename,test_name,test_id,issue_severity,issue_confidence,issue_cwe,issue_text,line_number,col_offset,end_col_offset,line_range,more_info"
    );
    let row = lines.next().unwrap();
    assert!(row.starts_with(&fname));
    assert!(row.contains(",4,8,16,[4],"), "row was: {row}");
    assert!(!row.ends_with(','));
}

#[test]
fn xml_report_has_testcase_and_error() {
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

#[test]
fn custom_report_renders_bare_tags() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 30, 38));

    let mut buf = Vec::new();
    custom::report(
        &mgr,
        &mut buf,
        Rank::Low,
        Rank::Low,
        Some("{line},{col},{end_col},{severity},{msg}"),
    )
    .unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert_eq!(text, format!("4,30,38,MEDIUM,{TEXT}\n"));
}

#[test]
fn text_report_no_issues() {
    let mgr = base_manager();
    let mut buf = Vec::new();
    text::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("No issues identified."));
}

#[test]
fn text_report_with_issue() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));
    mgr.verbose = true;
    mgr.files_list = vec![fname.clone()];
    mgr.scores = vec![banditrs::core::metrics::Scores::default()];
    mgr.skipped = vec![("abc.py".to_string(), "File is bad".to_string())];
    mgr.excluded_files = vec!["def.py".to_string()];

    let mut buf = Vec::new();
    text::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("Run started"));
    assert!(out.contains("Files in scope (1)"));
    assert!(out.contains(&format!("{fname} (score: ")));
    assert!(out.contains(&format!(">> Issue: [{TEST_ID}:{TEST_NAME}] {TEXT}")));
    assert!(out.contains("Severity: Medium   Confidence: Medium"));
    assert!(out.contains("CWE: CWE-605 (https://cwe.mitre.org/data/definitions/605.html)"));
    assert!(out.contains("Files excluded (1):"));
    assert!(out.contains("def.py"));
    assert!(out.contains("Total lines skipped "));
    assert!(out.contains("(#nosec): 0"));
    assert!(out.contains("Total potential issues skipped due to specifically being "));
    assert!(out.contains("disabled (e.g., #nosec BXXX): 0"));
    assert!(out.contains("Total issues (by severity)"));
    assert!(out.contains("Total issues (by confidence)"));
    assert!(out.contains("Files skipped (1)"));
    assert!(out.contains("abc.py (File is bad)"));
}

#[test]
fn screen_report_hint_when_output_file_given() {
    let mut mgr = base_manager();
    mgr.quiet = true; // avoid touching real stdout with ANSI issue output
    let out = std::io::Cursor::new(Vec::<u8>::new());
    let _ = out; // screen always targets real stdout; only exercise the hint path
    let (_lock, entries) = banditrs::log::with_buffer(|| {
        banditrs::log::set_level(banditrs::log::Level::Info);
        screen::report(&mgr, Rank::Low, Rank::Low, -1, Some("report.txt")).unwrap();
    });
    assert!(entries.iter().any(|e| {
        e.message
            .contains("Screen formatter output was not written to file: report.txt")
    }));
}

#[test]
fn html_report_contains_issue_block() {
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

#[test]
fn html_report_escapes_code_only() {
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

#[test]
fn sarif_report_has_expected_fields() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));

    let mut buf = Vec::new();
    sarif::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
    assert_eq!(
        v["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(v["version"], "2.1.0");
    let driver = &v["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "Bandit");
    assert_eq!(driver["organization"], "PyCQA");
    assert_eq!(driver["rules"][0]["id"], TEST_ID);
    assert_eq!(driver["rules"][0]["name"], TEST_NAME);
    let tags = driver["rules"][0]["properties"]["tags"].as_array().unwrap();
    assert!(tags.iter().any(|t| t == "security"));
    assert!(tags.iter().any(|t| t == "external/cwe/cwe-605"));
    assert_eq!(driver["rules"][0]["properties"]["precision"], "medium");
    assert!(
        v["runs"][0]["invocations"][0]["executionSuccessful"]
            .as_bool()
            .unwrap()
    );
    assert!(v["runs"][0]["invocations"][0]["endTimeUtc"].is_string());
    let result = &v["runs"][0]["results"][0];
    assert!(result.get("level").is_none());
    assert_eq!(result["message"]["text"], TEXT);
    let region = &result["locations"][0]["physicalLocation"]["region"];
    assert_eq!(region["startLine"], 4);
    assert_eq!(region["endLine"], 4);
    assert!(
        result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
            .as_str()
            .unwrap()
            .contains(&fname[1..])
    );
}
