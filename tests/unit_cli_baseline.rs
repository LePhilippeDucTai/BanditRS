//! Port of `tests/unit/cli/test_baseline.py` (`bandit-baseline`, dépôts git temporaires).
//!
//! Work package: `docs/plan/wp/WP-04-unit-cli-baseline.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/unit/cli/test_baseline.py`.
//!
//! Every test drives real, independent `TempDir` git repositories through the `git` CLI and
//! the compiled `bandit`/`bandit-baseline` binaries — never `std::env::set_current_dir` and
//! never `std::env::set_var` — so the suite stays safe to run in parallel (`cargo test`'s
//! default). Where a test needs to fake `git` or `bandit`, it does so by handing the child
//! process a restricted `PATH` / `BANDITRS_BANDIT_EXE` through `Command::env`, which only
//! affects that one spawned process.

use std::path::{Path, PathBuf};
use std::process::Command;

// upstream: tests/unit/cli/test_baseline.py `config` module constant (verbatim).
const CONFIG: &str = "\ninclude:\n    - '*.py'\n    - '*.pyw'\n\nprofiles:\n    test:\n        include:\n            - start_process_with_a_shell\n\nshell_injection:\n    subprocess: []\n    no_shell: []\n    shell:\n        - os.system\n";

fn examples_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

/// `cls.temp_file_contents` (`examples/mktemp.py`), used to fill "some file exists with
/// arbitrary content" fixtures.
fn mktemp_contents() -> String {
    std::fs::read_to_string(examples_path().join("mktemp.py")).expect("read examples/mktemp.py")
}

/// `git -C <repo> <args>`, panics on failure — a setup helper for the temporary
/// repositories these tests create, not the code under test.
fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `git.Repo.init(...)` + `git_repo.index.commit("Initial commit")`: a fresh repository
/// with one (empty) commit, `user.name`/`user.email` set so further commits succeed.
fn init_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.name", "BanditRS Test"]);
    git(
        dir.path(),
        &["config", "user.email", "banditrs-test@example.invalid"],
    );
    git(
        dir.path(),
        &["commit", "--allow-empty", "-m", "Initial commit"],
    );
    dir
}

/// Directory holding the `bandit` binary built for this `cargo test` run — parent of
/// `CARGO_BIN_EXE_bandit` — usable as a `PATH` entry.
fn bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_bandit"))
        .parent()
        .expect("bandit binary has a parent directory")
        .to_path_buf()
}

/// Runs the `bandit-baseline` binary built for this `cargo test` run with `cwd = repo`,
/// `args`, and the given extra environment variables (only for the spawned child — the
/// test process' own environment, including `PATH`, is never mutated). Returns the exit
/// code and the combined stdout+stderr (bandit-baseline's own log handler writes to
/// stdout; the inner `bandit` scan's stderr is inherited straight through).
fn run_baseline(repo: &Path, args: &[&str], env: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bandit-baseline"));
    cmd.current_dir(repo).args(args);
    for (key, value) in env {
        cmd.env(key, value);
    }
    let output = cmd.output().expect("run bandit-baseline");
    let rc = output.status.code().unwrap_or(-1);
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (rc, combined)
}

/// Absolute path to the real `git` executable found on the current `PATH`, used by the
/// fake `git` scripts below to delegate everything they don't want to intercept.
fn real_git_path() -> String {
    let out = Command::new("sh")
        .arg("-c")
        .arg("command -v git")
        .output()
        .expect("locate real git");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Writes an executable script named `name` under `dir` and returns its path.
fn write_script(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)
            .expect("script metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod script");
    }
    path
}

/// A fake `git` that delegates to `$REAL_GIT_BIN` for everything except `rev-parse HEAD`
/// (used by `baseline::main` to read the *current* commit), which it fails — reproducing
/// `test_main_git_command_failure`'s `mock.patch("git.Repo.commit")` by making the one Git
/// invocation `main()` uses to get the current commit fail, while leaving the
/// `rev-parse --is-inside-work-tree` / `status --porcelain` calls `initialize()` needs
/// (and `rev-parse HEAD^`, never reached) untouched.
const FAKE_GIT_FAIL_REV_PARSE_HEAD: &str = "#!/bin/sh\nprev=\"\"\nfor arg in \"$@\"; do\n    if [ \"$prev\" = \"rev-parse\" ] && [ \"$arg\" = \"HEAD\" ]; then\n        echo \"fake git: forced failure for rev-parse HEAD\" >&2\n        exit 1\n    fi\n    prev=\"$arg\"\ndone\nexec \"$REAL_GIT_BIN\" \"$@\"\n";

