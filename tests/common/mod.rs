//! Helpers shared by the integration tests (port of the `FunctionalTests`
//! helpers in `tests/functional/test_functional.py`).
//!
//! Ownership (see docs/plan/README.md §5): this file belongs to WP-01; the
//! `formatters` sub-module belongs to WP-13. Every other work package keeps
//! its helpers inside its own `tests/<file>.rs`.
#![allow(dead_code)]

pub mod formatters;

use std::path::PathBuf;

use indexmap::IndexMap;

use banditrs::core::config::{BanditConfig, ConfigValue, Profile};
use banditrs::core::manager::{AggType, Manager};
use banditrs::core::metrics::Scores;
use banditrs::core::test_set::TestSet;

/// Absolute path of `examples/<name>` (the fixtures copied from upstream).
pub fn example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
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
    assert_eq!(
        counts[0], severity,
        "severity mismatch for {name}: skipped={:?}",
        mgr.skipped
    );
    assert_eq!(
        counts[1], confidence,
        "confidence mismatch for {name}: skipped={:?}",
        mgr.skipped
    );
}

/// `BanditManager(b_conf, "file")` + `BanditTestSet(config=b_conf, profile=profile)`:
/// a manager built from an explicit config/profile pair, for the tests that
/// need a plugin config section or a `-p`/`--exclude` profile instead of the
/// defaults (`with_test_set` in the Python fixture).
pub fn manager_with(config: BanditConfig, profile: Profile) -> Manager {
    let test_set = TestSet::new(&config, &profile);
    Manager::new(config, AggType::File, test_set)
}

/// `BanditConfig()` with `config[section] = value` set (the
/// `b_conf.config["markupsafe_xss"] = {...}` pattern in the Python
/// fixtures): defaults plus one extra top-level section.
pub fn config_with_section(section: &str, value: ConfigValue) -> BanditConfig {
    let mut config = BanditConfig::default();
    if let ConfigValue::Map(m) = &mut config.raw {
        m.insert(section.to_string(), value);
    }
    config
}

/// A `ConfigValue::Map` built from `(key, value)` pairs, in order.
pub fn config_map(entries: Vec<(&str, ConfigValue)>) -> ConfigValue {
    ConfigValue::Map(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<IndexMap<_, _>>(),
    )
}

/// A `ConfigValue::List` of strings.
pub fn config_str_list(items: &[&str]) -> ConfigValue {
    ConfigValue::List(
        items
            .iter()
            .map(|s| ConfigValue::Str(s.to_string()))
            .collect(),
    )
}

/// `check_example` on a manager the caller built (`manager_with`): same
/// severity/confidence comparison, but resetting only `scores` between calls
/// on the same manager, matching `FunctionalTests.check_example` (`self.b_mgr
/// .scores = []`) — `results`/`skipped`/`files_list` are left to accumulate,
/// exactly as upstream leaves them.
pub fn check_example_with(mgr: &mut Manager, name: &str, severity: Counts, confidence: Counts) {
    mgr.scores = Vec::new();
    let path = example_path(name).to_string_lossy().into_owned();
    mgr.discover_files(&[path], true, None);
    mgr.run_tests();

    let mut total = Scores::default();
    for s in &mgr.scores {
        total.add(s);
    }
    let counts = total.issue_counts();
    assert_eq!(
        counts[0], severity,
        "severity mismatch for {name}: skipped={:?}",
        mgr.skipped
    );
    assert_eq!(
        counts[1], confidence,
        "confidence mismatch for {name}: skipped={:?}",
        mgr.skipped
    );
}

/// `check_metrics`: compare `_totals` entries (`loc`, `nosec`, `skipped_tests`
/// and optional `SEVERITY.*`/`CONFIDENCE.*` counts).
pub fn check_metrics(name: &str, expect: &[(&str, u64)]) {
    let mut mgr = manager_for_default();
    let path = example_path(name).to_string_lossy().into_owned();
    mgr.discover_files(&[path], true, None);
    mgr.run_tests();
    for (label, value) in expect {
        assert_eq!(
            mgr.metrics.totals.get(label),
            Some(*value),
            "metric {label} mismatch for {name}"
        );
    }
}
