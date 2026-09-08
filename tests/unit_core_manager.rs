//! Port of `tests/unit/core/test_manager.py` (`BanditManager`, découverte, baseline).
//!
//! Work package: `docs/plan/wp/WP-06-unit-core-manager.md`.
//! Python reference: `/home/user/bandit/tests/unit/core/test_manager.py`.
//!
//! The Python suite mocks `os.walk`, `os.path.isdir`, `_get_files_from_dir`
//! and `_is_file_included` to isolate `discover_files` from the real
//! filesystem. Rust has no equivalent of `unittest.mock.patch` for free
//! functions, so every discovery test below is adapted to use a real
//! `tempfile::TempDir` (absolute paths — tests run in parallel, so we never
//! call `set_current_dir`) instead: the directories/files created reproduce
//! exactly what the mocked `os.path.isdir`/`os.walk` calls would have
//! returned, so the assertions stay equivalent to the Python originals.

use std::sync::Mutex;

use banditrs::constants::Rank;
use banditrs::core::config::{BanditConfig, Profile};
use banditrs::core::discover;
use banditrs::core::issue::{BaselineIssue, Cwe, Issue};
use banditrs::core::manager::{self, AggType, Manager};
use banditrs::core::test_set::TestSet;
use banditrs::formatters::{self, Output};
use indexmap::IndexSet;

fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// `ManagerTests.setUp`: `BanditManager(config=BanditConfig(), agg_type="file",
/// debug=False, verbose=False)` (no profile → `{}`, i.e. every registered test).
fn base_manager() -> Manager {
    let config = BanditConfig::default();
    let profile = Profile::default();
    let test_set = TestSet::new(&config, &profile);
    Manager::new(config, AggType::File, test_set)
}

/// `ManagerTests._get_issue_instance` (`sev`/`conf` default to MEDIUM,
/// `cwe=Cwe.MULTIPLE_BINDS`, `fname="code.py"`, `test="bandit_plugin"`,
/// `lineno=1`; `test_id` keeps `Issue.__init__`'s own default, `""`).
fn get_issue_instance(sev: Rank, conf: Rank) -> Issue {
    Issue::new(
        sev,
        conf,
        Cwe::MULTIPLE_BINDS,
        "Test issue",
        "code.py",
        "bandit_plugin",
        "",
        1,
    )
}

