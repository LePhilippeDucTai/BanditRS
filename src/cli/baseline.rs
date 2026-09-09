//! `bandit-baseline` (port of `bandit/cli/baseline.py`) — see
//! docs/spec/cli_formatters_tests.md §A.13.
//!
//! Git is driven through the `git` CLI, every invocation carrying `-C <cwd>` (`rev-parse
//! --is-inside-work-tree`, `status --porcelain` for `is_dirty`, `rev-parse HEAD` / `HEAD^`,
//! `name-rev --name-only`, `reset --hard <commit>`); the inner scans invoke the `bandit`
//! executable found via `BANDITRS_BANDIT_EXE` (test-only override, documented on
//! [`bandit_path`], no effect for end users who never set it), else next to the current
//! executable, else `PATH`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `baseline_tmp_file` (Python module constant).
pub const BASELINE_TMP_FILE: &str = "_bandit_baseline_run.json_";
/// `report_basename` (Python module constant).
pub const REPORT_BASENAME: &str = "bandit_baseline_result";
const DEFAULT_OUTPUT_FORMAT: &str = "terminal";
const VALID_FORMATS: [&str; 3] = ["txt", "html", "json"];

fn log_info(msg: impl std::fmt::Display) {
    crate::log_info!("baseline", "{}", msg);
}

/// `argparse`'s usage line for this parser (`prog` = `bandit-baseline`, one optional
/// `-f` with `choices`, one `nargs="+"` positional).
const USAGE: &str = "usage: bandit-baseline [-h] [-f {txt,html,json}] targets [targets ...]";

/// Full `--help` text of `initialize()`'s `ArgumentParser`
/// (`RawDescriptionHelpFormatter`, so `description` and `epilog` are emitted verbatim).
const HELP: &str = "\nBandit Baseline - Generates Bandit results compared to a baseline\n\npositional arguments:\n  targets             source file(s) or directory(s) to be tested\n\noptions:\n  -h, --help          show this help message and exit\n  -f {txt,html,json}  specify output format\n\nAdditional Bandit arguments such as severity filtering (-ll) can be added and will be passed to Bandit.\n";

/// `argparse.ArgumentParser.error()`: usage line and `prog: error: <message>` on stderr,
/// then exit 2. Used for the two errors the parser itself raises (a missing `targets`, an
/// out-of-`choices` `-f`) — everything after argument parsing is reported through `LOG`
/// instead, exactly as in Python.
fn argparse_error(msg: impl std::fmt::Display) {
    eprintln!("{USAGE}");
    eprintln!("bandit-baseline: error: {msg}");
}

fn log_error(msg: impl std::fmt::Display) {
    crate::log_error!("baseline", "{}", msg);
}

