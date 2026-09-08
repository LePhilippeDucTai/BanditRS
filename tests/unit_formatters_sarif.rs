//! Port of `tests/unit/formatters/test_sarif.py` (`bandit.formatters.sarif`).
//! Work package: `docs/plan/wp/WP-13-unit-formatters-structured.md`.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::sarif;
use common::formatters::{TEST_ID, TEST_NAME, TEXT, base_manager, make_issue, tmp_name};

/// Port of `tests/unit/formatters/test_sarif.py::SarifFormatterTests::test_report`.
#[test]
fn test_report() {
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
    assert_eq!(driver["organization"], banditrs::AUTHOR);
    assert_eq!(driver["semanticVersion"], banditrs::VERSION);
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
    // MEDIUM → "warning", the SARIF default, which is omitted from the output.
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
