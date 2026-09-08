//! End-to-end micro-benchmarks (criterion). Scaffold owned by WP-15
//! (docs/plan/wp/WP-15-benchmarks.md, docs/plan/benchmarks.md).
//!
//! Run: `cargo bench --bench e2e` (HTML report under `target/criterion/`).
//! The Python-vs-Rust wall-clock comparison lives in
//! `scripts/bench_vs_python.sh`; these benches track the Rust side only so
//! regressions are caught without a Python environment.

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};

use banditrs::ast::PyCompat;
use banditrs::core::config::BanditConfig;
use banditrs::core::manager::{AggType, Manager};
use banditrs::core::test_set::TestSet;
use banditrs::source::parse::parse_module;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn manager() -> Manager {
    let config = BanditConfig::default();
    let profile = config.default_profile();
    let test_set = TestSet::new(&config, &profile);
    Manager::new(config, AggType::File, test_set)
}

fn scan(targets: &[String]) -> usize {
    let mut mgr = manager();
    mgr.discover_files(targets, true, None);
    mgr.run_tests();
    mgr.results.len()
}

/// Whole `examples/` directory (96 files, parallel via rayon): the
/// end-to-end number closest to `bandit -r examples`.
fn bench_scan_examples(c: &mut Criterion) {
    let dir = examples_dir().to_string_lossy().into_owned();
    c.bench_function("scan_examples_dir", |b| {
        b.iter(|| black_box(scan(std::slice::from_ref(&dir))))
    });
}

/// One mid-size fixture end-to-end (discover + parse + walk + plugins), i.e.
/// the per-file latency without parallelism.
fn bench_scan_single_file(c: &mut Criterion) {
    let file = examples_dir()
        .join("subprocess_shell.py")
        .to_string_lossy()
        .into_owned();
    c.bench_function("scan_subprocess_shell_py", |b| {
        b.iter(|| black_box(scan(std::slice::from_ref(&file))))
    });
}

/// Parser only (ruff), on the largest fixture (`long_set.py`, 65 KiB).
fn bench_parse_long_set(c: &mut Criterion) {
    let text = std::fs::read_to_string(examples_dir().join("long_set.py")).unwrap();
    let compat = PyCompat::from_env();
    c.bench_function("parse_long_set_py", |b| {
        b.iter(|| black_box(parse_module(&text, compat).is_ok()))
    });
}

criterion_group!(
    benches,
    bench_scan_examples,
    bench_scan_single_file,
    bench_parse_long_set
);
criterion_main!(benches);
