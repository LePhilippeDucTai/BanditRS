//! Helpers shared by the integration tests (port of the `FunctionalTests`
//! helpers in `tests/functional/test_functional.py`).
//!
//! Status: stubs until the manager exists (PLAN.md M5). Expected usage:
//!
//! ```ignore
//! let mut mgr = manager_for(&Profile::default(), BanditConfig::default());
//! check_example(&mut mgr, "binding.py", [0, 0, 1, 0], [0, 0, 1, 0], false);
//! ```
#![allow(dead_code)]

use std::path::PathBuf;

/// Absolute path of `examples/<name>` (the fixtures copied from upstream).
pub fn example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join(name)
}

/// Expected counts per rank, in `RANKING` order (UNDEFINED, LOW, MEDIUM, HIGH).
pub type Counts = [u64; 4];

/// Run bandit on `examples/<name>` and compare the issue counts by severity
/// and confidence (`check_example`). `ignore_nosec` mirrors the helper's
/// argument.
/// TODO(M5): build a `Manager` with the default profile (plus `profile` /
/// `config` overrides), `discover_files([path], true)`, `run_tests()`, then
/// derive the counts from `manager.scores` exactly like the Python helper
/// (`score // RANKING_VALUES[rank]`).
pub fn check_example(_name: &str, _severity: Counts, _confidence: Counts, _ignore_nosec: bool) {
    unimplemented!("M5: check_example")
}

/// `check_metrics`: compare `_totals` entries (`loc`, `nosec`, `skipped_tests`
/// and optional `SEVERITY.*`/`CONFIDENCE.*` counts).
pub fn check_metrics(_name: &str, _expect: &[(&str, u64)]) {
    unimplemented!("M5: check_metrics")
}
