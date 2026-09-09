#!/usr/bin/env python3
"""Differential over real third-party Python code (WP-19).

`scripts/diff_against_python.sh` forks both binaries **once per file**. That is
fine for `examples/` (94 files) but not for a real corpus: at ~0.2 s of Python
start-up per file, 28 000 files would cost about an hour and a half in process
spawns alone. So this script inverts the shape:

  1. one `-r <package> -f json` per tool per package (a few dozen forks total);
  2. a structured comparison of the two reports — `results[]` in order,
     `errors[]`, and the `metrics` block, which also proves the two tools
     discovered exactly the same set of files with the same line counts;
  3. bisection **only** into a package that diverged, falling back to the
     per-file comparison there to name the culprit.

    scripts/diff_corpus.py [--tier smoke|standard|full|frontier] [--json OUT]

Exits non-zero iff a package in a parity tier diverges. The `frontier` tier is
expected to diverge (DEVIATIONS.md #16) and never affects the exit code; it is
reported with the divergence quantified instead.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import corpus  # noqa: E402  (same directory, shared manifest reader)

ROOT = Path(__file__).resolve().parent.parent
PY_BANDIT = Path(os.environ.get("PY_BANDIT", "/home/user/.pyenv-bandit/bin/bandit"))
RS_BANDIT = Path(os.environ.get("RS_BANDIT", ROOT / "target" / "release" / "bandit"))

# Same normalisation as gen_golden.sh / cli_matrix.py, applied before parsing.
DOCS_URL = re.compile(r"readthedocs\.io/en/[^/]+/")
ADDRESS = re.compile(r"0x[0-9a-fA-F]+")


# Parser target handed to BanditRS. `BANDITRS_PYTHON_COMPAT=3.11` makes it
# reject exactly what CPython 3.11 rejects, which is what a comparison against a
# 3.11 reference requires. Setting it to "latest" instead measures what the
# *default* target costs in agreement — see DEVIATIONS.md #16.
RS_COMPAT = "3.11"


def scan(tool, target):
    """Run one tool over one package; return (report, seconds, exit code)."""
    env = {**os.environ}
    if tool == RS_BANDIT:
        if RS_COMPAT == "latest":
            env.pop("BANDITRS_PYTHON_COMPAT", None)
        else:
            env["BANDITRS_PYTHON_COMPAT"] = RS_COMPAT
    started = time.monotonic()
    proc = subprocess.run(
        [str(tool), "-r", str(target), "-f", "json", "-q"],
        capture_output=True,
        env=env,
    )
    elapsed = time.monotonic() - started
    text = proc.stdout.decode("utf-8", "replace")
    text = DOCS_URL.sub("readthedocs.io/en/X/", ADDRESS.sub("0x0", text))
    try:
        report = json.loads(text)
    except json.JSONDecodeError:
        return None, elapsed, proc.returncode
    report.pop("generated_at", None)
    return report, elapsed, proc.returncode


def key(issue):
    """The identity of an issue: everything a consumer would act on."""
    return (
        issue["filename"],
        issue["line_number"],
        issue["col_offset"],
        issue["end_col_offset"],
        issue["test_id"],
        issue["issue_severity"],
        issue["issue_confidence"],
        issue["issue_text"],
        issue["code"],
    )


def compare(py, rs):
    """Structured comparison; returns a list of human-readable differences."""
    diffs = []
    if py is None or rs is None:
        return ["one of the tools produced no parsable JSON report"]

    pk, rk = [key(i) for i in py["results"]], [key(i) for i in rs["results"]]
    if pk != rk:
        sp, sr = set(pk), set(rk)
        only_py, only_rs = sp - sr, sr - sp
        if only_py or only_rs:
            diffs.append(
                f"issues differ: {len(only_py)} only in python, {len(only_rs)} only in rust"
            )
            for k in list(only_py)[:3]:
                diffs.append(f"    only python: {k[0]}:{k[1]} {k[4]}")
            for k in list(only_rs)[:3]:
                diffs.append(f"    only rust:   {k[0]}:{k[1]} {k[4]}")
        else:
            diffs.append("same issues, different order")

    pe = {(e["filename"], e["reason"]) for e in py.get("errors", [])}
    re_ = {(e["filename"], e["reason"]) for e in rs.get("errors", [])}
    if pe != re_:
        diffs.append(f"errors differ: {len(pe - re_)} only in python, {len(re_ - pe)} only in rust")
        for f, r in list(pe - re_)[:3]:
            diffs.append(f"    only python: {f}: {r}")
        for f, r in list(re_ - pe)[:3]:
            diffs.append(f"    only rust:   {f}: {r}")

    if py["metrics"] != rs["metrics"]:
        pm, rm = py["metrics"], rs["metrics"]
        missing = set(pm) - set(rm)
        extra = set(rm) - set(pm)
        if missing or extra:
            diffs.append(
                f"file discovery differs: {len(missing)} file(s) only python, "
                f"{len(extra)} only rust"
            )
        for name in sorted(set(pm) & set(rm)):
            if pm[name] != rm[name]:
                diffs.append(f"    metrics differ for {name}: {pm[name]} vs {rm[name]}")
                break
    return diffs


def bisect(target, limit=25):
    """Per-file comparison inside one divergent package, to name the culprit."""
    print(f"    bisecting {target} file by file …")
    named = []
    for path in sorted(Path(target).rglob("*.py")):
        py, _, _ = scan(PY_BANDIT, path)
        rs, _, _ = scan(RS_BANDIT, path)
        if compare(py, rs):
            named.append(str(path))
            print(f"    culprit: {path}")
            if len(named) >= limit:
                print(f"    (stopping after {limit} culprits)")
                break
    return named


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--tier", choices=["smoke", "standard", "full", "frontier", "all"], default="smoke")
    ap.add_argument("--json", help="write the per-package table here (for the parity report)")
    ap.add_argument("--no-bisect", action="store_true")
    ap.add_argument(
        "--rs-compat",
        default="3.11",
        help="parser target for BanditRS: 3.11 to match the reference interpreter "
        "(default), or 'latest' to measure what the default target costs",
    )
    args = ap.parse_args()
    global RS_COMPAT
    # With the target aligned the frontier tier matches the reference exactly,
    # which is true but says nothing; the tier is only informative under the
    # default target, so that is what it uses unless asked otherwise.
    if args.tier == "frontier" and "--rs-compat" not in sys.argv:
        args.rs_compat = "latest"
    RS_COMPAT = args.rs_compat

    for tool in (PY_BANDIT, RS_BANDIT):
        if not os.access(tool, os.X_OK):
            print(f"diff_corpus.py: not executable: {tool}", file=sys.stderr)
            return 2

    rows = corpus.select(corpus.read_manifest(), args.tier)
    table, unexpected = [], []
    for row in rows:
        target = corpus.target_dir(row)
        if not target.exists():
            print(f"MISSING {row['name']} — run `scripts/corpus.py fetch --tier {args.tier}`")
            unexpected.append(row["name"])
            continue
        py, py_secs, _ = scan(PY_BANDIT, target)
        rs, rs_secs, _ = scan(RS_BANDIT, target)
        diffs = compare(py, rs)
        frontier = row["tier"] == "frontier"
        entry = {
            "tier": row["tier"],
            "name": row["name"],
            "version": row["version"],
            "py_files": int(row["py_files"]) if row["py_files"].isdigit() else 0,
            "py_lines": int(row["py_lines"]) if row["py_lines"].isdigit() else 0,
            "issues_python": len(py["results"]) if py else None,
            "issues_rust": len(rs["results"]) if rs else None,
            "errors_python": len(py.get("errors", [])) if py else None,
            "errors_rust": len(rs.get("errors", [])) if rs else None,
            "seconds_python": round(py_secs, 3),
            "seconds_rust": round(rs_secs, 3),
            "identical": not diffs,
            "diffs": diffs,
            "test_ids": sorted({i["test_id"] for i in py["results"]}) if py else [],
        }
        table.append(entry)

        label = f"{row['name']} {row['version']}"
        if not diffs:
            speed = py_secs / rs_secs if rs_secs else 0
            print(
                f"ok       {label:<28} {entry['issues_python']:>6} issues, "
                f"{entry['py_files']:>5} files, {speed:>5.1f}x"
            )
        elif frontier:
            print(f"frontier {label:<28} expected divergence (DEVIATIONS #16)")
            for d in diffs:
                print(f"    {d}")
        else:
            print(f"DIFF     {label:<28}")
            for d in diffs:
                print(f"    {d}")
            unexpected.append(row["name"])
            if not args.no_bisect:
                entry["culprits"] = bisect(target)

    parity = [e for e in table if e["tier"] != "frontier"]
    files = sum(e["py_files"] for e in parity)
    lines = sum(e["py_lines"] for e in parity)
    issues = sum(e["issues_python"] or 0 for e in parity)
    ids = sorted({i for e in parity for i in e["test_ids"]})
    print(
        f"\n{len(parity) - len(unexpected)}/{len(parity)} package(s) identical — "
        f"{files} files, {lines} lines, {issues} issues, {len(ids)} distinct test ids"
    )

    if args.json:
        Path(args.json).parent.mkdir(parents=True, exist_ok=True)
        Path(args.json).write_text(json.dumps(table, indent=2) + "\n")
        print(f"wrote {args.json}")

    if unexpected:
        print("unexpected divergence in: " + ", ".join(unexpected), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
