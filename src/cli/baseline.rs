//! `bandit-baseline` (port of `bandit/cli/baseline.py`) — see
//! docs/spec/cli_formatters_tests.md §A.13.
//!
//! Git is driven through the `git` CLI (`rev-parse --show-toplevel`,
//! `status --porcelain` for `is_dirty`, `rev-parse HEAD` / `HEAD^`,
//! `name-rev --name-only`, `reset --hard <commit>`); the inner scans invoke
//! the `bandit` executable next to the current one (fallback: PATH).

use std::path::PathBuf;
use std::process::Command;

const BASELINE_TMP_FILE: &str = "_bandit_baseline_run.json_";
const DEFAULT_OUTPUT_FORMAT: &str = "terminal";
const REPORT_BASENAME: &str = "bandit_baseline_result";
const VALID_FORMATS: [&str; 3] = ["txt", "html", "json"];

fn log_info(msg: impl std::fmt::Display) {
    crate::log_info!("baseline", "{}", msg);
}
fn log_error(msg: impl std::fmt::Display) {
    crate::log_error!("baseline", "{}", msg);
}

/// Path to the `bandit` binary: next to the current executable, else `PATH`.
fn bandit_path() -> PathBuf {
    if let Ok(cur) = std::env::current_exe() {
        if let Some(dir) = cur.parent() {
            let candidate = dir.join("bandit");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from("bandit")
}

fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git").args(args).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn is_git_command_not_found() -> bool {
    Command::new("git").arg("--version").output().is_err()
}

fn is_dirty() -> bool {
    git(&["status", "--porcelain", "--untracked-files=no"]).map(|s| !s.is_empty()).unwrap_or(true)
}

fn name_rev(sha: &str) -> String {
    git(&["name-rev", "--name-only", sha]).unwrap_or_else(|_| sha.to_string())
}

fn reset_hard(commit: &str) -> Result<(), String> {
    git(&["reset", "--hard", commit]).map(|_| ())
}

struct Initialized {
    output_format: String,
    report_fname: String,
}

/// `initialize()`.
fn initialize(targets: &[String], bandit_args: &[String]) -> Option<Initialized> {
    let mut valid = true;

    let output_format = match targets_output_format(bandit_args) {
        Some(v) => {
            if !VALID_FORMATS.contains(&v.as_str()) {
                log_error(format!("argument -f: invalid choice: '{v}' (choose from 'txt', 'html', 'json')"));
                return None;
            }
            v
        }
        None => DEFAULT_OUTPUT_FORMAT.to_string(),
    };
    if targets.is_empty() {
        log_error("the following arguments are required: targets");
        return None;
    }

    if output_format == DEFAULT_OUTPUT_FORMAT {
        log_info(format!("No output format specified, using {DEFAULT_OUTPUT_FORMAT}"));
    }
    let report_fname = format!("{REPORT_BASENAME}.{output_format}");

    if is_git_command_not_found() {
        log_error("Git not available, reinstall with baseline extra");
        return None;
    }

    match git(&["rev-parse", "--is-inside-work-tree"]) {
        Ok(_) => {
            if is_dirty() {
                log_error("Current working directory is dirty and must be resolved");
                valid = false;
            }
        }
        Err(_) => {
            log_error("Bandit baseline must be called from a git project root");
            valid = false;
        }
    }

    if output_format != DEFAULT_OUTPUT_FORMAT && std::path::Path::new(&report_fname).exists() {
        log_error(format!("File {report_fname} already exists, aborting"));
        valid = false;
    }
    if std::path::Path::new(BASELINE_TMP_FILE).exists() {
        log_error(format!("Temporary file {BASELINE_TMP_FILE} needs to be removed prior to running"));
        valid = false;
    }
    if bandit_args.iter().any(|a| a == "-o") {
        log_error("Bandit baseline must not be called with the -o option");
        valid = false;
    }

    if valid { Some(Initialized { output_format, report_fname }) } else { None }
}

/// Extract `-f <fmt>` from the raw argv (the only flag `initialize()` itself
/// parses; everything else is passed through to `bandit` verbatim).
fn targets_output_format(argv: &[String]) -> Option<String> {
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

/// `subprocess.check_output(["bandit"] + args)`: stdout is captured; stderr
/// is inherited (Python doesn't pass `stderr=STDOUT`, so log output streams
/// straight to the terminal instead of being captured here).
fn run_bandit(args: &[String]) -> (i32, String) {
    match Command::new(bandit_path()).args(args).stderr(std::process::Stdio::inherit()).output() {
        Ok(o) => (o.status.code().unwrap_or(1), String::from_utf8_lossy(&o.stdout).into_owned()),
        Err(e) => (1, e.to_string()),
    }
}

/// Entry point; returns the exit code.
pub fn main(bandit_args: Vec<String>) -> i32 {
    crate::log::set_level(crate::log::Level::Info);
    crate::log::set_format("[%(levelname)7s ] %(message)s");
    crate::log::set_stdout(true);

    let targets = targets_of(&bandit_args);
    let Some(init) = initialize(&targets, &bandit_args) else {
        return 2;
    };

    let current_commit = match git(&["rev-parse", "HEAD"]) {
        Ok(sha) => sha,
        Err(_) => {
            log_error("Unable to get current or parent commit");
            return 2;
        }
    };
    log_info(format!("Got current commit: [{}]", name_rev(&current_commit)));

    let parent_commit = match git(&["rev-parse", "HEAD^"]) {
        Ok(sha) => sha,
        Err(_) => {
            log_error("Parent commit not available");
            return 2;
        }
    };
    log_info(format!("Got parent commit: [{}]", name_rev(&parent_commit)));

    let output_type: Vec<String> =
        if init.output_format == DEFAULT_OUTPUT_FORMAT { vec!["-f".to_string(), "txt".to_string()] } else { vec!["-o".to_string(), init.report_fname.clone()] };

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
            a.extend(["-f".to_string(), "json".to_string(), "-o".to_string(), bandit_tmpfile_str.clone()]);
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
        if let Err(e) = reset_hard(commit) {
            log_error(e);
            let _ = reset_hard(&current_commit);
            let _ = std::fs::remove_dir_all(&tmpdir);
            return 2;
        }
        log_info(*message);
        let (code, output) = run_bandit(args);
        return_code = code;
        last_output = output;
        if !(0..=1).contains(&return_code) {
            log_error(format!("Error running command: {bandit_args:?}\nOutput: {last_output}\n"));
        }
    }

    let _ = reset_hard(&current_commit);
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
