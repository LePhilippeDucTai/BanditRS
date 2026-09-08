//! Helpers shared by the integration tests (port of the `FunctionalTests`
//! helpers in `tests/functional/test_functional.py`).
#![allow(dead_code)]

use std::path::PathBuf;

use banditrs::core::config::BanditConfig;
use banditrs::core::manager::{AggType, Manager};
use banditrs::core::metrics::Scores;
use banditrs::core::test_set::TestSet;

/// Absolute path of `examples/<name>` (the fixtures copied from upstream).
pub fn example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join(name)
}

/// Expected counts per rank, in `RANKING` order (UNDEFINED, LOW, MEDIUM, HIGH).
pub type Counts = [u64; 4];

/// A manager built from the default configuration and profile (no `-p`, no
/// `-t`/`-s`), matching `FunctionalTests.setUp`.
fn manager_for_default() -> Manager {
    let config = BanditConfig::default();
    let profile = config.default_profile();
    let test_set = TestSet::new(&config, &profile);
    Manager::new(config, AggType::File, test_set)
}

/// Run bandit on `examples/<name>` and compare the issue counts by severity
/// and confidence (`check_example`).
pub fn check_example(name: &str, severity: Counts, confidence: Counts, ignore_nosec: bool) {
    let mut mgr = manager_for_default();
    mgr.ignore_nosec = ignore_nosec;
    let path = example_path(name).to_string_lossy().into_owned();
    mgr.discover_files(&[path], true, None);
    mgr.run_tests();

    let mut total = Scores::default();
    for s in &mgr.scores {
        total.add(s);
    }
    let counts = total.issue_counts();
    assert_eq!(counts[0], severity, "severity mismatch for {name}: skipped={:?}", mgr.skipped);
    assert_eq!(counts[1], confidence, "confidence mismatch for {name}: skipped={:?}", mgr.skipped);
}

/// `check_metrics`: compare `_totals` entries (`loc`, `nosec`, `skipped_tests`
/// and optional `SEVERITY.*`/`CONFIDENCE.*` counts).
pub fn check_metrics(name: &str, expect: &[(&str, u64)]) {
    let mut mgr = manager_for_default();
    let path = example_path(name).to_string_lossy().into_owned();
    mgr.discover_files(&[path], true, None);
    mgr.run_tests();
    for (label, value) in expect {
        assert_eq!(mgr.metrics.totals.get(label), Some(*value), "metric {label} mismatch for {name}");
    }
}