/// A fake `bandit` standing in for `BANDITRS_BANDIT_EXE`: writes to stdout and exits 3,
/// reproducing `test_main_subprocess_error`'s `mock.patch("subprocess.check_output")` with
/// a `CalledProcessError(returncode=3)`.
const FAKE_BANDIT_EXIT_3: &str = "#!/bin/sh\necho \"fake bandit output\"\nexit 3\n";

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_bandit_baseline`.
#[test]
fn test_bandit_baseline() {
    let repo = init_repo();
    let repo_path = repo.path();

    let benign_contents =
        std::fs::read_to_string(examples_path().join("okay.py")).expect("read okay.py");
    let malicious_contents =
        std::fs::read_to_string(examples_path().join("os_system.py")).expect("read os_system.py");

    std::fs::write(repo_path.join("bandit.yaml"), CONFIG).expect("write bandit.yaml");

    // `bin_dir()` on PATH so `bandit_path()`'s PATH fallback finds the `bandit` binary
    // built for this test run (see WP-04 fiche: "le PATH doit contenir bin_dir()").
    let path = format!(
        "{}:{}",
        bin_dir().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let env = [("PATH", path.as_str())];

    // upstream: tests/unit/cli/test_baseline.py `branches` (benign1 -> malicious -> benign2,
    // each branch created from the current HEAD, i.e. the previous branch's tip).
    let branches: [(&str, &[(&str, &str)], i32); 3] = [
        ("benign1", &[("benign_one.py", benign_contents.as_str())], 0),
        (
            "malicious",
            &[
                ("benign_one.py", benign_contents.as_str()),
                ("malicious.py", malicious_contents.as_str()),
            ],
            1,
        ),
        (
            "benign2",
            &[
                ("benign_one.py", benign_contents.as_str()),
                ("malicious.py", malicious_contents.as_str()),
                ("benign_two.py", benign_contents.as_str()),
            ],
            0,
        ),
    ];

    for (name, files, expected_rc) in branches {
        git(repo_path, &["checkout", "-b", name]);
        for (fname, contents) in files {
            std::fs::write(repo_path.join(fname), contents).expect("write branch file");
        }
        let mut add_args = vec!["add"];
        add_args.extend(files.iter().map(|(f, _)| *f));
        git(repo_path, &add_args);
        git(repo_path, &["commit", "-m", name]);

        let (rc, output) = run_baseline(
            repo_path,
            &["-c", "bandit.yaml", "-r", ".", "-p", "test"],
            &env,
        );
        assert_eq!(expected_rc, rc, "branch {name}: output was:\n{output}");
    }
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_init_logger`.
#[test]
fn test_init_logger() {
    banditrs::cli::baseline::init_logger();
    let logger_level = banditrs::log::level();

    // verify that logger was initialized
    assert_eq!(banditrs::log::Level::Info, logger_level);
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_initialize_dirty_repo`.
#[test]
fn test_initialize_dirty_repo() {
    let repo = init_repo();

    // make the git repo 'dirty'
    std::fs::write(repo.path().join("dirty_file.py"), mktemp_contents())
        .expect("write dirty_file.py");
    git(repo.path(), &["add", "dirty_file.py"]);

    let return_value = banditrs::cli::baseline::initialize(repo.path(), &[".".to_string()]);

    // assert bandit did not run due to dirty repo
    assert!(return_value.is_none());
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_initialize_existing_report_file`.
#[test]
fn test_initialize_existing_report_file() {
    let repo = init_repo();

    // create an existing version of output report file
    let existing_report = format!("{}.txt", banditrs::cli::baseline::REPORT_BASENAME);
    std::fs::write(repo.path().join(existing_report), mktemp_contents())
        .expect("write existing report file");

    // upstream: `@mock.patch("sys.argv", ["bandit", "-f", "txt", "test"])`.
    let bandit_args = vec!["-f".to_string(), "txt".to_string(), "test".to_string()];
    let return_value = banditrs::cli::baseline::initialize(repo.path(), &bandit_args);

    // assert bandit did not run due to existing report file
    assert!(return_value.is_none());
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_initialize_existing_temp_file`.
#[test]
fn test_initialize_existing_temp_file() {
    let repo = init_repo();

    // create an existing version of temporary output file
    std::fs::write(
        repo.path().join(banditrs::cli::baseline::BASELINE_TMP_FILE),
        mktemp_contents(),
    )
    .expect("write existing temp file");

    let return_value = banditrs::cli::baseline::initialize(repo.path(), &[".".to_string()]);

    // assert bandit did not run due to existing temporary report file
    assert!(return_value.is_none());
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_initialize_git_command_failure` (adapted, see WP).
///
/// Python mocks `git.Repo` itself to raise `GitCommandNotFound`; here we reproduce "git is
/// unavailable" for real by handing the `bandit-baseline` child a `PATH` made of a single
/// empty directory (no `git` in it) via `Command::env`, and observe the process-level
/// effect (`main()` exits 2 right after `initialize()` returns `None`) instead of calling
/// `initialize()` in-process, since faking "no git on PATH" for an in-process call would
/// require mutating the test binary's own `PATH` and break test parallelism.
#[test]
fn test_initialize_git_command_failure() {
    let repo = init_repo();
    std::fs::write(repo.path().join("additional_file.py"), mktemp_contents())
        .expect("write additional_file.py");
    git(repo.path(), &["add", "additional_file.py"]);
    git(repo.path(), &["commit", "-m", "Additional Content"]);

    let empty_path_dir = tempfile::tempdir().expect("tempdir");

    let (rc, output) = run_baseline(
        repo.path(),
        &["."],
        &[("PATH", empty_path_dir.path().to_str().expect("utf8 path"))],
    );

    // assert bandit did not run due to git command failure
    assert_eq!(2, rc, "output was:\n{output}");
    assert!(
        output.contains("Git not available") || output.contains("Git command not found"),
        "output was:\n{output}"
    );
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_initialize_no_repo`.
#[test]
fn test_initialize_no_repo() {
    let repo_directory = tempfile::tempdir().expect("tempdir");

    let return_value =
        banditrs::cli::baseline::initialize(repo_directory.path(), &[".".to_string()]);

    // assert bandit did not run due to no git repo
    assert!(return_value.is_none());
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_initialize_with_output_argument` (adapted, see WP).
///
/// Python mocks the module-level `bandit_args` global; here `bandit_args` is an ordinary
/// parameter, so the equivalent is simply passing it directly.
#[test]
fn test_initialize_with_output_argument() {
    let repo = init_repo();

    let bandit_args = vec!["-o".to_string(), "bandit_baseline_result".to_string()];
    let return_value = banditrs::cli::baseline::initialize(repo.path(), &bandit_args);

    // assert bandit did not run due to provided -o (--ouput) argument
    assert!(return_value.is_none());
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_main_git_command_failure` (adapted, see WP).
///
/// Python mocks `git.Repo.commit` to raise `GitCommandError`; here a fake `git` in front of
/// `PATH` delegates to the real `git` for everything except the `rev-parse HEAD` call
/// `main()` uses to read the current commit, which it fails.
#[test]
fn test_main_git_command_failure() {
    let repo = init_repo();
    std::fs::write(repo.path().join("additional_file.py"), mktemp_contents())
        .expect("write additional_file.py");
    git(repo.path(), &["add", "additional_file.py"]);
    git(repo.path(), &["commit", "-m", "Additional Content"]);

    let fake_git_dir = tempfile::tempdir().expect("tempdir");
    write_script(fake_git_dir.path(), "git", FAKE_GIT_FAIL_REV_PARSE_HEAD);
    let real_git = real_git_path();
    let path = format!(
        "{}:{}",
        fake_git_dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let (rc, output) = run_baseline(
        repo.path(),
        &["."],
        &[("PATH", path.as_str()), ("REAL_GIT_BIN", real_git.as_str())],
    );

    // assert the system exits with code 2
    assert_eq!(2, rc, "output was:\n{output}");
    assert!(
        output.contains("Unable to get current or parent commit"),
        "output was:\n{output}"
    );
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_main_no_parent_commit`.
#[test]
fn test_main_no_parent_commit() {
    let repo = init_repo();

    let (rc, output) = run_baseline(repo.path(), &["."], &[]);

    // assert the system exits with code 2
    assert_eq!(2, rc, "output was:\n{output}");
    assert!(
        output.contains("Parent commit not available"),
        "output was:\n{output}"
    );
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_main_non_repo`.
#[test]
fn test_main_non_repo() {
    let repo_dir = tempfile::tempdir().expect("tempdir");

    let (rc, output) = run_baseline(repo_dir.path(), &["."], &[]);

    // assert the system exits with code 2
    assert_eq!(2, rc, "output was:\n{output}");
}

/// Port of `tests/unit/cli/test_baseline.py::BanditBaselineToolTests::test_main_subprocess_error` (adapted, see WP).
///
/// Python mocks `subprocess.check_output` to raise `CalledProcessError(returncode=3)`;
/// here `BANDITRS_BANDIT_EXE` points at a fake `bandit` that writes to stdout and exits 3.
#[test]
fn test_main_subprocess_error() {
    let repo = init_repo();
    std::fs::write(repo.path().join("additional_file.py"), mktemp_contents())
        .expect("write additional_file.py");
    git(repo.path(), &["add", "additional_file.py"]);
    git(repo.path(), &["commit", "-m", "Additional Content"]);

    let fake_bandit_dir = tempfile::tempdir().expect("tempdir");
    let fake_bandit = write_script(fake_bandit_dir.path(), "bandit_mock", FAKE_BANDIT_EXIT_3);

    let (rc, output) = run_baseline(
        repo.path(),
        &["."],
        &[(
            "BANDITRS_BANDIT_EXE",
            fake_bandit.to_str().expect("utf8 path"),
        )],
    );

    // assert the system exits with code 3 (returned from CalledProcessError)
    assert_eq!(3, rc, "output was:\n{output}");
}
