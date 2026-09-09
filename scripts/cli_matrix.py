#!/usr/bin/env python3
"""CLI differential matrix: the drop-in claim, checked option by option (WP-18).

Every earlier harness compared one thing: the `-f json` report on stdout. A
drop-in replacement has to match more than that — the **exit code** a CI job
branches on, and the **stderr** a developer reads. This script runs the same
argv through the reference Python bandit and through BanditRS and compares all
three.

    scripts/cli_matrix.py gen     regenerate the committed golden from $PY_BANDIT
    scripts/cli_matrix.py diff    run both tools now and report divergences
    scripts/cli_matrix.py run ID  run one case with both tools, print both sides

The golden lets `cargo test --test cli_matrix` replay the whole matrix against
the Rust binary with no Python installed, exactly like `tests/golden.rs`.

Cases live in `tests/cli_matrix/cases.tsv`; the fixture tree they run against is
`tests/cli_matrix/workspace/`, copied to a temporary directory per case so that
every path in the output is relative and needs no path normalisation.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / "tests" / "cli_matrix"
WORKSPACE = BASE / "workspace"
CASES = BASE / "cases.tsv"
GOLDEN = BASE / "out"

PY_PREFIX = Path(os.environ.get("PY_BANDIT_BIN", "/home/user/.pyenv-bandit/bin"))
RS_PREFIX = Path(os.environ.get("RS_BANDIT_BIN", ROOT / "target" / "release"))

# Substitutions applied to stdout and stderr before comparison. Each entry is
# (pattern, replacement, why). Anything normalised here is content the two
# tools are *allowed* to differ on; keep the list as short as the truth allows,
# because every entry is a place where a real divergence could hide.
SUBS = [
    (r"\A\s*Working\.\.\..*\n", "", "rich progress line, DEVIATIONS #6"),
    (r'"generated_at": "[^"]*"', '"generated_at": "<TS>"', "report timestamp"),
    (r"generated_at: '[^']*'", "generated_at: '<TS>'", "report timestamp (yaml)"),
    (r'"endTimeUtc": "[^"]*"', '"endTimeUtc": "<TS>"', "sarif timestamp"),
    (r"Run started:[^\n]*", "Run started:<TS>", "txt/screen timestamp"),
    (
        r'"version": "[^"]*",(\s*)"semanticVersion": "[^"]*"',
        r'"version": "<VER>",\1"semanticVersion": "<VER>"',
        "sarif driver version",
    ),
    (r"readthedocs\.io/en/[^/]+/", "readthedocs.io/en/X/", "docs URL version, DEVIATIONS #12"),
    (r"0x[0-9a-fA-F]+", "0x0", "memory addresses, DEVIATIONS #5"),
    (r"(?m)^bandit \S+$", "bandit <VER>", "--version"),
    (r"(?m)^\s*python version = .*\n", "", "--version python line, DEVIATIONS #6"),
    (r"(?m)^\[main\]\tINFO\trunning on Python .*\n", "", "startup log line, DEVIATIONS #6"),
    # `bandit-baseline` builds a throwaway git repo and a scratch directory; the
    # commit shas depend on the second the run happened in, and the scratch
    # directory is named by each tool's own mkdtemp. Neither is a behavioural
    # difference, and both appear only in bandit-baseline's own log lines.
    (r"\b[0-9a-f]{40}\b", "<SHA>", "git commit shas, one repo per tool per run"),
    (r"/tmp/[^/\s]+/_bandit_baseline_run\.json_", "<TMPDIR>/_bandit_baseline_run.json_", "scratch dir"),
]


# `bandit/cli/main.py:_log_info` joins `profile["include"]`, a Python *set*, so
# its order comes out of a randomised string hash and changes from one run to
# the next (verified: five runs of the same command gave three different
# orders). BanditRS iterates a deterministic collection instead. Sorting both
# sides is the only way to compare the line at all; DEVIATIONS.md #17 records
# that BanditRS is the reproducible one here.
PROFILE_LINE = re.compile(r"(?m)^(\[main\]\tINFO\tprofile (?:include|exclude) tests: )(.+)$")


def _sort_profile_ids(match):
    ids = match.group(2)
    if ids == "None":
        return match.group(0)
    return match.group(1) + ",".join(sorted(ids.split(",")))


def normalize(text):
    for pattern, repl, _ in SUBS:
        text = re.sub(pattern, repl, text)
    return PROFILE_LINE.sub(_sort_profile_ids, text)


# Cases where the two tools are *expected* to differ, each keyed to the
# DEVIATIONS.md entry that explains why. For these the golden records the Rust
# output (so the replay still catches a Rust regression) and `diff` reports
# them as expected rather than as failures. Adding an entry here is a decision,
# not a shortcut: it needs a DEVIATIONS.md number.
EXPECTED = {
    "debug": (18, "-d dumps the Python Context dict, ast node reprs included"),
    "fmt_yaml": (10, "PyYAML anchors/aliases for a shared line_range list"),
    "config_bad": (19, "PyYAML's parse-error wording for an invalid config"),
}


def read_cases():
    cases = []
    for line in CASES.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        cid, binary, prep, args = line.split("\t")
        cases.append({"id": cid, "bin": binary, "prep": prep, "args": args.split()})
    return cases


def make_workspace(prep, tool_dir):
    ws = Path(tempfile.mkdtemp(prefix="cli_matrix_"))
    shutil.copytree(WORKSPACE, ws, dirs_exist_ok=True, symlinks=True)
    if prep == "git":
        env = {
            **os.environ,
            "GIT_AUTHOR_NAME": "t",
            "GIT_AUTHOR_EMAIL": "t@e",
            "GIT_COMMITTER_NAME": "t",
            "GIT_COMMITTER_EMAIL": "t@e",
        }
        run = lambda *a: subprocess.run(a, cwd=ws, env=env, capture_output=True)
        run("git", "init", "-q", "-b", "main")
        run("git", "add", "-A")
        run("git", "commit", "-qm", "base")
        (ws / "src" / "extra.py").write_text("import subprocess\nsubprocess.call('ls', shell=True)\n")
        run("git", "add", "-A")
        run("git", "commit", "-qm", "change")
    elif prep == "selfbaseline":
        subprocess.run(
            [str(tool_dir / "bandit"), "src/assert.py", "-f", "json", "-o", "baseline.json"],
            cwd=ws,
            capture_output=True,
            env=tool_env(tool_dir),
        )
    return ws


def tool_env(tool_dir):
    """`bandit-baseline` shells out to plain `bandit`, so each tool has to find
    *its own* binary first on PATH, not the other one's."""
    return {
        **os.environ,
        "PATH": f"{tool_dir}{os.pathsep}{os.environ.get('PATH', '')}",
        "BANDITRS_PYTHON_COMPAT": "3.11",
        "COLUMNS": "80",
        "NO_COLOR": "1",
    }


