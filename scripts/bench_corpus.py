#!/usr/bin/env python3
"""Benchmarks over real third-party code (WP-20, extends WP-15).

`scripts/bench_vs_python.sh` measures `examples/` and the CPython stdlib. Both
are unrepresentative in opposite directions: `examples/` is 94 tiny files
written to trip every plugin, and the stdlib is unusually plugin-sparse. This
measures the corpus of real libraries instead, and adds the four things the
existing protocol names but never produced by script:

  * peak RSS per tool, measured in the harness (`/usr/bin/time -v` is absent on
    the reference machine, so `getrusage(RUSAGE_CHILDREN)` is used with one
    dedicated child per measurement — a shared child would report the running
    maximum of *all* children);
  * the `RAYON_NUM_THREADS` scaling series and its Amdahl fit;
  * cold start as a distribution (p50/p90/max), not a median — the editor and
    pre-commit case, where start-up dominates;
  * the cost of all nine formatters on one large report, where only `json` and
    `sarif` on `examples/` were measured before.

    scripts/bench_corpus.py [-n RUNS] [--tier T] [--out FILE] [--skip ...]

Writes a markdown table to `target/bench/corpus.md` by default.
"""

import argparse
import os
import resource
import statistics
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import corpus  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
PY_BANDIT = Path(os.environ.get("PY_BANDIT", "/home/user/.pyenv-bandit/bin/bandit"))
RS_BANDIT = Path(os.environ.get("RS_BANDIT", ROOT / "target" / "release" / "bandit"))


def run_once(cmd, env=None):
    """Wall-clock seconds and peak RSS (MiB) of one child.

    RUSAGE_CHILDREN accumulates the *maximum over all reaped children*, so the
    fork+exec is isolated in a dedicated subprocess whose own children count is
    read before and after.
    """
    before = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    started = time.monotonic()
    subprocess.run(cmd, capture_output=True, env={**os.environ, **(env or {})})
    elapsed = time.monotonic() - started
    after = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    # ru_maxrss is KiB on Linux, and only rises, so a value equal to `before`
    # means this child stayed under an earlier peak: report it as unknown
    # rather than as a wrong number.
    rss = after / 1024 if after > before else None
    return elapsed, rss


def measure(cmd, runs, env=None):
    """Median wall clock over `runs` timed runs after one warm-up."""
    run_once(cmd, env)
    times, peaks = [], []
    for _ in range(runs):
        secs, rss = run_once(cmd, env)
        times.append(secs)
        if rss is not None:
            peaks.append(rss)
    return statistics.median(times), (max(peaks) if peaks else None), times


def scan_cmd(tool, target, fmt="json"):
    return [str(tool), "-r", str(target), "-q", "-f", fmt]


def bench_packages(rows, runs, out):
    out.append("## 1. Per package, real third-party code\n")
    out.append("| package | files | lines | python (s) | rust (s) | speed-up | python RSS | rust RSS |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|")
    totals = {"files": 0, "lines": 0, "py": 0.0, "rs": 0.0}
    for row in rows:
        target = corpus.target_dir(row)
        if not target.exists():
            continue
        py, py_rss, _ = measure(scan_cmd(PY_BANDIT, target), runs)
        rs, rs_rss, _ = measure(scan_cmd(RS_BANDIT, target), runs, {"BANDITRS_PYTHON_COMPAT": "3.11"})
        files, lines = int(row["py_files"] or 0), int(row["py_lines"] or 0)
        totals["files"] += files
        totals["lines"] += lines
        totals["py"] += py
        totals["rs"] += rs
        out.append(
            f"| `{row['name']} {row['version']}` | {files} | {lines} | {py:.3f} | {rs:.3f} | "
            f"**{py / rs:.1f}×** | {f'{py_rss:.0f} MiB' if py_rss else '—'} | "
            f"{f'{rs_rss:.0f} MiB' if rs_rss else '—'} |"
        )
        print(f"  {row['name']:<20} {py:.3f} -> {rs:.3f}  {py / rs:.1f}x", flush=True)
    out.append(
        f"| **total** | **{totals['files']}** | **{totals['lines']}** | "
        f"**{totals['py']:.2f}** | **{totals['rs']:.2f}** | "
        f"**{totals['py'] / totals['rs']:.1f}×** | | |"
    )
    out.append("")
    return totals


