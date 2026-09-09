//! Replays the CLI differential matrix (`tests/cli_matrix/**`) against the Rust
//! binaries: `docs/plan/wp/WP-18-cli-matrix.md`.
//!
//! Where `tests/golden.rs` proves the *report* matches, this proves the whole
//! command-line surface does: for each of the invocations in
//! `tests/cli_matrix/cases.tsv` it compares **stdout, stderr and the exit
//! code** against output recorded from the reference Python bandit by
//! `scripts/cli_matrix.py gen`. No Python is needed to run it — every expected
//! value comes from a committed file.
//!
//! A case with a `<id>.expected` marker is one the two tools are known to
//! differ on; its golden was recorded from BanditRS instead, with the
//! DEVIATIONS.md entry that justifies it named in the marker. Those still fail
//! here if BanditRS changes, they just are not a claim about Python.

#[path = "cli_matrix/normalize.rs"]
mod normalize;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pretty_assertions::assert_eq;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn base() -> PathBuf {
    manifest_dir().join("tests/cli_matrix")
}

/// Directory holding the compiled binaries, so `bandit-baseline` — which shells
/// out to plain `bandit` — finds ours rather than one from the ambient PATH.
fn bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_bandit"))
        .parent()
        .expect("binary has a parent directory")
        .to_path_buf()
}

fn binary(name: &str) -> PathBuf {
    match name {
        "bandit" => PathBuf::from(env!("CARGO_BIN_EXE_bandit")),
        "bandit-baseline" => PathBuf::from(env!("CARGO_BIN_EXE_bandit-baseline")),
        "bandit-config-generator" => PathBuf::from(env!("CARGO_BIN_EXE_bandit-config-generator")),
        other => panic!("unknown binary in cases.tsv: {other}"),
    }
}

struct Case {
    id: String,
    bin: String,
    prep: String,
    args: Vec<String>,
}

fn read_cases() -> Vec<Case> {
    let path = base().join("cases.tsv");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|line| {
            let mut f = line.split('\t');
            let id = f.next().expect("id").to_string();
            let bin = f.next().expect("bin").to_string();
            let prep = f.next().expect("prep").to_string();
            let args = f
                .next()
                .expect("args")
                .split_whitespace()
                .map(str::to_string)
                .collect();
            Case {
                id,
                bin,
                prep,
                args,
            }
        })
        .collect()
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create workspace dir");
    for entry in fs::read_dir(from).expect("read workspace") {
        let entry = entry.expect("dir entry");
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let meta = entry.metadata().expect("metadata");
        if meta.is_symlink() {
            let target = fs::read_link(&src).expect("read symlink");
            std::os::unix::fs::symlink(target, &dst).expect("recreate symlink");
        } else if meta.is_dir() {
            copy_tree(&src, &dst);
        } else {
            fs::copy(&src, &dst).expect("copy fixture");
        }
    }
}

fn git(cwd: &Path, args: &[&str]) {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@e")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@e")
        .output()
        .expect("run git");
}

fn prepare(case: &Case, ws: &Path) {
    copy_tree(&base().join("workspace"), ws);
    match case.prep.as_str() {
        "-" => {}
        "git" => {
            git(ws, &["init", "-q", "-b", "main"]);
            git(ws, &["add", "-A"]);
            git(ws, &["commit", "-qm", "base"]);
            fs::write(
                ws.join("src/extra.py"),
                "import subprocess\nsubprocess.call('ls', shell=True)\n",
            )
            .expect("write extra fixture");
            git(ws, &["add", "-A"]);
            git(ws, &["commit", "-qm", "change"]);
        }
        "selfbaseline" => {
            Command::new(binary("bandit"))
                .args(["src/assert.py", "-f", "json", "-o", "baseline.json"])
                .current_dir(ws)
                .env("BANDITRS_PYTHON_COMPAT", "3.11")
                .output()
                .expect("write baseline");
        }
        other => panic!("unknown prep in cases.tsv: {other}"),
    }
}