def execute(case, tool_dir):
    """Run one case; return (stdout, stderr, exit code), all normalised."""
    ws = make_workspace(case["prep"], tool_dir)
    try:
        # Bytes, not `text=True`: universal-newline translation would rewrite
        # the CRLF line endings Python's own `csv` module emits, hiding a real
        # difference in the `-f csv` output behind the harness.
        proc = subprocess.run(
            [str(tool_dir / case["bin"]), *case["args"]],
            cwd=ws,
            capture_output=True,
            env=tool_env(tool_dir),
        )
        out = proc.stdout.decode("utf-8", "replace")
        # `-o FILE` puts the report in a file, which is the part that matters;
        # fold it into stdout so the comparison covers it.
        for arg in case["args"]:
            written = ws / arg
            if arg.startswith("report.") and written.exists():
                out += f"\n--- file {arg} ---\n" + written.read_bytes().decode("utf-8", "replace")
        # `{abspath}` (custom format) and error paths print the workspace's
        # absolute path, which is a fresh temp dir per tool per run.
        ws_str = str(ws)
        err = proc.stderr.decode("utf-8", "replace")
        return (
            normalize(out).replace(ws_str, "<WS>"),
            normalize(err).replace(ws_str, "<WS>"),
            proc.returncode,
        )
    finally:
        shutil.rmtree(ws, ignore_errors=True)