def bench_threads(target, runs, out):
    out.append("## 2. Thread scaling (`RAYON_NUM_THREADS`)\n")
    out.append("| threads | rust (s) | speed-up vs 1 thread | parallel efficiency |")
    out.append("|---:|---:|---:|---:|")
    base = None
    series = []
    for n in (1, 2, 4):
        secs, _, _ = measure(
            scan_cmd(RS_BANDIT, target), runs,
            {"RAYON_NUM_THREADS": str(n), "BANDITRS_PYTHON_COMPAT": "3.11"},
        )
        base = base or secs
        series.append((n, secs))
        out.append(f"| {n} | {secs:.3f} | {base / secs:.2f}× | {base / secs / n * 100:.0f} % |")
        print(f"  threads={n}: {secs:.3f}s", flush=True)
    # Amdahl: with speed-up S on N cores, the parallel fraction is
    # f = (1 - 1/S) / (1 - 1/N).
    n, secs = series[-1]
    speedup = base / secs
    f = (1 - 1 / speedup) / (1 - 1 / n)
    out.append(
        f"\nAmdahl fit on the {n}-thread point: parallel fraction **f ≈ {f:.2f}**, "
        f"so the ceiling on infinitely many cores is ≈ {1 / (1 - f):.1f}× the single-thread time.\n"
    )


def bench_cold_start(runs, out):
    out.append("## 3. Cold start (one-line file), as a distribution\n")
    tiny = ROOT / "target" / "bench" / "tiny.py"
    tiny.parent.mkdir(parents=True, exist_ok=True)
    tiny.write_text("assert True\n")
    out.append("| tool | p50 (ms) | p90 (ms) | max (ms) |")
    out.append("|---|---:|---:|---:|")
    for label, tool, env in (
        ("python", PY_BANDIT, {}),
        ("rust", RS_BANDIT, {"BANDITRS_PYTHON_COMPAT": "3.11"}),
    ):
        _, _, times = measure([str(tool), str(tiny), "-q", "-f", "json"], max(runs, 15), env)
        ms = sorted(t * 1000 for t in times)
        p50 = statistics.median(ms)
        p90 = ms[int(len(ms) * 0.9) - 1]
        out.append(f"| {label} | {p50:.1f} | {p90:.1f} | {max(ms):.1f} |")
        print(f"  cold start {label}: p50={p50:.1f}ms p90={p90:.1f}ms", flush=True)
    out.append("")


def bench_formats(target, runs, out):
    out.append("## 4. Cost of each formatter on one large report\n")
    out.append(f"Target: `{target.name}`. Same scan, nine output formats.\n")
    out.append("| format | python (s) | rust (s) | speed-up |")
    out.append("|---|---:|---:|---:|")
    for fmt in ("csv", "custom", "html", "json", "sarif", "screen", "txt", "xml", "yaml"):
        py, _, _ = measure(scan_cmd(PY_BANDIT, target, fmt), runs)
        rs, _, _ = measure(scan_cmd(RS_BANDIT, target, fmt), runs, {"BANDITRS_PYTHON_COMPAT": "3.11"})
        out.append(f"| `{fmt}` | {py:.3f} | {rs:.3f} | {py / rs:.1f}× |")
        print(f"  format {fmt}: {py:.3f} -> {rs:.3f}", flush=True)
    out.append("")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("-n", "--runs", type=int, default=5)
    ap.add_argument("--tier", choices=["smoke", "standard", "full", "all"], default="smoke")
    ap.add_argument("--out", default=str(ROOT / "target" / "bench" / "corpus.md"))
    ap.add_argument("--skip", nargs="*", default=[], choices=["packages", "threads", "cold", "formats"])
    args = ap.parse_args()

    rows = [r for r in corpus.select(corpus.read_manifest(), args.tier) if r["tier"] != "frontier"]
    present = [r for r in rows if corpus.target_dir(r).exists()]
    if not present:
        print("bench_corpus.py: no corpus on disk — run `scripts/corpus.py fetch`", file=sys.stderr)
        return 2

    # The biggest package present makes the most stable target for the
    # single-target benches (thread scaling, formatter cost).
    biggest = max(present, key=lambda r: int(r["py_lines"] or 0))
    target = corpus.target_dir(biggest)

    out = [
        f"# BanditRS vs bandit (Python) on real code — {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}",
        "",
        f"- tier `{args.tier}`, {len(present)} package(s); median of {args.runs} timed runs after one warm-up",
        f"- host: {os.cpu_count()} CPU(s), {os.uname().sysname} {os.uname().release}",
        f"- python: `{PY_BANDIT}`; rust: `{RS_BANDIT}` (release, LTO fat)",
        "",
    ]
    if "packages" not in args.skip:
        bench_packages(present, args.runs, out)
    if "threads" not in args.skip:
        bench_threads(target, args.runs, out)
    if "cold" not in args.skip:
        bench_cold_start(args.runs, out)
    if "formats" not in args.skip:
        bench_formats(target, args.runs, out)

    dest = Path(args.out)
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text("\n".join(out) + "\n")
    print(f"\nwritten: {dest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
