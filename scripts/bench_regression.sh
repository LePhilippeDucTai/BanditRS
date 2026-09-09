#!/usr/bin/env bash
# Regression guard for `benches/e2e.rs`, built on criterion's own baseline
# comparison (no `git stash` involved). Protocol: docs/plan/benchmarks.md
# §3. Owned by WP-15.
#
# Usage: scripts/bench_regression.sh [ref]
#        scripts/bench_regression.sh --record [ref]
#   ref       baseline name (default: "integration").
#   --record  save the current worktree as that baseline and refresh the
#             committed summary in benches/baseline.json.
#
# `target/criterion/` is gitignored, so on a fresh clone there is no criterion
# baseline to compare against. The old behaviour was to silently create one
# from the current worktree and then compare the worktree to itself — a guard
# that cannot fail. Instead, when no criterion baseline named <ref> exists, the
# comparison falls back to the committed summary `benches/baseline.json`, whose
# absolute timings are machine-specific but which at least catches an order-of-
# magnitude regression on the same machine. The message says which of the two
# was used, because they are not equally trustworthy.
#
# In the criterion path it re-runs the benches against the baseline
# (`cargo bench -- --baseline <ref>`) and reads every
# `target/criterion/<bench>/change/estimates.json`; in the fallback path it
# re-runs them plain and compares `new/estimates.json` to the committed means.
# Either way it exits 1 if a bench's mean regressed by more than +10%
# (criterion reports a fraction, not a percentage).
set -euo pipefail
cd "$(dirname "$0")/.."

RECORD=0
if [ "${1:-}" = "--record" ]; then
  RECORD=1
  shift
fi
REF=${1:-integration}
THRESHOLD=0.10
SUMMARY=benches/baseline.json

if [ "$RECORD" -eq 1 ]; then
  cargo bench --bench e2e -- --save-baseline "$REF"
  python3 - "$REF" "$SUMMARY" <<'PY'
import json, os, pathlib, sys, time

ref, dest = sys.argv[1], pathlib.Path(sys.argv[2])
root = pathlib.Path("target/criterion")
benches = {}
for est in sorted(root.rglob(f"{ref}/estimates.json")):
    name = str(est.parent.parent.relative_to(root))
    if name == "report":
        continue
    d = json.loads(est.read_text())
    benches[name] = {
        "mean_ns": round(d["mean"]["point_estimate"], 1),
        "median_ns": round(d["median"]["point_estimate"], 1),
        "std_dev_ns": round(d["std_dev"]["point_estimate"], 1),
    }
doc = json.loads(dest.read_text()) if dest.exists() else {}
doc.update(
    recorded_at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    host={"cpus": os.cpu_count(), "kernel": os.uname().release},
    profile="release, lto=fat, codegen-units=1",
    benches=benches,
)
dest.write_text(json.dumps(doc, indent=2) + "\n")
print(f"recorded {len(benches)} bench(es) into {dest}")
PY
  exit 0
fi

if find target/criterion -mindepth 2 -maxdepth 4 -type d -name "$REF" 2>/dev/null | grep -q .; then
  MODE="criterion baseline '$REF'"
  cargo bench --bench e2e -- --baseline "$REF"
else
  echo "no criterion baseline '$REF' under target/criterion/ — falling back to the" >&2
  echo "committed summary $SUMMARY (absolute timings, machine-specific)" >&2
  [ -f "$SUMMARY" ] || { echo "$SUMMARY missing: run scripts/bench_regression.sh --record" >&2; exit 2; }
  MODE="committed summary $SUMMARY"
  cargo bench --bench e2e
  python3 - "$SUMMARY" "$THRESHOLD" <<'PY'
import json, pathlib, sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text())["benches"]
threshold = float(sys.argv[2])
root = pathlib.Path("target/criterion")
fail = False
for name, recorded in sorted(summary.items()):
    est = root / name / "new" / "estimates.json"
    if not est.exists():
        print(f"{name:<40} {'(not run)':>10}")
        continue
    now = json.loads(est.read_text())["mean"]["point_estimate"]
    ratio = now / recorded["mean_ns"] - 1
    status = "ok"
    if ratio > threshold:
        status, fail = "REGRESSION", True
    print(f"{name:<40} {ratio * 100:+8.2f}%  {status}")
sys.exit(1 if fail else 0)
PY
  echo "OK: no bench regressed by more than +10% vs $MODE"
  exit 0
fi

echo
echo "=== regression check vs $MODE (threshold: +${THRESHOLD} = +10%) ==="
fail=0
while IFS= read -r -d '' f; do
  bench=${f#target/criterion/}
  bench=${bench%/change/estimates.json}
  point=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['mean']['point_estimate'])" "$f")
  status="ok"
  if python3 -c "import sys; sys.exit(0 if float(sys.argv[1]) > float(sys.argv[2]) else 1)" "$point" "$THRESHOLD"; then
    status="REGRESSION"
    fail=1
  fi
  printf '%-40s %+8.2f%%  %s\n' "$bench" "$(python3 -c "print(float('$point') * 100)")" "$status"
done < <(find target/criterion -path '*/change/estimates.json' -print0)

if [ "$fail" -ne 0 ]; then
  echo "FAIL: at least one bench regressed by more than +10% vs $MODE" >&2
  exit 1
fi
echo "OK: no bench regressed by more than +10% vs $MODE"