def write_golden(case, out, err, code):
    GOLDEN.mkdir(parents=True, exist_ok=True)
    (GOLDEN / f"{case['id']}.out").write_text(out)
    (GOLDEN / f"{case['id']}.err").write_text(err)
    (GOLDEN / f"{case['id']}.code").write_text(f"{code}\n")


def cmd_gen(args):
    cases = read_cases()
    if GOLDEN.exists():
        shutil.rmtree(GOLDEN)
    total = 0
    for case in cases:
        expected = EXPECTED.get(case["id"])
        source = RS_PREFIX if expected else PY_PREFIX
        out, err, code = execute(case, source)
        write_golden(case, out, err, code)
        if expected:
            number, why = expected
            (GOLDEN / f"{case['id']}.expected").write_text(
                f"DEVIATIONS.md #{number}: {why}\n"
                "Golden recorded from BanditRS, not from Python: the two are\n"
                "known to differ here. The replay still guards against a Rust\n"
                "regression; scripts/cli_matrix.py diff reports it as expected.\n"
            )
        total += len(out) + len(err)
        tag = f"  (expected diff, DEVIATIONS #{expected[0]})" if expected else ""
        print(f"{case['id']:<28} exit={code} out={len(out)}B err={len(err)}B{tag}")
    print(f"\nwrote {len(cases)} case(s) to {GOLDEN} ({total / 1024:.0f} KiB)")
    return 0


def compare(case):
    po, pe, pc = execute(case, PY_PREFIX)
    ro, re_, rc = execute(case, RS_PREFIX)
    diffs = []
    if pc != rc:
        diffs.append(f"exit code {pc} vs {rc}")
    if po != ro:
        diffs.append("stdout")
    if pe != re_:
        diffs.append("stderr")
    return diffs, (po, pe, pc), (ro, re_, rc)


def cmd_diff(args):
    cases = read_cases()
    if args.only:
        cases = [c for c in cases if c["id"] in args.only]
    bad, expected = [], []
    for case in cases:
        diffs, _, _ = compare(case)
        known = EXPECTED.get(case["id"])
        if diffs and known:
            expected.append(case["id"])
            print(f"note {case['id']:<28} {', '.join(diffs)} (expected, DEVIATIONS #{known[0]})")
        elif diffs:
            bad.append((case["id"], diffs))
            print(f"DIFF {case['id']:<28} {', '.join(diffs)}")
        elif known:
            print(f"NOTE {case['id']:<28} no longer differs — drop its EXPECTED entry")
        elif args.verbose:
            print(f"ok   {case['id']}")
    identical = len(cases) - len(bad) - len(expected)
    print(
        f"\ncli matrix: {identical}/{len(cases)} identical on stdout, stderr and exit code"
        f"; {len(expected)} expected divergence(s); {len(bad)} unexpected"
    )
    if bad:
        print("unexpected:", ", ".join(i for i, _ in bad), file=sys.stderr)
        return 1
    return 0


def cmd_run(args):
    case = next((c for c in read_cases() if c["id"] == args.id), None)
    if case is None:
        print(f"no such case: {args.id}", file=sys.stderr)
        return 2
    diffs, py, rs = compare(case)
    for label, (out, err, code) in (("python", py), ("rust", rs)):
        print(f"===== {label}: exit={code} =====")
        print("--- stdout ---");  print(out, end="")
        print("--- stderr ---");  print(err, end="")
    print("=====", "IDENTICAL" if not diffs else f"DIFFERS: {', '.join(diffs)}")
    return 1 if diffs else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("gen").set_defaults(fn=cmd_gen)
    p = sub.add_parser("diff")
    p.add_argument("--only", nargs="*")
    p.add_argument("-v", "--verbose", action="store_true")
    p.set_defaults(fn=cmd_diff)
    p = sub.add_parser("run")
    p.add_argument("id")
    p.set_defaults(fn=cmd_run)
    args = ap.parse_args()
    raise SystemExit(args.fn(args))


if __name__ == "__main__":
    main()
