//! Port of `tests/functional/test_baseline.py` (`bandit -b`). See
//! docs/spec/cli_formatters_tests.md §C.3 for the 7 scenarios (baseline
//! created with `bandit -r [--ignore-nosec] -f json -o <tmp>/baseline_report.json <tmp>`
//! on a copy of `examples/<baseline file>` renamed to `<target name>`, then
//! `bandit -r [--ignore-nosec] -b <baseline> <tmp>` on the target file).
//! TODO(M8): implement with `tempfile` + `CARGO_BIN_EXE_bandit`.

#[test]
#[ignore = "CLI not implemented yet (PLAN.md M7/M8)"]
fn placeholder_baseline_scenarios() {
    unimplemented!("see docs/spec/cli_formatters_tests.md §C.3");
}