/// Runs one case in a fresh workspace and returns its normalised
/// `(stdout, stderr, exit code)`.
fn run(case: &Case) -> (String, String, i32) {
    // A fresh directory per invocation, not one derived from the case id: the
    // tests in this file run concurrently and two of them replay the same case,
    // so a name-derived path would have them delete each other's workspace.
    let dir = tempfile::tempdir().expect("temp workspace");
    let ws = dir.path().to_path_buf();
    prepare(case, &ws);

    let path = format!(
        "{}:{}",
        bin_dir().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(binary(&case.bin))
        .args(&case.args)
        .current_dir(&ws)
        .env("PATH", path)
        .env("BANDITRS_PYTHON_COMPAT", "3.11")
        .env("COLUMNS", "80")
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("spawn {} for {}: {e}", case.bin, case.id));

    let mut stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    // `-o FILE` puts the report in a file; fold it in so it is compared too.
    for arg in &case.args {
        if arg.starts_with("report.") && ws.join(arg).exists() {
            let body = fs::read_to_string(ws.join(arg)).unwrap_or_default();
            stdout.push_str(&format!("\n--- file {arg} ---\n{body}"));
        }
    }
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let ws_str = ws.to_string_lossy().into_owned();
    (
        normalize::normalize(&stdout, &ws_str),
        normalize::normalize(&stderr, &ws_str),
        out.status.code().unwrap_or(-1),
    )
}

fn golden(id: &str, ext: &str) -> String {
    let path = base().join("out").join(format!("{id}.{ext}"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// First line where two streams disagree, rendered for the failure message.
/// Naming the case is not enough when a golden runs to thousands of lines.
fn first_difference(expected: &str, actual: &str) -> Option<String> {
    if expected == actual {
        return None;
    }
    let (mut e, mut a) = (expected.lines(), actual.lines());
    let mut n = 1;
    loop {
        match (e.next(), a.next()) {
            (None, None) => return Some("differs only in trailing newline".into()),
            (x, y) if x == y => n += 1,
            (x, y) => {
                return Some(format!(
                    "at line {n}:\n      expected: {:?}\n      actual:   {:?}",
                    x.unwrap_or("<end of output>"),
                    y.unwrap_or("<end of output>")
                ));
            }
        }
    }
}

/// Every case in one test, accumulating failures: a single run then reports
/// *all* the invocations that drifted, which is what you want when a change to
/// a formatter or a log line moves several at once.
#[test]
fn cli_matrix_replays_the_reference_surface() {
    let cases = read_cases();
    assert!(cases.len() >= 80, "cases.tsv looks truncated");
    let mut failures = Vec::new();

    for case in &cases {
        let (stdout, stderr, code) = run(case);
        let expected_code: i32 = golden(&case.id, "code").trim().parse().expect("exit code");
        if let Some(d) = first_difference(&golden(&case.id, "out"), &stdout) {
            failures.push(format!("{}: stdout {d}", case.id));
        }
        if let Some(d) = first_difference(&golden(&case.id, "err"), &stderr) {
            failures.push(format!("{}: stderr {d}", case.id));
        }
        if code != expected_code {
            failures.push(format!("{}: exit {} != {}", case.id, code, expected_code));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} CLI cases diverged from the recorded reference surface:\n  {}\n\
         Re-run `scripts/cli_matrix.py run <id>` to see both sides.",
        failures.len(),
        cases.len(),
        failures.join("\n  ")
    );
}

/// The two most load-bearing cases get their own test so a failure prints the
/// actual diff rather than just naming the case.
#[test]
fn default_json_report_matches() {
    let case = read_cases()
        .into_iter()
        .find(|c| c.id == "sev_default")
        .expect("sev_default case");
    let (stdout, stderr, code) = run(&case);
    assert_eq!(golden("sev_default", "out"), stdout);
    assert_eq!(golden("sev_default", "err"), stderr);
    assert_eq!(1, code);
}

#[test]
fn usage_error_matches() {
    let case = read_cases()
        .into_iter()
        .find(|c| c.id == "excl_verbose_quiet")
        .expect("case");
    let (stdout, stderr, code) = run(&case);
    assert_eq!(golden("excl_verbose_quiet", "out"), stdout);
    assert_eq!(golden("excl_verbose_quiet", "err"), stderr);
    assert_eq!(2, code, "argparse usage errors exit 2");
}
