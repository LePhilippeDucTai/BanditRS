//! End-to-end micro-benchmarks (criterion). Scaffold owned by WP-15
//! (docs/plan/wp/WP-15-benchmarks.md, docs/plan/benchmarks.md).
//!
//! Run: `cargo bench --bench e2e` (HTML report under `target/criterion/`).
//! The Python-vs-Rust wall-clock comparison lives in
//! `scripts/bench_vs_python.sh`; these benches track the Rust side only so
//! regressions are caught without a Python environment (`scripts/bench_regression.sh`).

use std::hint::black_box;
use std::path::{Path, PathBuf};

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

/// Real third-party code, from the corpus pinned in `tests/corpus/manifest.tsv`
/// (WP-17). `examples/` is 94 tiny files written to trip every plugin and the
/// stdlib is unusually plugin-sparse; a real library is neither, and is the
/// shape of input the tool actually meets. Skipped cleanly when the corpus has
/// not been fetched — it is gitignored, so this is the normal case on a fresh
/// clone (`scripts/corpus.py fetch --tier smoke` provides it).
fn corpus_package(prefix: &str) -> Option<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/corpus");
    let entries = std::fs::read_dir(root).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(prefix))
        })
        .map(|p| p.to_string_lossy().into_owned())
}

fn bench_scan_corpus(c: &mut Criterion) {
    // paramiko: small enough to keep the bench quick, dense enough in security
    // findings (13 distinct test ids) that plugin cost, not walking, dominates.
    let Some(dir) = corpus_package("paramiko-") else {
        return;
    };
    let mut group = c.benchmark_group("corpus");
    group.sample_size(20);
    group.bench_function("scan_paramiko", |b| {
        b.iter(|| {
            let config = BanditConfig::default();
            let mut mgr = manager_with(full_test_set(&config));
            black_box(scan(&mut mgr, std::slice::from_ref(&dir)))
        })
    });
    group.finish();
}

/// A single very large machine-generated module, the shape `botocore` and other
/// SDK packages are full of: one file, thousands of lines, few findings. Isolates
/// parser and walker throughput from plugin cost.
fn bench_scan_large_generated_file(c: &mut Criterion) {
    let Some(dir) = corpus_package("botocore-") else {
        return;
    };
    let biggest = walkdir_biggest_py(Path::new(&dir));
    let Some(file) = biggest else { return };
    let mut group = c.benchmark_group("corpus");
    group.sample_size(20);
    group.bench_function("scan_largest_botocore_file", |b| {
        b.iter(|| {
            let config = BanditConfig::default();
            let mut mgr = manager_with(full_test_set(&config));
            black_box(scan(&mut mgr, std::slice::from_ref(&file)))
        })
    });
    group.finish();
}

fn walkdir_biggest_py(root: &Path) -> Option<String> {
    let mut stack = vec![root.to_path_buf()];
    let mut best: Option<(u64, PathBuf)> = None;
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "py")
                && best.as_ref().is_none_or(|(size, _)| meta.len() > *size)
            {
                best = Some((meta.len(), path));
            }
        }
    }
    best.map(|(_, p)| p.to_string_lossy().into_owned())
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
    bench_scan_corpus,
    bench_scan_large_generated_file,
);
criterion_main!(benches);
