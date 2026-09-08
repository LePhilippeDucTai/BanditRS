//! Port of `tests/functional/test_baseline.py` (`bandit -b`, 7 scénarios via le binaire).
//!
//! Work package: `docs/plan/wp/WP-02-functional-baseline.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/functional/test_baseline.py`.

use std::path::{Path, PathBuf};
use std::process::Command;

// upstream: tests/functional/test_baseline.py module-level constants (verbatim).
const NEW_CANDIDATES_ALL_TOTAL_LINES: &str = "Total lines of code: 12";
const NEW_CANDIDATES_SOME_TOTAL_LINES: &str = "Total lines of code: 9";
const NEW_CANDIDATES_NO_NOSEC_LINES: &str = "Total lines skipped (#nosec): 0";
const NEW_CANDIDATES_SKIP_NOSEC_LINES: &str = "Total lines skipped (#nosec): 3";
const BASELINE_NO_SKIPPED_FILES: &str = "Files skipped (0):";
const BASELINE_NO_ISSUES_FOUND: &str = "No issues identified.";
const XML_SAX_ISSUE_ID: &str = "Issue: [B317:blacklist]";
const YAML_LOAD_ISSUE_ID: &str = "Issue: [B506:yaml_load]";
const SHELL_ISSUE_ID: &str = "Issue: [B602:subprocess_popen_with_shell_equals_true]";
const CANDIDATE_EXAMPLE_ONE: &str = "subprocess.Popen('/bin/ls *', shell=True)";
const CANDIDATE_EXAMPLE_TWO: &str = "subprocess.Popen('/bin/ls *', shell=True) # nosec";
const CANDIDATE_EXAMPLE_THREE: &str = "y = yaml.load(temp_str)";
const CANDIDATE_EXAMPLE_FOUR: &str = "y = yaml.load(temp_str) # nosec";
const CANDIDATE_EXAMPLE_FIVE: &str = "xml.sax.make_parser()";
const CANDIDATE_EXAMPLE_SIX: &str = "xml.sax.make_parser() # nosec";

fn examples_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn bandit_bin() -> &'static str {
    env!("CARGO_BIN_EXE_bandit")
}

/// Port of `BaselineFunctionalTests::_create_baseline`: copies `examples/<value>` over
/// `<tmp>/<key>` for each pair, runs `bandit -r [--ignore-nosec] -f json -o
/// <tmp>/baseline_report.json <tmp>`, then restores `examples/<key>` over `<tmp>/<key>`.
/// Returns the target directory and the return code of the baseline-creation run.
fn create_baseline(pairs: &[(&str, &str)], ignore_nosec: bool) -> (tempfile::TempDir, i32) {
    let dir = tempfile::tempdir().expect("tempdir");
    let baseline_report = dir.path().join("baseline_report.json");

    for (key, value) in pairs {
        std::fs::copy(examples_path().join(value), dir.path().join(key)).expect("copy value");
    }

    let mut cmd = Command::new(bandit_bin());
    cmd.arg("-r");
    if ignore_nosec {
        cmd.arg("--ignore-nosec");
    }
    cmd.arg("-f")
        .arg("json")
        .arg("-o")
        .arg(&baseline_report)
        .arg(dir.path());
    let output = cmd.output().expect("run bandit (create baseline)");
    let rc = output.status.code().unwrap_or(-1);

    for (key, _) in pairs {
        std::fs::copy(examples_path().join(key), dir.path().join(key)).expect("restore key");
    }

    (dir, rc)
}

/// Port of `BaselineFunctionalTests::_run_bandit_baseline`: `bandit -r [--ignore-nosec] -b
/// <baseline> <dir>`. Only stdout is returned (Python reads only `stdout`, and stdout is the
/// `txt` formatter since it is not a TTY).
fn run_baseline(dir: &Path, ignore_nosec: bool) -> (String, i32) {
    let baseline_report = dir.join("baseline_report.json");
    let mut cmd = Command::new(bandit_bin());
    cmd.arg("-r");
    if ignore_nosec {
        cmd.arg("--ignore-nosec");
    }
    cmd.arg("-b").arg(&baseline_report).arg(dir);
    let output = cmd.output().expect("run bandit (baseline compare)");
    let rc = output.status.code().unwrap_or(-1);
    (String::from_utf8_lossy(&output.stdout).into_owned(), rc)
}

/// Port of `tests/functional/test_baseline.py::BaselineFunctionalTests::test_no_new_candidates`.
#[test]
fn test_no_new_candidates() {
    let (dir, baseline_code) =
        create_baseline(&[("new_candidates-all.py", "new_candidates-all.py")], false);
    assert_eq!(1, baseline_code);
    let (stdout, rc) = run_baseline(dir.path(), false);
    assert_eq!(0, rc);
    assert!(stdout.contains(NEW_CANDIDATES_ALL_TOTAL_LINES));
    assert!(stdout.contains(NEW_CANDIDATES_SKIP_NOSEC_LINES));
    assert!(stdout.contains(BASELINE_NO_SKIPPED_FILES));
    assert!(stdout.contains(BASELINE_NO_ISSUES_FOUND));
}

