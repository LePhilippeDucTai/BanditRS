//! `sarif` formatter — port of `bandit/formatters/sarif.py`.
//!
//! SARIF 2.1.0 (`$schema` https://json.schemastore.org/sarif-2.1.0.json), tool driver Bandit / organization PyCQA / version + semanticVersion, rules with `help_uri`, `properties.tags = ["security", "external/cwe/cwe-N"]`, `precision = confidence.lower()`, results with `level` (HIGH→error, MEDIUM→warning (omitted as default), LOW→note), region/contextRegion from `get_code`, invocations with endTimeUtc/executionSuccessful and toolConfigurationNotifications for skipped files, `properties.metrics` in insertion order. Key order follows `jschema_to_python` (required first, then alphabetical of the sarif-om attribute names).

use std::io::{self, Write};

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::constants::Rank;
use crate::core::docs_utils::get_url;
use crate::core::issue::Issue;
use crate::core::manager::Manager;
use crate::pycompat::datetime::UtcDateTime;
use crate::pycompat::json::dumps_indent2_preserve_order;
use crate::pycompat::path::isabs;
use crate::pycompat::urlquote::{as_file_uri, quote};

const SCHEMA_URI: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const SCHEMA_VER: &str = "2.1.0";

fn to_uri(file_path: &str) -> String {
    if isabs(file_path) {
        as_file_uri(file_path)
    } else {
        quote(file_path, "/")
    }
}

fn level_from_severity(severity: Rank) -> &'static str {
    match severity {
        Rank::High => "error",
        Rank::Medium => "warning",
        Rank::Low => "note",
        Rank::Undefined => "warning",
    }
}

/// `parse_code(code)`: `(first_line_number, snippet_lines)`.
fn parse_code(code: &str) -> (u32, Vec<String>) {
    let mut code_lines: Vec<&str> = code.split('\n').collect();
    let last_real_line_ends_in_newline = code_lines.last().is_some_and(|l| l.is_empty());
    if last_real_line_ends_in_newline {
        code_lines.pop();
    }
    let mut first_line_number = 0u32;
    let mut snippet_lines: Vec<String> = Vec::with_capacity(code_lines.len());
    for (i, line) in code_lines.iter().enumerate() {
        let (num, text) = line.split_once(' ').unwrap_or((line, ""));
        if i == 0 {
            first_line_number = num.parse().unwrap_or(0);
        }
        snippet_lines.push(format!("{text}\n"));
    }
    if !last_real_line_ends_in_newline {
        if let Some(last) = snippet_lines.last_mut() {
            last.pop();
        }
    }
    (first_line_number, snippet_lines)
}

fn artifact_location(uri: &str) -> Value {
    let mut m = Map::new();
    m.insert("uri".into(), Value::from(uri));
    Value::Object(m)
}

fn region_and_context_region(issue: &Issue) -> (Value, Option<Value>) {
    let code = issue.get_code(-1, false);
    let line_range = issue.linerange;
    let start_line = line_range.start;
    let end_line = if line_range.len() > 1 { line_range.end } else { line_range.start };

    let mut region = Map::new();
    let mut context_region = None;
    if !code.is_empty() {
        let (first_line_number, snippet_lines) = parse_code(&code);
        let idx = (start_line as i64 - first_line_number as i64) as usize;
        if let Some(snippet_line) = snippet_lines.get(idx) {
            let mut snippet = Map::new();
            snippet.insert("text".into(), Value::from(snippet_line.as_str()));
            region.insert("snippet".into(), Value::Object(snippet));
        }
        let mut ctx_snippet = Map::new();
        ctx_snippet.insert("text".into(), Value::from(snippet_lines.concat()));
        let mut ctx_ordered = Map::new();
        ctx_ordered.insert("snippet".into(), Value::Object(ctx_snippet));
        ctx_ordered.insert("endLine".into(), Value::from(first_line_number + snippet_lines.len() as u32 - 1));
        ctx_ordered.insert("startLine".into(), Value::from(first_line_number));
        context_region = Some(Value::Object(ctx_ordered));
    }
    region.insert("endColumn".into(), Value::from(issue.end_col_offset + 1));
    region.insert("endLine".into(), Value::from(end_line));
    region.insert("startColumn".into(), Value::from(issue.col_offset + 1));
    region.insert("startLine".into(), Value::from(start_line));
    (Value::Object(region), context_region)
}