/// Order-sensitive comparison of two `&Issue` slices using `Issue::same_signature`
/// (the Rust equivalent of Python's `Issue.__eq__`, since `Issue` carries a
/// `SourceFile` handle and does not derive `PartialEq`).
fn assert_issues_eq(actual: &[&Issue], expected: &[&Issue]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "actual={actual:?} expected={expected:?}"
    );
    for (a, e) in actual.iter().zip(expected.iter()) {
        assert!(a.same_signature(e), "actual={a} expected={e}");
    }
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_create_manager`.
#[test]
fn test_create_manager() {
    let mgr = base_manager();
    assert!(!mgr.debug);
    assert!(!mgr.verbose);
    assert_eq!(mgr.agg_type, AggType::File);
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_create_manager_with_profile`.
///
/// The Python profile lists test *names* (`any_other_function_with_shell_equals_true`,
/// `assert_used`), while `BanditTestSet._get_filter` matches plugin *ids* — so
/// upstream this profile selects nothing and the test only checks that
/// construction succeeds. Same here: `TestSet::new` filters `PLUGINS` by id, so
/// these names simply match no plugin.
#[test]
fn test_create_manager_with_profile() {
    let config = BanditConfig::default();
    let profile = Profile {
        include: IndexSet::from_iter(strs(&[
            "any_other_function_with_shell_equals_true",
            "assert_used",
        ])),
        exclude: IndexSet::new(),
        blacklist: None,
    };
    let test_set = TestSet::new(&config, &profile);
    let mgr = Manager::new(config, AggType::File, test_set);
    assert!(!mgr.debug);
    assert!(!mgr.verbose);
    assert_eq!(mgr.agg_type, AggType::File);
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_matches_globlist`.
#[test]
fn test_matches_globlist() {
    assert!(discover::matches_glob_list("test", &strs(&["*tes*"])));
    assert!(!discover::matches_glob_list("test", &strs(&["*fes*"])));
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_is_file_included`.
#[test]
fn test_is_file_included() {
    let a = discover::is_file_included("a.py", &strs(&["*.py"]), &[], true);
    let b = discover::is_file_included("a.dd", &strs(&["*.py"]), &[], false);
    let c = discover::is_file_included("a.py", &strs(&["*.py"]), &strs(&["a.py"]), true);
    let d = discover::is_file_included("a.dd", &strs(&["*.py"]), &[], true);
    let e = discover::is_file_included("x_a.py", &strs(&["*.py"]), &strs(&["x_*.py"]), true);
    let f = discover::is_file_included("x.py", &strs(&["*.py"]), &strs(&["x_*.py"]), true);
    assert!(a);
    assert!(b);
    assert!(!c);
    assert!(!d);
    assert!(!e);
    assert!(f);
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_get_files_from_dir`
/// (adapted, see WP): `os.walk` mocked as `("/", ("a"), ())`, `("/a", (), ("a.py",
/// "b.py", "c.ww"))` becomes a real `<tmp>/a/{a.py,b.py,c.ww}` tree, walked from
/// `<tmp>`; results are sorted before comparison since `os.walk`/`fs::read_dir`
/// ordering isn't guaranteed either way (the Python test compares Python `set`s).
#[test]
fn test_get_files_from_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let a_dir = tmp.path().join("a");
    std::fs::create_dir(&a_dir).unwrap();
    std::fs::write(a_dir.join("a.py"), "").unwrap();
    std::fs::write(a_dir.join("b.py"), "").unwrap();
    std::fs::write(a_dir.join("c.ww"), "").unwrap();

    let (mut inc, mut exc) =
        discover::get_files_from_dir(&tmp.path().to_string_lossy(), &strs(&["*.py"]), &[]);
    inc.sort();
    exc.sort();

    assert_eq!(
        inc,
        vec![
            a_dir.join("a.py").to_string_lossy().into_owned(),
            a_dir.join("b.py").to_string_lossy().into_owned(),
        ]
    );
    assert_eq!(exc, vec![a_dir.join("c.ww").to_string_lossy().into_owned()]);
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_populate_baseline_success`.
#[test]
fn test_populate_baseline_success() {
    let mut mgr = base_manager();
    let baseline_data = r#"{
        "results": [
            {
                "code": "test code",
                "filename": "example_file.py",
                "issue_severity": "low",
                "issue_cwe": {
                    "id": 605,
                    "link": "https://cwe.mitre.org/data/definitions/605.html"
                },
                "issue_confidence": "low",
                "issue_text": "test issue",
                "test_name": "some_test",
                "test_id": "x",
                "line_number": "n",
                "line_range": "n-m"
            }
        ]
    }"#;
    let issue_dictionary = serde_json::json!({
        "code": "test code",
        "filename": "example_file.py",
        "issue_severity": "low",
        "issue_cwe": Cwe::MULTIPLE_BINDS.as_dict(),
        "issue_confidence": "low",
        "issue_text": "test issue",
        "test_name": "some_test",
        "test_id": "x",
        "line_number": "n",
        "line_range": "n-m",
    });
    let expected = BaselineIssue::from_dict(&issue_dictionary).unwrap();

    mgr.populate_baseline(baseline_data);
    assert_eq!(mgr.baseline, vec![expected]);
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_populate_baseline_invalid_json`
/// (adapted, see WP): Python mocks `logging.Logger.warning`; here we capture the
/// buffered log entries with `log::with_buffer` instead. The global log level is
/// serialized with a file-local mutex since it's process-wide state and tests run
/// in parallel.
#[test]
fn test_populate_baseline_invalid_json() {
    static LEVEL_GUARD: Mutex<()> = Mutex::new(());
    let _guard = LEVEL_GUARD.lock().unwrap();

    let mut mgr = base_manager();
    let (_, entries) = banditrs::log::with_buffer(|| {
        banditrs::log::set_level(banditrs::log::Level::Warning);
        mgr.populate_baseline(r#"{"data": "bad"}"#);
    });

    assert!(mgr.baseline.is_empty());
    assert!(
        entries
            .iter()
            .any(|e| e.level == banditrs::log::Level::Warning)
    );
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_results_count`.
#[test]
fn test_results_count() {
    let mut mgr = base_manager();
    let levels = [Rank::Low, Rank::Medium, Rank::High];
    mgr.results = levels
        .iter()
        .map(|&level| Issue::new(level, level, Cwe::MULTIPLE_BINDS, "", "", "", "", 1))
        .collect();

    let r: Vec<usize> = levels
        .iter()
        .map(|&level| mgr.results_count(level, level))
        .collect();

    assert_eq!(r, vec![3, 2, 1]);
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_output_results_invalid_format`.
#[test]
fn test_output_results_invalid_format() {
    let mgr = base_manager();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("_temp_output");
    let file = std::fs::File::create(&path).unwrap();
    let mut output = Output::File {
        name: path.to_string_lossy().into_owned(),
        file,
    };

    let result =
        formatters::output_results(&mgr, 5, Rank::Low, Rank::Low, &mut output, "invalid", None);

    assert!(result.is_ok());
    assert!(path.is_file());
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_output_results_valid_format`.
#[test]
fn test_output_results_valid_format() {
    let mgr = base_manager();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("_temp_output.txt");
    let file = std::fs::File::create(&path).unwrap();
    let mut output = Output::File {
        name: path.to_string_lossy().into_owned(),
        file,
    };

    let result =
        formatters::output_results(&mgr, 5, Rank::Low, Rank::Low, &mut output, "txt", None);

    assert!(result.is_ok());
    assert!(path.is_file());
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_discover_files_recurse_skip`
/// (adapted, see WP): `os.path.isdir` mocked `True` becomes a real (empty)
/// `TempDir`; without `-r`, `discover_files` only logs and touches neither list.
#[test]
fn test_discover_files_recurse_skip() {
    let mut mgr = base_manager();
    let tmp = tempfile::tempdir().unwrap();
    mgr.discover_files(&[tmp.path().to_string_lossy().into_owned()], false, None);
    assert!(mgr.files_list.is_empty());
    assert!(mgr.excluded_files.is_empty());
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_discover_files_recurse_files`
/// (adapted, see WP): `_get_files_from_dir` mocked to return `({"files"}, {"excluded"})`
/// becomes a real `TempDir` containing one included (`files.py`) and one excluded
/// (`excluded.ww`) file.
#[test]
fn test_discover_files_recurse_files() {
    let mut mgr = base_manager();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("files.py"), "").unwrap();
    std::fs::write(tmp.path().join("excluded.ww"), "").unwrap();

    mgr.discover_files(&[tmp.path().to_string_lossy().into_owned()], true, None);

    assert_eq!(
        mgr.files_list,
        vec![tmp.path().join("files.py").to_string_lossy().into_owned()]
    );
    assert_eq!(
        mgr.excluded_files,
        vec![
            tmp.path()
                .join("excluded.ww")
                .to_string_lossy()
                .into_owned()
        ]
    );
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_discover_files_exclude`
/// (adapted, see WP): `os.path.isdir` mocked `False` and `_is_file_included` mocked
/// `False` become a real (nonexistent) file path excluded by naming itself on the
/// command line — the real `is_file_included` excludes it the same way, by exact
/// glob match.
#[test]
fn test_discover_files_exclude() {
    let mut mgr = base_manager();
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("thing.py").to_string_lossy().into_owned();

    mgr.discover_files(std::slice::from_ref(&target), true, Some(target.as_str()));

    assert!(mgr.files_list.is_empty());
    assert_eq!(mgr.excluded_files, vec![target]);
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_discover_files_exclude_dir`
/// (adapted, see WP): the `os.path.isdir` mock sequence for each of the four cases
/// is reproduced by a real `<tmp>/x/y.py` + `<tmp>/x/y/z.py` tree, so `Path::is_dir`
/// returns exactly what each mock would have.
#[test]
fn test_discover_files_exclude_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let x = tmp.path().join("x");
    std::fs::create_dir_all(x.join("y")).unwrap();
    std::fs::write(x.join("y.py"), "").unwrap();
    std::fs::write(x.join("y").join("z.py"), "").unwrap();
    let y_py = x.join("y.py").to_string_lossy().into_owned();
    let z_py = x.join("y").join("z.py").to_string_lossy().into_owned();
    let x_str = x.to_string_lossy().into_owned();

    let mut mgr = base_manager();

    // Exclude dir using a wildcard.
    mgr.discover_files(
        std::slice::from_ref(&y_py),
        true,
        Some(&format!("{x_str}/*")),
    );
    assert!(mgr.files_list.is_empty());
    assert_eq!(mgr.excluded_files, vec![y_py.clone()]);

    // Exclude dir without a wildcard (trailing slash).
    mgr.discover_files(
        std::slice::from_ref(&y_py),
        true,
        Some(&format!("{x_str}/")),
    );
    assert!(mgr.files_list.is_empty());
    assert_eq!(mgr.excluded_files, vec![y_py.clone()]);

    // Exclude dir without wildcard or trailing slash.
    mgr.discover_files(std::slice::from_ref(&y_py), true, Some(&x_str));
    assert!(mgr.files_list.is_empty());
    assert_eq!(mgr.excluded_files, vec![y_py.clone()]);

    // Exclude by substring, no prefix or suffix.
    mgr.discover_files(std::slice::from_ref(&z_py), true, Some("y"));
    assert!(mgr.files_list.is_empty());
    assert_eq!(mgr.excluded_files, vec![z_py]);
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_discover_files_exclude_cmdline`
/// (adapted, see WP): Python only asserts the arguments `_is_file_included` was
/// called with (`enforce_glob=False`); here we run the real (unmocked) function and
/// assert its effect on `files_list`/`excluded_files` directly, which is what that
/// call would have produced given a truthy mock return value.
#[test]
fn test_discover_files_exclude_cmdline() {
    let mut mgr = base_manager();
    mgr.discover_files(&strs(&["a", "b", "c"]), true, Some("a,b"));
    assert_eq!(mgr.excluded_files, strs(&["a", "b"]));
    assert_eq!(mgr.files_list, strs(&["./c"]));
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_discover_files_exclude_glob`.
#[test]
fn test_discover_files_exclude_glob() {
    let mut mgr = base_manager();
    mgr.discover_files(
        &strs(&["a.py", "test_a.py", "test.py"]),
        true,
        Some("test_*.py"),
    );
    assert_eq!(mgr.files_list, strs(&["./a.py", "./test.py"]));
    assert_eq!(mgr.excluded_files, strs(&["test_a.py"]));
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_discover_files_include`
/// (adapted, see WP): `_is_file_included` mocked `True` becomes the real function
/// evaluated on a nonexistent, extension-less target — which it also includes,
/// since `enforce_glob=False` for command-line targets.
#[test]
fn test_discover_files_include() {
    let mut mgr = base_manager();
    mgr.discover_files(&strs(&["thing"]), true, None);
    assert_eq!(mgr.files_list, strs(&["./thing"]));
    assert!(mgr.excluded_files.is_empty());
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_run_tests_ioerror`.
#[test]
fn test_run_tests_ioerror() {
    let mut mgr = base_manager();
    let tmp = tempfile::tempdir().unwrap();
    let no_such_file = tmp
        .path()
        .join("no_such_file.py")
        .to_string_lossy()
        .into_owned();
    mgr.files_list = vec![no_such_file.clone()];

    mgr.run_tests();

    let skipped_str = format!("{:?}", mgr.skipped);
    assert!(skipped_str.contains(&no_such_file));
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_compare_baseline`.
#[test]
fn test_compare_baseline() {
    let mut issue_a = get_issue_instance(Rank::Medium, Rank::Medium);
    issue_a.fname = "file1.py".to_string();
    let mut issue_b = get_issue_instance(Rank::Medium, Rank::Medium);
    issue_b.fname = "file2.py".to_string();
    let mut issue_c = get_issue_instance(Rank::High, Rank::Medium);
    issue_c.fname = "file1.py".to_string();

    // issue_c is in results, not in baseline.
    assert_issues_eq(
        &manager::compare_baseline_results(&[&issue_a, &issue_b], &[&issue_a, &issue_b, &issue_c]),
        &[&issue_c],
    );

    // baseline and results are the same.
    assert_issues_eq(
        &manager::compare_baseline_results(
            &[&issue_a, &issue_b, &issue_c],
            &[&issue_a, &issue_b, &issue_c],
        ),
        &[],
    );

    // results are better than baseline.
    assert_issues_eq(
        &manager::compare_baseline_results(&[&issue_a, &issue_b, &issue_c], &[&issue_a, &issue_b]),
        &[],
    );
}

/// Port of `tests/unit/core/test_manager.py::ManagerTests::test_find_candidate_matches`.
#[test]
fn test_find_candidate_matches() {
    let issue_a = get_issue_instance(Rank::Medium, Rank::Medium);
    let issue_b = get_issue_instance(Rank::Medium, Rank::Medium);
    let mut issue_c = get_issue_instance(Rank::Medium, Rank::Medium);
    issue_c.fname = "file1.py".to_string();

    // issue_a and issue_b are the same, both should be returned as candidates.
    let m = manager::find_candidate_matches(&[&issue_a], &[&issue_a, &issue_b]);
    assert_eq!(m.len(), 1);
    assert!(m[0].0.same_signature(&issue_a));
    assert_issues_eq(&m[0].1, &[&issue_a, &issue_b]);

    // issue_a and issue_c are different, only issue_a should be returned.
    let m = manager::find_candidate_matches(&[&issue_a], &[&issue_a, &issue_c]);
    assert_eq!(m.len(), 1);
    assert!(m[0].0.same_signature(&issue_a));
    assert_issues_eq(&m[0].1, &[&issue_a]);

    // issue_c doesn't match issue_a, an empty list should be returned.
    let m = manager::find_candidate_matches(&[&issue_a], &[&issue_c]);
    assert_eq!(m.len(), 1);
    assert!(m[0].0.same_signature(&issue_a));
    assert_issues_eq(&m[0].1, &[]);

    // issue_a and issue_b match, both should return issue_a and issue_b as candidates.
    let m = manager::find_candidate_matches(&[&issue_a, &issue_b], &[&issue_a, &issue_b, &issue_c]);
    assert_eq!(m.len(), 2);
    assert!(m[0].0.same_signature(&issue_a));
    assert_issues_eq(&m[0].1, &[&issue_a, &issue_b]);
    assert!(m[1].0.same_signature(&issue_b));
    assert_issues_eq(&m[1].1, &[&issue_a, &issue_b]);
}