/// Port of `tests/functional/test_baseline.py::BaselineFunctionalTests::test_no_existing_no_new_candidates`.
#[test]
fn test_no_existing_no_new_candidates() {
    let (dir, baseline_code) = create_baseline(&[("okay.py", "okay.py")], false);
    assert_eq!(0, baseline_code);
    let (stdout, rc) = run_baseline(dir.path(), false);
    assert_eq!(0, rc);
    assert!(stdout.contains("Total lines of code: 1"));
    assert!(stdout.contains(NEW_CANDIDATES_NO_NOSEC_LINES));
    assert!(stdout.contains(BASELINE_NO_SKIPPED_FILES));
    assert!(stdout.contains(BASELINE_NO_ISSUES_FOUND));
}

/// Port of `tests/functional/test_baseline.py::BaselineFunctionalTests::test_no_existing_with_new_candidates`.
#[test]
fn test_no_existing_with_new_candidates() {
    let (dir, baseline_code) = create_baseline(
        &[("new_candidates-all.py", "new_candidates-none.py")],
        false,
    );
    assert_eq!(0, baseline_code);
    let (stdout, rc) = run_baseline(dir.path(), false);
    assert_eq!(1, rc);
    assert!(stdout.contains(NEW_CANDIDATES_ALL_TOTAL_LINES));
    assert!(stdout.contains(NEW_CANDIDATES_SKIP_NOSEC_LINES));
    assert!(stdout.contains(BASELINE_NO_SKIPPED_FILES));
    assert!(stdout.contains(XML_SAX_ISSUE_ID));
    assert!(stdout.contains(YAML_LOAD_ISSUE_ID));
    assert!(stdout.contains(SHELL_ISSUE_ID));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_ONE));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_THREE));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_FIVE));
}

/// Port of `tests/functional/test_baseline.py::BaselineFunctionalTests::test_existing_and_new_candidates`.
#[test]
fn test_existing_and_new_candidates() {
    let (dir, baseline_code) = create_baseline(
        &[("new_candidates-all.py", "new_candidates-some.py")],
        false,
    );
    assert_eq!(1, baseline_code);
    let (stdout, rc) = run_baseline(dir.path(), false);
    assert_eq!(1, rc);
    assert!(stdout.contains(NEW_CANDIDATES_ALL_TOTAL_LINES));
    assert!(stdout.contains(NEW_CANDIDATES_SKIP_NOSEC_LINES));
    assert!(stdout.contains(BASELINE_NO_SKIPPED_FILES));
    assert!(stdout.contains(XML_SAX_ISSUE_ID));
    assert!(stdout.contains(YAML_LOAD_ISSUE_ID));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_THREE));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_FIVE));
}

/// Port of `tests/functional/test_baseline.py::BaselineFunctionalTests::test_no_new_candidates_include_nosec`.
#[test]
fn test_no_new_candidates_include_nosec() {
    let (dir, baseline_code) =
        create_baseline(&[("new_candidates-all.py", "new_candidates-all.py")], true);
    assert_eq!(1, baseline_code);
    let (stdout, rc) = run_baseline(dir.path(), true);
    assert_eq!(0, rc);
    assert!(stdout.contains(NEW_CANDIDATES_ALL_TOTAL_LINES));
    assert!(stdout.contains(NEW_CANDIDATES_NO_NOSEC_LINES));
    assert!(stdout.contains(BASELINE_NO_SKIPPED_FILES));
    assert!(stdout.contains(BASELINE_NO_ISSUES_FOUND));
}

/// Port of `tests/functional/test_baseline.py::BaselineFunctionalTests::test_new_candidates_include_nosec_only_nosecs`.
#[test]
fn test_new_candidates_include_nosec_only_nosecs() {
    let (dir, baseline_code) = create_baseline(
        &[("new_candidates-nosec.py", "new_candidates-none.py")],
        true,
    );
    assert_eq!(0, baseline_code);
    let (stdout, rc) = run_baseline(dir.path(), true);
    assert_eq!(1, rc);
    assert!(stdout.contains(NEW_CANDIDATES_SOME_TOTAL_LINES));
    assert!(stdout.contains(NEW_CANDIDATES_NO_NOSEC_LINES));
    assert!(stdout.contains(BASELINE_NO_SKIPPED_FILES));
    assert!(stdout.contains(XML_SAX_ISSUE_ID));
    assert!(stdout.contains(YAML_LOAD_ISSUE_ID));
    assert!(stdout.contains(SHELL_ISSUE_ID));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_TWO));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_FOUR));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_SIX));
}

/// Port of `tests/functional/test_baseline.py::BaselineFunctionalTests::test_new_candidates_include_nosec_new_nosecs`.
#[test]
fn test_new_candidates_include_nosec_new_nosecs() {
    let (dir, baseline_code) =
        create_baseline(&[("new_candidates-all.py", "new_candidates-none.py")], true);
    assert_eq!(0, baseline_code);
    let (stdout, rc) = run_baseline(dir.path(), true);
    assert_eq!(1, rc);
    assert!(stdout.contains(NEW_CANDIDATES_ALL_TOTAL_LINES));
    assert!(stdout.contains(NEW_CANDIDATES_NO_NOSEC_LINES));
    assert!(stdout.contains(BASELINE_NO_SKIPPED_FILES));
    assert!(stdout.contains(XML_SAX_ISSUE_ID));
    assert!(stdout.contains(YAML_LOAD_ISSUE_ID));
    assert!(stdout.contains(SHELL_ISSUE_ID));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_ONE));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_TWO));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_THREE));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_FOUR));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_FIVE));
    assert!(stdout.contains(CANDIDATE_EXAMPLE_SIX));
}
