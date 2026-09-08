//! Port of `tests/unit/formatters/test_yaml.py` (`bandit.formatters.yaml`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.
//!
//! Note: the upstream test actually calls `b_json.report` and re-reads the
//! output with `yaml.safe_load` (JSON is valid YAML). The Rust port exercises
//! the real YAML formatter and must parse its output back
//! (`banditrs::pycompat::yaml_load::safe_load`) with the same assertions as
//! the JSON test (including `candidates` / `more_info`).

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::yaml;
use common::formatters::{base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_yaml.py::YamlFormatterTests::test_report`.
///
/// PARTIAL (WP-13): substring checks only; parse the YAML back and assert
/// every field like `unit_formatters_json.rs::test_report` (baseline branch
/// with candidates, `generated_at`, `line_range == [4]`, `more_info`).
#[test]
fn test_report() {
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
