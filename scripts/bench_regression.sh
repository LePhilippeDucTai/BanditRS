#!/usr/bin/env bash
# Regression guard for `benches/e2e.rs`, built on criterion's own baseline
# comparison (no `git stash` involved). Protocol: docs/plan/benchmarks.md
# §3. Owned by WP-15.
#
# Usage: scripts/bench_regression.sh [ref]
#   ref   baseline name (default: "integration"). If `target/criterion/*/<ref>`
#         does not exist yet, it is created from the current worktree first
#         (`cargo bench -- --save-baseline <ref>`) — run that once on the
#         integration branch (main / a merged wp/* branch) before comparing
#         a feature branch against it.
#
# Then this script re-runs the benches against that baseline
# (`cargo bench -- --baseline <ref>`) and parses every
# `target/criterion/<bench>/change/estimates.json` produced: it fails
# (exit 1) if any bench's `mean.point_estimate` regressed by more than
# +10% (i.e. point_estimate > 0.10; criterion reports a fraction, not a
# percentage). Demo run (baseline = HEAD, comparison = HEAD): 0% for every
# bench, see docs/plan/wp/WP-15-benchmarks.md "Critères d'acceptation".
set -euo pipefail
cd "$(dirname "$0")/.."

REF=${1:-integration}
THRESHOLD=0.10

if ! find target/criterion -mindepth 2 -maxdepth 4 -type d -name "$REF" 2>/dev/null | grep -q .; then
  echo "baseline '$REF' not found under target/criterion/, creating it from the current worktree" >&2
  cargo bench --bench e2e -- --save-baseline "$REF"
fi

cargo bench --bench e2e -- --baseline "$REF"

echo
echo "=== regression check vs baseline '$REF' (threshold: +${THRESHOLD} = +10%) ==="
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
  echo "FAIL: at least one bench regressed by more than +10% vs baseline '$REF'" >&2
  exit 1
fi
echo "OK: no bench regressed by more than +10% vs baseline '$REF'"
