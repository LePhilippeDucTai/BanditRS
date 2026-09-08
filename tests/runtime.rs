//! Port of `tests/functional/test_runtime.py`: run the `bandit` binary and
//! check exit codes and output substrings (stdout + stderr merged).
//! `#[ignore]`d until the CLI exists (PLAN.md M7).

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn run(args: &[&str], stdin_file: Option<&str>) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bandit"));
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(f) = stdin_file {
        cmd.stdin(std::fs::File::open(examples().join(f)).unwrap());
    } else {
        cmd.stdin(Stdio::null());
    }
    let out = cmd.output().unwrap();
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

fn example(name: &str) -> String {
    examples().join(name).to_string_lossy().into_owned()
}

#[test]
fn test_no_arguments() {
    let (rc, out) = run(&[], None);
    assert_eq!(rc, 2);
    assert!(out.contains("usage: bandit [-h]"));
}

#[test]
fn test_piped_input() {
    let (rc, out) = run(&["-"], Some("imports.py"));
    assert_eq!(rc, 1);
    for s in [
        "Total lines of code: 4",
        "Low: 2",
        "High: 2",
        "Files skipped (0):",
        "Issue: [B403:blacklist] Consider possible",
        "<stdin>:2",
        "<stdin>:4",
    ] {
        assert!(out.contains(s), "missing {s:?} in {out}");
    }
}

#[test]
fn test_nonexistent_config() {
    let (rc, out) = run(&["-c", "nonexistent.yml", "xx.py"], None);
    assert_eq!(rc, 2);
    assert!(out.contains("nonexistent.yml : Could not read config file."));
}

#[test]
fn test_help_arg() {
    let (rc, out) = run(&["-h"], None);
    assert_eq!(rc, 0);
    for s in [
        "Bandit - a Python source code security analyzer",
        "usage: bandit [-h]",
        "positional arguments:",
        "tests were discovered and loaded:",
    ] {
        assert!(out.contains(s), "missing {s:?}");
    }
}

#[test]
fn test_example_nonexistent() {
    let (rc, out) = run(&[&example("nonexistent.py")], None);
    assert_eq!(rc, 0);
    assert!(out.contains("Files skipped (1):"));
    assert!(out.contains("nonexistent.py (No such file or directory"));
}

#[test]
fn test_example_okay() {
    let (rc, out) = run(&[&example("okay.py")], None);
    assert_eq!(rc, 0);
    for s in [
        "Total lines of code: 1",
        "Files skipped (0):",
        "No issues identified.",
    ] {
        assert!(out.contains(s), "missing {s:?}");
    }
}

#[test]
fn test_example_nonsense() {
    let (rc, out) = run(&[&example("nonsense.py")], None);
    assert_eq!(rc, 0);
    assert!(out.contains("Files skipped (1):"));
    assert!(out.contains("nonsense.py (syntax error while parsing AST"));
}

#[test]
fn test_example_nonsense2() {
    let (rc, out) = run(&[&example("nonsense2.py")], None);
    assert_eq!(rc, 0);
    assert!(out.contains("Files skipped (1):"));
    assert!(out.contains("nonsense2.py (syntax error while parsing AST"));
}

#[test]
fn test_example_imports() {
    let (rc, out) = run(&[&example("imports.py")], None);
    assert_eq!(rc, 1);
    for s in [
        "Total lines of code: 4",
        "Low: 2",
        "High: 2",
        "Files skipped (0):",
        "Issue: [B403:blacklist] Consider possible",
        "imports.py:2",
        "imports.py:4",
    ] {
        assert!(out.contains(s), "missing {s:?}");
    }
}
