//! End-to-end micro-benchmarks (criterion). Scaffold owned by WP-15
//! (docs/plan/wp/WP-15-benchmarks.md, docs/plan/benchmarks.md).
//!
//! Run: `cargo bench --bench e2e` (HTML report under `target/criterion/`).
//! The Python-vs-Rust wall-clock comparison lives in
//! `scripts/bench_vs_python.sh`; these benches track the Rust side only so
//! regressions are caught without a Python environment (`scripts/bench_regression.sh`).

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};

use banditrs::ast::PyCompat;
use banditrs::ast::vnode::NodeKind;
use banditrs::core::config::BanditConfig;
use banditrs::core::manager::{AggType, Manager};
use banditrs::core::plugin_config::PluginConfigs;
use banditrs::core::test_set::TestSet;
use banditrs::source::parse::parse_module;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

/// Full test set (every plugin), the config's default profile.
fn full_test_set(config: &BanditConfig) -> TestSet {
    let profile = config.default_profile();
    TestSet::new(config, &profile)
}

/// No plugins registered for any node kind and `B001` off: same
/// discover/parse/walk pipeline as `scan()`, minus the plugin cost, to
/// isolate the walker's own overhead (context building, `linerange`,
/// `qualname`...).
fn empty_test_set(config: &BanditConfig) -> TestSet {
    TestSet {
        tests: (0..NodeKind::COUNT).map(|_| Vec::new()).collect(),
        blacklist: None,
        configs: PluginConfigs::from_config(config),
    }
}

fn manager_with(test_set: TestSet) -> Manager {
    Manager::new(BanditConfig::default(), AggType::File, test_set)
}

fn scan(mgr: &mut Manager, targets: &[String]) -> usize {
    mgr.discover_files(targets, true, None);
    mgr.run_tests();
    mgr.results.len()
}

/// A `Manager` pre-filled by scanning `examples/` once (outside any timed
/// loop), for the formatter benches below.
fn scanned_examples_manager() -> Manager {
    let config = BanditConfig::default();
    let mut mgr = manager_with(full_test_set(&config));
    let dir = examples_dir().to_string_lossy().into_owned();
    scan(&mut mgr, std::slice::from_ref(&dir));
    mgr
}

/// Whole `examples/` directory (96 files, parallel via rayon): the
/// end-to-end number closest to `bandit -r examples`.
fn bench_scan_examples(c: &mut Criterion) {
    let dir = examples_dir().to_string_lossy().into_owned();
    c.bench_function("scan_examples_dir", |b| {
        b.iter(|| {
            let config = BanditConfig::default();
            let mut mgr = manager_with(full_test_set(&config));
            black_box(scan(&mut mgr, std::slice::from_ref(&dir)))
        })
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
        b.iter(|| {
            let config = BanditConfig::default();
            let mut mgr = manager_with(full_test_set(&config));
            black_box(scan(&mut mgr, std::slice::from_ref(&file)))
        })
    });
}

/// Discover + parse + walk on `examples/`, with an empty test set (no
/// plugins run): isolates the walker/context cost from the plugin cost.
fn bench_walk_only(c: &mut Criterion) {
    let dir = examples_dir().to_string_lossy().into_owned();
    c.bench_function("walk_only_examples_dir", |b| {
        b.iter(|| {
            let config = BanditConfig::default();
            let mut mgr = manager_with(empty_test_set(&config));
            black_box(scan(&mut mgr, std::slice::from_ref(&dir)))
        })
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

/// JSON formatter on the issues collected from `examples/` (`Manager`
/// pre-filled once, outside the timed loop).
fn bench_format_json_examples(c: &mut Criterion) {
    use banditrs::constants::Rank;
    let mgr = scanned_examples_manager();
    c.bench_function("format_json_examples", |b| {
        b.iter(|| {
            let mut out = Vec::new();
            banditrs::formatters::json::report(&mgr, &mut out, Rank::Undefined, Rank::Undefined, 3)
                .unwrap();
            black_box(out.len())
        })
    });
}

/// SARIF formatter on the same pre-filled `Manager`.
fn bench_format_sarif_examples(c: &mut Criterion) {
    use banditrs::constants::Rank;
    let mgr = scanned_examples_manager();
    c.bench_function("format_sarif_examples", |b| {
        b.iter(|| {
            let mut out = Vec::new();
            banditrs::formatters::sarif::report(
                &mgr,
                &mut out,
                Rank::Undefined,
                Rank::Undefined,
                3,
            )
            .unwrap();
            black_box(out.len())
        })
    });
}

/// CPython 3.11 standard library (672 files, ~307k lines): the corpus-scale
/// number from `benchmarks.md` §2. Skipped cleanly when the container does
/// not ship the stdlib sources (e.g. a slim Python image). `sample_size` is
/// reduced because each iteration is multiple seconds.
fn bench_scan_stdlib(c: &mut Criterion) {
    let path = PathBuf::from("/usr/lib/python3.11");
    if !path.exists() {
        return;
    }
    let dir = path.to_string_lossy().into_owned();
    let mut group = c.benchmark_group("stdlib");
    group.sample_size(10);
    group.bench_function("scan_stdlib_3_11", |b| {
        b.iter(|| {
            let config = BanditConfig::default();
            let mut mgr = manager_with(full_test_set(&config));
            black_box(scan(&mut mgr, std::slice::from_ref(&dir)))
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_scan_examples,
    bench_scan_single_file,
    bench_walk_only,
    bench_parse_long_set,
    bench_format_json_examples,
    bench_format_sarif_examples,
    bench_scan_stdlib,
);
criterion_main!(benches);
