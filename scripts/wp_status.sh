#!/usr/bin/env bash
# Progress dashboard of the test-porting work packages (docs/plan/README.md).
#
# Every Python test of the reference suite has a same-named Rust test in the
# mirror file (docs/plan/test-inventory.md). Tests not ported yet are
# `#[ignore = "WP-xx: ..."]` stubs; a WP is done when none of its stubs is
# ignored any more. This script counts them. Exit code 0 always (informational)
# unless --check is given, in which case it fails when any stub is left.
#
# Usage: scripts/wp_status.sh [--check]
set -euo pipefail
cd "$(dirname "$0")/.."

printf '%-38s %6s %8s %8s\n' "test file" "tests" "ignored" "active"
total=0; ignored=0
for f in tests/*.rs; do
  # `#[test]` at column 0 (the copy inside `macro_rules! example_test` is indented)
  # plus every `example_test!(...)` invocation, which expands to one test.
  t=$(( $(grep -c '^#\[test\]' "$f" || true) + $(grep -c '^example_test!(' "$f" || true) ))
  i=$(grep -c '^#\[ignore' "$f" || true)
  printf '%-38s %6d %8d %8d\n' "$f" "$t" "$i" "$((t - i))"
  total=$((total + t)); ignored=$((ignored + i))
done
printf '%-38s %6d %8d %8d\n' "TOTAL (integration tests)" "$total" "$ignored" "$((total - ignored))"
echo
echo "Remaining stubs per work package:"
grep -ho '#\[ignore = "WP-[0-9]*' tests/*.rs | sed 's/.*"//' | sort | uniq -c | awk '{printf "  %-6s %3d\n", $2, $1}' || true
unit=$(grep -rc '#\[test\]' src | awk -F: '{s+=$2} END {print s}')
echo
echo "Unit tests inside src/: $unit"
if [ "${1:-}" = "--check" ] && [ "$ignored" -gt 0 ]; then
  echo "ERROR: $ignored stub(s) still ignored" >&2
  exit 1
fi
