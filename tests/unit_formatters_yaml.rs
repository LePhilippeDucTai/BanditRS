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
use banditrs::core::config::ConfigValue;
use banditrs::formatters::yaml;
use banditrs::pycompat::yaml_load::safe_load;
use common::formatters::{TEST_NAME, TEXT, base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_yaml.py::YamlFormatterTests::test_report`.
///
/// Parses the emitted document back with `safe_load` and asserts each field,
/// like the Python test's `yaml.safe_load`. Adaptation: the Python test
/// actually calls `bandit.formatters.json.report` (JSON is valid YAML) with a
/// mocked `get_issue_list` returning candidates, hence its `candidates`
/// assertion; the real `yaml` formatter under test here has no baseline
/// branch (`json::build_results(..., false)`), so `candidates` is
/// deliberately not asserted.
#[test]
fn test_report() {
    let mut mgr = base_manager();
    let (_tmp, fname) = tmp_name();
    mgr.results.push(make_issue(&fname, 4, 8, 16));

    let mut buf = Vec::new();
    yaml::report(&mgr, &mut buf, Rank::Low, Rank::Low, -1).unwrap();
    let text = String::from_utf8(buf).unwrap();

    let data = safe_load(&text).unwrap();
    let map = data.as_map().unwrap();
    assert!(map.contains_key("generated_at"));
    let result = &map["results"].as_list().unwrap()[0];
    let result = result.as_map().unwrap();
    assert_eq!(result["filename"].as_str(), Some(fname.as_str()));
    assert_eq!(result["issue_severity"].as_str(), Some("MEDIUM"));
    assert_eq!(result["issue_confidence"].as_str(), Some("MEDIUM"));
    assert_eq!(result["issue_text"].as_str(), Some(TEXT));
    assert_eq!(result["line_number"], ConfigValue::Int(4));
    assert_eq!(
        result["line_range"],
        ConfigValue::List(vec![ConfigValue::Int(4)])
    );
    assert_eq!(result["test_name"].as_str(), Some(TEST_NAME));
    assert!(map["results"].as_list().is_some());
    let more_info = &result["more_info"];
    assert_ne!(*more_info, ConfigValue::Null);
}