fn create_result(issue: &Issue, rules: &mut IndexMap<String, Value>) -> Value {
    let rule_index = if let Some(idx) = rules.get_index_of(issue.test_id.as_ref()) {
        idx
    } else {
        let mut rule = Map::new();
        rule.insert("id".into(), Value::from(issue.test_id.as_ref()));
        rule.insert("name".into(), Value::from(issue.test.as_ref()));
        let mut props = Map::new();
        props.insert("tags".into(), Value::Array(vec![Value::from("security"), Value::from(format!("external/cwe/cwe-{}", issue.cwe.id()))]));
        props.insert("precision".into(), Value::from(issue.confidence.lower()));
        rule.insert("properties".into(), Value::Object(props));
        rule.insert("helpUri".into(), Value::from(get_url(&issue.test_id)));
        let idx = rules.len();
        rules.insert(issue.test_id.to_string(), Value::Object(rule));
        idx
    };

    let mut physical_location = Map::new();
    let (region, context_region) = region_and_context_region(issue);
    physical_location.insert("region".into(), region);
    physical_location.insert("artifactLocation".into(), artifact_location(&to_uri(&issue.fname)));
    if let Some(cr) = context_region {
        physical_location.insert("contextRegion".into(), cr);
    }

    let mut location = Map::new();
    location.insert("physicalLocation".into(), Value::Object(physical_location));

    let mut message = Map::new();
    message.insert("text".into(), Value::from(issue.text.as_str()));

    let mut props = Map::new();
    props.insert("issue_confidence".into(), Value::from(issue.confidence.as_str()));
    props.insert("issue_severity".into(), Value::from(issue.severity.as_str()));

    let mut result = Map::new();
    result.insert("message".into(), Value::Object(message));
    // SARIF default level is "warning"; jschema_to_python omits fields equal
    // to their default, so MEDIUM/UNDEFINED (both map to "warning") vanish.
    if matches!(issue.severity, Rank::High | Rank::Low) {
        result.insert("level".into(), Value::from(level_from_severity(issue.severity)));
    }
    result.insert("locations".into(), Value::Array(vec![Value::Object(location)]));
    result.insert("properties".into(), Value::Object(props));
    result.insert("ruleId".into(), Value::from(issue.test_id.as_ref()));
    result.insert("ruleIndex".into(), Value::from(rule_index));
    Value::Object(result)
}

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(manager: &Manager, out: &mut dyn Write, sev_level: Rank, conf_level: Rank, _lines: i64) -> io::Result<()> {
    let mut driver = Map::new();
    driver.insert("name".into(), Value::from("Bandit"));
    driver.insert("organization".into(), Value::from(crate::AUTHOR));

    let mut rules: IndexMap<String, Value> = IndexMap::new();

    let mut results = Vec::new();
    for issue in manager.get_issue_list(sev_level, conf_level).issues() {
        results.push(create_result(issue, &mut rules));
    }

    if !rules.is_empty() {
        driver.insert("rules".into(), Value::Array(rules.into_values().collect()));
    }
    driver.insert("version".into(), Value::from(crate::VERSION));
    driver.insert("semanticVersion".into(), Value::from(crate::VERSION));

    let mut tool = Map::new();
    tool.insert("driver".into(), Value::Object(driver));

    let mut invocation = Map::new();
    invocation.insert("executionSuccessful".into(), Value::from(true));
    invocation.insert("endTimeUtc".into(), Value::from(UtcDateTime::now().iso_z()));
    let skips = manager.get_skipped();
    if !skips.is_empty() {
        let notifications: Vec<Value> = skips
            .iter()
            .map(|(fname, reason)| {
                let mut message = Map::new();
                message.insert("text".into(), Value::from(reason.as_str()));
                let mut physical_location = Map::new();
                physical_location.insert("artifactLocation".into(), artifact_location(&to_uri(fname)));
                let mut location = Map::new();
                location.insert("physicalLocation".into(), Value::Object(physical_location));
                let mut notification = Map::new();
                notification.insert("message".into(), Value::Object(message));
                notification.insert("level".into(), Value::from("error"));
                notification.insert("locations".into(), Value::Array(vec![Value::Object(location)]));
                Value::Object(notification)
            })
            .collect();
        invocation.insert("toolConfigurationNotifications".into(), Value::Array(notifications));
    }

    let mut properties = Map::new();
    properties.insert("metrics".into(), manager.metrics.to_json());

    let mut run = Map::new();
    run.insert("tool".into(), Value::Object(tool));
    run.insert("invocations".into(), Value::Array(vec![Value::Object(invocation)]));
    run.insert("properties".into(), Value::Object(properties));
    run.insert("results".into(), Value::Array(results));

    let mut log = Map::new();
    log.insert("runs".into(), Value::Array(vec![Value::Object(run)]));
    log.insert("version".into(), Value::from(SCHEMA_VER));
    log.insert("$schema".into(), Value::from(SCHEMA_URI));

    out.write_all(dumps_indent2_preserve_order(&Value::Object(log)).as_bytes())
}