/// Path to the `bandit` binary: `BANDITRS_BANDIT_EXE` (test-only override — lets
/// `tests/unit_cli_baseline.rs` point at the binary built for the current `cargo test` run
/// without depending on `PATH`; unset in normal use, so it has no effect for end users) if
/// set, else next to the current executable, else `PATH`.
fn bandit_path() -> PathBuf {
    if let Ok(over) = std::env::var("BANDITRS_BANDIT_EXE") {
        return PathBuf::from(over);
    }
    if let Ok(cur) = std::env::current_exe()
        && let Some(dir) = cur.parent()
    {
        let candidate = dir.join("bandit");
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("bandit")
}

/// `git -C <cwd> <args>`.
fn git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn is_git_command_not_found() -> bool {
    Command::new("git").arg("--version").output().is_err()
}

fn is_dirty(cwd: &Path) -> bool {
    git(cwd, &["status", "--porcelain", "--untracked-files=no"])
        .map(|s| !s.is_empty())
        .unwrap_or(true)
}

fn name_rev(cwd: &Path, sha: &str) -> String {
    // GitPython's `commit.name_rev` is `git name-rev <sha>`, which prints
    // "<sha> <name>"; `--name-only` would drop the sha (DEVIATIONS #15).
    git(cwd, &["name-rev", sha]).unwrap_or_else(|_| sha.to_string())
}

fn reset_hard(cwd: &Path, commit: &str) -> Result<(), String> {
    git(cwd, &["reset", "--hard", commit]).map(|_| ())
}

/// Result of a successful [`initialize`] — Python's `(output_format, repo, report_fname)`
/// when valid; `repo` itself is implicit here since every subsequent Git call is given
/// `cwd` explicitly instead of holding on to a repository handle.
pub struct Initialized {
    pub output_format: String,
    pub report_fname: String,
}

/// Port of `bandit.cli.baseline.initialize()`. Returns `None` where Python returns
/// `(None, None, None)`. `cwd` is explicit (Python reads `os.getcwd()`) so tests can call
/// this directly, in parallel, without touching the process' working directory; [`main`]
/// passes `std::env::current_dir()`.
pub fn initialize(cwd: &Path, bandit_args: &[String]) -> Option<Initialized> {
    let mut valid = true;

    let output_format = match extract_output_format(bandit_args) {
        Some(v) => {
            if !VALID_FORMATS.contains(&v.as_str()) {
                argparse_error(format!(
                    "argument -f: invalid choice: '{v}' (choose from 'txt', 'html', 'json')"
                ));
                return None;
            }
            v
        }
        None => DEFAULT_OUTPUT_FORMAT.to_string(),
    };
    if targets_of(bandit_args).is_empty() {
        argparse_error("the following arguments are required: targets");
        return None;
    }

    if output_format == DEFAULT_OUTPUT_FORMAT {
        log_info(format!(
            "No output format specified, using {DEFAULT_OUTPUT_FORMAT}"
        ));
    }
    let report_fname = format!("{REPORT_BASENAME}.{output_format}");

    if is_git_command_not_found() {
        log_error("Git not available, reinstall with baseline extra");
        return None;
    }

    match git(cwd, &["rev-parse", "--is-inside-work-tree"]) {
        Ok(_) => {
            if is_dirty(cwd) {
                log_error("Current working directory is dirty and must be resolved");
                valid = false;
            }
        }
        Err(_) => {
            log_error("Bandit baseline must be called from a git project root");
            valid = false;
        }
    }

    if output_format != DEFAULT_OUTPUT_FORMAT && cwd.join(&report_fname).exists() {
        log_error(format!("File {report_fname} already exists, aborting"));
        valid = false;
    }
    if cwd.join(BASELINE_TMP_FILE).exists() {
        log_error(format!(
            "Temporary file {BASELINE_TMP_FILE} needs to be removed prior to running"
        ));
        valid = false;
    }
    if bandit_args.iter().any(|a| a == "-o") {
        log_error("Bandit baseline must not be called with the -o option");
        valid = false;
    }

    if valid {
        Some(Initialized {
            output_format,
            report_fname,
        })
    } else {
        None
    }
}

/// Extract `-f <fmt>` from the raw argv (the only flag `initialize()` itself parses;
/// everything else is passed through to `bandit` verbatim).
fn extract_output_format(argv: &[String]) -> Option<String> {
    let mut it = argv.iter();
    while let Some(a) = it.next() {
        if a == "-f" {
            return it.next().cloned();
        }
    }
    None
}

fn targets_of(argv: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut it = argv.iter().peekable();
    while let Some(a) = it.next() {
        if a == "-f" {
            it.next();
            continue;
        }
        if a.starts_with('-') {
            continue;
        }
        out.push(a.clone());
    }
    out
}

/// `subprocess.check_output(["bandit"] + args)`: stdout is captured; stderr is inherited
/// (Python doesn't pass `stderr=STDOUT`, so log output streams straight to the terminal
/// instead of being captured here).
fn run_bandit(args: &[String]) -> (i32, String) {
    match Command::new(bandit_path())
        .args(args)
        .stderr(std::process::Stdio::inherit())
        .output()
    {
        Ok(o) => (
            o.status.code().unwrap_or(1),
            String::from_utf8_lossy(&o.stdout).into_owned(),
        ),
        Err(e) => (1, e.to_string()),
    }
}

/// Init the `LOG` used by this module: level INFO, format `"[%(levelname)7s ]
/// %(message)s"`, handler on stdout.
pub fn init_logger() {
    crate::log::set_level(crate::log::Level::Info);
    crate::log::set_format("[%(levelname)7s ] %(message)s");
    crate::log::set_stdout(true);
}

/// Entry point; returns the exit code.
pub fn main(bandit_args: Vec<String>) -> i32 {
    init_logger();

    // `argparse` handles `-h`/`--help` while parsing, i.e. before the `targets` requirement
    // is enforced, so `bandit-baseline --help` prints the help and exits 0 even though no
    // target was given.
    if bandit_args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        print!("{HELP}");
        return 0;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let Some(init) = initialize(&cwd, &bandit_args) else {
        return 2;
    };

    let current_commit = match git(&cwd, &["rev-parse", "HEAD"]) {
        Ok(sha) => sha,
        Err(_) => {
            log_error("Unable to get current or parent commit");
            return 2;
        }
    };
    log_info(format!(
        "Got current commit: [{}]",
        name_rev(&cwd, &current_commit)
    ));

    let parent_commit = match git(&cwd, &["rev-parse", "HEAD^"]) {
        Ok(sha) => sha,
        Err(_) => {
            log_error("Parent commit not available");
            return 2;
        }
    };
    log_info(format!(
        "Got parent commit: [{}]",
        name_rev(&cwd, &parent_commit)
    ));

    let output_type: Vec<String> = if init.output_format == DEFAULT_OUTPUT_FORMAT {
        vec!["-f".to_string(), "txt".to_string()]
    } else {
        vec!["-o".to_string(), init.report_fname.clone()]
    };

    let tmpdir = match tempfile_dir() {
        Ok(d) => d,
        Err(e) => {
            log_error(e);
            return 2;
        }
    };
    let bandit_tmpfile = tmpdir.join(BASELINE_TMP_FILE);
    let bandit_tmpfile_str = bandit_tmpfile.to_string_lossy().into_owned();

    let steps: [(&str, &str, Vec<String>); 2] = [
        ("Getting Bandit baseline results", &parent_commit, {
            let mut a = bandit_args.clone();
            a.extend([
                "-f".to_string(),
                "json".to_string(),
                "-o".to_string(),
                bandit_tmpfile_str.clone(),
            ]);
            a
        }),
        ("Comparing Bandit results to baseline", &current_commit, {
            let mut a = bandit_args.clone();
            a.extend(["-b".to_string(), bandit_tmpfile_str.clone()]);
            a.extend(output_type);
            a
        }),
    ];

    let mut return_code = 0;
    let mut last_output = String::new();
    for (message, commit, args) in &steps {
        if let Err(e) = reset_hard(&cwd, commit) {
            log_error(e);
            let _ = reset_hard(&cwd, &current_commit);
            let _ = std::fs::remove_dir_all(&tmpdir);
            return 2;
        }
        log_info(*message);
        let (code, output) = run_bandit(args);
        return_code = code;
        last_output = output;
        if !(0..=1).contains(&return_code) {
            log_error(format!(
                "Error running command: {bandit_args:?}\nOutput: {last_output}\n"
            ));
        }
    }

    let _ = reset_hard(&cwd, &current_commit);
    let _ = std::fs::remove_dir_all(&tmpdir);

    if init.output_format == DEFAULT_OUTPUT_FORMAT {
        println!("{last_output}");
    } else {
        log_info(format!("Successfully wrote {}", init.report_fname));
    }

    return_code
}

fn tempfile_dir() -> Result<PathBuf, String> {
    let base = std::env::temp_dir();
    let name = format!("bandit-baseline-{}", std::process::id());
    let dir = base.join(name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
