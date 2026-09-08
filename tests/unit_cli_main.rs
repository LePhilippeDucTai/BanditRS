//! Port of `tests/unit/cli/test_main.py` (`bandit.cli.main`).
//!
//! Work package: `docs/plan/wp/WP-03-unit-cli-main.md`.
//! Python reference: `/home/user/bandit/tests/unit/cli/test_main.py`.
//!
//! The Python suite forces branches with `unittest.mock` (mocked `sys.argv`, mocked
//! `BanditManager` methods, a mocked `logging.getLogger()` fixture per test class). BanditRS's
//! logger and CLI are process-global, so instead: the `_init_logger`/`_get_options_from_ini`/
//! `_log_option_source` unit tests call the exposed `banditrs::cli::main` functions directly
//! (serialized on `LOGGER_LOCK` — they share the same global logger state, like
//! `tests/unit_cli_config_generator.rs` does), and the `main()` tests run the real `bandit` binary
//! in a temporary directory (`Command::current_dir`, never `std::env::set_current_dir`, so the
//! tests stay safe to run in parallel) with the situation that actually triggers the branch the
//! Python test mocks.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use banditrs::cli::main;
use banditrs::log::{self, Level};

/// `test_init_logger`/`test_init_logger_debug_mode` mutate the process-global logger state;
/// serialize them so they don't race other tests in this file.
static LOGGER_LOCK: Mutex<()> = Mutex::new(());

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn example(name: &str) -> String {
    examples().join(name).to_string_lossy().into_owned()
}

fn bandit_bin() -> &'static str {
    env!("CARGO_BIN_EXE_bandit")
}

