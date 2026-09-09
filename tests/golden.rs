//! Replays the golden corpus (`tests/golden/**`, committed by
//! `scripts/gen_golden.sh` from the reference Python bandit) against the
//! Rust binary: `docs/plan/wp/WP-14-golden-differential.md`. Proves parity
//! without Python installed — every expected value here comes from a
//! committed file, never from re-running the reference.
//!
//! `Command::current_dir` is used throughout (never `std::env::set_current_dir`,
//! see `tests/unit_cli_main.rs`), so this file is safe to run in parallel with
//! the rest of the suite.

#[path = "golden/normalize.rs"]
mod normalize;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use pretty_assertions::assert_eq;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn golden_dir() -> PathBuf {
    manifest_dir().join("tests/golden")
}

/// Runs the compiled `bandit` binary from the repository root
/// (`current_dir = CARGO_MANIFEST_DIR`) with `BANDITRS_PYTHON_COMPAT=3.11`
/// (f-string constant positions, PLAN.md §6) and returns its normalised
/// stdout.
fn run_bandit(args: &[&str]) -> String {
    let root = manifest_dir();
    let out = Command::new(env!("CARGO_BIN_EXE_bandit"))
        .args(args)
        .current_dir(&root)
        .env("BANDITRS_PYTHON_COMPAT", "3.11")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("spawn bandit");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    normalize::normalize(&stdout, &root.to_string_lossy())
}

fn read_golden(name: &str) -> String {
    let path = golden_dir().join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e))
}

macro_rules! format_test {
    ($name:ident, $fmt:literal) => {
        #[test]
        fn $name() {
            let golden = read_golden(concat!("examples.", $fmt));
            let actual = run_bandit(&["-r", "examples", "-f", $fmt]);
            assert_eq!(golden, actual, "golden/examples.{} diverges", $fmt);
        }
    };
}

format_test!(test_examples_json, "json");
format_test!(test_examples_yaml, "yaml");
format_test!(test_examples_csv, "csv");
format_test!(test_examples_xml, "xml");
format_test!(test_examples_txt, "txt");
format_test!(test_examples_html, "html");
format_test!(test_examples_sarif, "sarif");
format_test!(test_examples_custom, "custom");

/// One test for the whole `files/` corpus (`bandit examples/<fixture> -f
/// json` per golden file): failures accumulate instead of stopping at the
/// first divergent fixture, so a single run reports every fixture that needs
/// attention.
#[test]
fn test_files_json() {
    let dir = golden_dir().join("files");
    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {}", dir.display(), e))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();
    assert!(
        !entries.is_empty(),
        "no golden fixtures under {}",
        dir.display()
    );

    let mut failures = Vec::new();
    for golden_path in &entries {
        // "<fixture>.py.json" -> "<fixture>.py"
        let fixture = Path::new(golden_path.file_stem().unwrap());
        let golden = fs::read_to_string(golden_path)
            .unwrap_or_else(|e| panic!("read {}: {}", golden_path.display(), e));
        let target = format!("examples/{}", fixture.display());
        let actual = run_bandit(&[&target, "-f", "json"]);
        if actual != golden {
            failures.push(fixture.display().to_string());
        }
    }

    assert!(
        failures.is_empty(),
        "{} fixture(s) diverge from tests/golden/files: {}",
        failures.len(),
        failures.join(", ")
    );
}