/// Runs the `bandit` binary with `dir` as its current directory and returns
/// `(exit code, stdout + stderr)`.
fn run_in(dir: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(bandit_bin())
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .expect("run bandit");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

// upstream: tests/unit/cli/test_main.py `bandit_config_content` (lines 15-30), verbatim.
const BANDIT_CONFIG_CONTENT: &str = "
include:
    - '*.py'
    - '*.pyw'

profiles:
    test:
        include:
            - start_process_with_a_shell

shell_injection:
    subprocess:

    shell:
        - os.system
";

// upstream: tests/unit/cli/test_main.py `bandit_baseline_content` (lines 32-47), verbatim.
const BANDIT_BASELINE_CONTENT: &str = "{
    \"results\": [
        {
            \"code\": \"some test code\",
            \"filename\": \"test_example.py\",
            \"issue_severity\": \"low\",
            \"issue_confidence\": \"low\",
            \"issue_text\": \"test_issue\",
            \"test_name\": \"some_test\",
            \"test_id\": \"x\",
            \"line_number\": \"n\",
            \"line_range\": \"n-m\"
        }
    ]
}
";

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainLoggerTests::test_init_logger`.
#[test]
fn test_init_logger() {
    let _guard = LOGGER_LOCK.lock().unwrap();
    main::init_logger(Level::Info, None);
    assert_eq!(Level::Info, log::level());
    assert!(log::enabled(Level::Info));
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainLoggerTests::test_init_logger_debug_mode`.
#[test]
fn test_init_logger_debug_mode() {
    let _guard = LOGGER_LOCK.lock().unwrap();
    main::init_logger(Level::Debug, None);
    assert_eq!(Level::Debug, log::level());
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_get_options_from_ini_no_ini_path_no_target`.
#[test]
fn test_get_options_from_ini_no_ini_path_no_target() {
    assert_eq!(Ok(None), main::get_options_from_ini(None, &[]));
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_get_options_from_ini_empty_directory_no_target`.
#[test]
fn test_get_options_from_ini_empty_directory_no_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ini_directory = dir.path().to_string_lossy().into_owned();
    assert_eq!(
        Ok(None),
        main::get_options_from_ini(Some(ini_directory.as_str()), &[])
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_get_options_from_ini_no_ini_path_no_bandit_files`.
#[test]
fn test_get_options_from_ini_no_ini_path_no_bandit_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target_directory = dir.path().to_string_lossy().into_owned();
    assert_eq!(
        Ok(None),
        main::get_options_from_ini(None, &[target_directory])
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_get_options_from_ini_no_ini_path_multi_bandit_files`.
#[test]
fn test_get_options_from_ini_no_ini_path_multi_bandit_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let second_config = dir.path().join("second_config_directory");
    std::fs::create_dir(&second_config).expect("mkdir second_config_directory");
    std::fs::write(dir.path().join(".bandit"), BANDIT_CONFIG_CONTENT).expect("write .bandit");
    std::fs::write(second_config.join(".bandit"), BANDIT_CONFIG_CONTENT).expect("write .bandit");

    let target_directory = dir.path().to_string_lossy().into_owned();
    match main::get_options_from_ini(None, &[target_directory]) {
        Err(main::MultipleIniFiles(paths)) => assert_eq!(2, paths.len()),
        other => panic!("expected Err(MultipleIniFiles), got {other:?}"),
    }

    // The binary itself: no `--ini` given, two `.bandit` files under the target -> exit 2 with
    // the message `_get_options_from_ini` logs before `sys.exit(2)`.
    let (rc, out) = run_in(dir.path(), &["."]);
    assert_eq!(2, rc);
    assert!(
        out.contains("Multiple .bandit files found"),
        "missing message in {out}"
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_log_option_source_arg_val`.
#[test]
fn test_log_option_source_arg_val() {
    let arg_val = Some("file");
    let ini_val = Some("vuln");
    let option_name = "aggregate";
    for default_val in [None, Some("default")] {
        assert_eq!(
            Some("file".to_string()),
            main::log_option_source(default_val, arg_val, ini_val, option_name)
        );
    }
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_log_option_source_ini_value`.
#[test]
fn test_log_option_source_ini_value() {
    assert_eq!(
        Some("vuln".to_string()),
        main::log_option_source(None, None, Some("vuln"), "aggregate")
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_log_option_source_ini_val_with_str_default_and_no_arg_val`.
#[test]
fn test_log_option_source_ini_val_with_str_default_and_no_arg_val() {
    assert_eq!(
        Some("vuln".to_string()),
        main::log_option_source(Some("file"), Some("file"), Some("vuln"), "aggregate")
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_log_option_source_no_values`.
#[test]
fn test_log_option_source_no_values() {
    assert_eq!(None, main::log_option_source(None, None, None, "aggregate"));
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_config_unopenable`.
#[test]
fn test_main_config_unopenable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (rc, out) = run_in(dir.path(), &["-c", "bandit.yaml", "test"]);
    assert_eq!(2, rc);
    assert!(
        out.contains("bandit.yaml : Could not read config file."),
        "{out}"
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_invalid_config`.
#[test]
fn test_main_invalid_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bandit.yaml"), "- [ something").expect("write bandit.yaml");
    let (rc, out) = run_in(dir.path(), &["-c", "bandit.yaml", "test"]);
    assert_eq!(2, rc);
    assert!(out.contains("Error parsing file."), "{out}");
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_handle_ini_options`
/// (adapted): the Python test mocks `_get_options_from_ini` to return
/// `{"exclude": "/tmp", "skips": "skip_test", "tests": "some_test"}`; here a real `.bandit` file
/// with the same content is read through `--ini`, exercising the same
/// `apply_ini_options`/`extension_mgr.validate_profile` path with an equivalent end state (the
/// unknown `some_test`/`skip_test` ids leave no test active, so the manager still errors out).
#[test]
fn test_main_handle_ini_options() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bandit.yaml"), BANDIT_CONFIG_CONTENT).expect("write");
    std::fs::write(
        dir.path().join(".bandit"),
        "[bandit]\nexclude = /tmp\nskips = skip_test\ntests = some_test\n",
    )
    .expect("write .bandit");
    let (rc, out) = run_in(
        dir.path(),
        &["-c", "bandit.yaml", "--ini", ".bandit", "test"],
    );
    assert_eq!(2, rc);
    assert!(
        out.contains("No tests would be run, please check the profile."),
        "{out}"
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_profile_not_found`.
#[test]
fn test_main_profile_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bandit.yaml"), BANDIT_CONFIG_CONTENT).expect("write");
    let (rc, out) = run_in(dir.path(), &["-c", "bandit.yaml", "-p", "bad", "test"]);
    assert_eq!(2, rc);
    assert!(
        out.contains("Unable to find profile (bad) in config file: bandit.yaml"),
        "{out}"
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_baseline_ioerror`
/// (adapted): the Python test mocks `BanditManager.populate_baseline` to raise `IOError`; here
/// `base.json` is a real directory, so `open(args.baseline)` (`std::fs::read_to_string`) fails
/// with the same `OSError`/`Err` branch bandit's `main` already catches.
#[test]
fn test_main_baseline_ioerror() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bandit.yaml"), BANDIT_CONFIG_CONTENT).expect("write");
    std::fs::create_dir(dir.path().join("base.json")).expect("mkdir base.json");
    let (rc, out) = run_in(
        dir.path(),
        &["-c", "bandit.yaml", "-b", "base.json", "test"],
    );
    assert_eq!(2, rc);
    assert!(
        out.contains("Could not open baseline report: base.json"),
        "{out}"
    );
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_invalid_output_format`.
#[test]
fn test_main_invalid_output_format() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bandit.yaml"), BANDIT_CONFIG_CONTENT).expect("write");
    std::fs::write(dir.path().join("base.json"), BANDIT_BASELINE_CONTENT).expect("write");
    let (rc, _out) = run_in(
        dir.path(),
        &["-c", "bandit.yaml", "-b", "base.json", "-f", "csv", "test"],
    );
    assert_eq!(2, rc);
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_exit_with_results`
/// (adapted): the Python test mocks `BanditManager.results_count` to return `1`; here the target
/// is a real file with a genuine issue (`examples/os_system.py`), which drives `results_count`
/// to the same nonzero value through the real scan.
#[test]
fn test_main_exit_with_results() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bandit.yaml"), BANDIT_CONFIG_CONTENT).expect("write");
    let target = example("os_system.py");
    let (rc, _out) = run_in(dir.path(), &["-c", "bandit.yaml", &target, "-o", "output"]);
    assert_eq!(1, rc);
    assert!(dir.path().join("output").is_file());
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_exit_with_no_results`
/// (adapted): the Python test mocks `BanditManager.results_count` to return `0`; here the target
/// is a real file with no issue (`examples/okay.py`).
#[test]
fn test_main_exit_with_no_results() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bandit.yaml"), BANDIT_CONFIG_CONTENT).expect("write");
    let target = example("okay.py");
    let (rc, _out) = run_in(dir.path(), &["-c", "bandit.yaml", &target, "-o", "output"]);
    assert_eq!(0, rc);
}

/// Port of `tests/unit/cli/test_main.py::BanditCLIMainTests::test_main_exit_with_results_and_with_exit_zero_flag`
/// (adapted, same substitution as `test_main_exit_with_results`).
#[test]
fn test_main_exit_with_results_and_with_exit_zero_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bandit.yaml"), BANDIT_CONFIG_CONTENT).expect("write");
    let target = example("os_system.py");
    let (rc, _out) = run_in(
        dir.path(),
        &["-c", "bandit.yaml", &target, "-o", "output", "--exit-zero"],
    );
    assert_eq!(0, rc);
}
