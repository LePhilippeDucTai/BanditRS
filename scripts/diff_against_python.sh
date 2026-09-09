#!/usr/bin/env bash
# Differential test harness: compare BanditRS with the reference Python bandit.
#
# Usage:
#   scripts/diff_against_python.sh [target ...]   (default: examples)
#   scripts/diff_against_python.sh --stdlib        (target: /usr/lib/python3.11)
#
# Env overrides:
#   PY_BANDIT (default /home/user/.pyenv-bandit/bin/bandit)
#   RS_BANDIT (default target/release/bandit — build it first: `cargo build --release`)
#   PYDUMP    (default /home/user/.pyenv-bandit/bin/python, used only for the walk-trace step)
#   OUT       (default target/diff)
#
# Primary check (docs/plan/wp/WP-14-golden-differential.md §5): file-by-file
# JSON reports (`bandit <file> -f json`), normalised the same way as the
# golden corpus (scripts/gen_golden.sh), compared against a whitelist of
# documented gaps (DEVIATIONS.md). Exits non-zero iff a file diverges for a
# reason that is NOT on the whitelist. Also runs, purely informationally
# (never affects the exit code): a walk-trace cross-check (AST node order)
# and the eight aggregate report formats over the target as a whole.
set -euo pipefail

PY_BANDIT=${PY_BANDIT:-/home/user/.pyenv-bandit/bin/bandit}
RS_BANDIT=${RS_BANDIT:-target/release/bandit}
PYDUMP=${PYDUMP:-/home/user/.pyenv-bandit/bin/python}
OUT=${OUT:-target/diff}

targets=()
for a in "$@"; do
  if [ "$a" = "--stdlib" ]; then
    targets+=("/usr/lib/python3.11")
  else
    targets+=("$a")
  fi
done
[ ${#targets[@]} -eq 0 ] && targets=(examples)

if [ ! -x "$PY_BANDIT" ]; then
  echo "diff_against_python.sh: reference bandit not found/executable: $PY_BANDIT" >&2
  exit 1
fi
if [ ! -x "$RS_BANDIT" ]; then
  echo "diff_against_python.sh: $RS_BANDIT not found — run 'cargo build --release' first" >&2
  exit 1
fi

mkdir -p "$OUT"

file_count=$(find "${targets[@]}" -name '*.py' | wc -l)
# The walk-trace and aggregate-format steps below are purely informational
# (the exit code never depends on them) and re-run bandit twice per file (or
# once per format over the whole target); skip them on a large target such
# as --stdlib (672 files) where they would dominate the run time without
# adding to the pass/fail signal — the authoritative step (§3) still covers
# every file.
run_informational=1
if [ "$file_count" -gt 150 ]; then
  run_informational=0
  echo "diff_against_python.sh: $file_count files — skipping the informational walk-trace/aggregate-format steps"
fi

# Whitelist of documented, expected per-file JSON gaps (DEVIATIONS.md entry
# number -> substring match against the scanned file's path). Currently
# empty: DEVIATIONS.md #10/#11 (YAML anchors / double-quoted folding) only
# manifest with multiple issues sharing one AST node inside a *recursive*
# scan aggregating several files under one report — they never show up in
# this script's single-file JSON comparison, nor in `-r examples -f json`
# (see tests/golden.rs, run without Python). Add entries here (with a
# comment citing the DEVIATIONS.md number) if a future upstream file ever
# needs one.
declare -a WHITELIST=()

is_whitelisted() {
  local f="$1"
  for pattern in "${WHITELIST[@]:-}"; do
    [ -n "$pattern" ] && [[ "$f" == *"$pattern"* ]] && return 0
  done
  return 1
}

NORM_PY="$(mktemp)"
trap 'rm -f "$NORM_PY"' EXIT
cat > "$NORM_PY" <<'PYEOF'
import re
import sys

root = sys.argv[1]
text = sys.stdin.read()

text = re.sub(r"^Working\.\.\..*\n", "", text)
text = re.sub(r'"generated_at": "[^"]*"', '"generated_at": "<TS>"', text)
text = re.sub(r"generated_at: '[^']*'", "generated_at: '<TS>'", text)
text = re.sub(r'"endTimeUtc": "[^"]*"', '"endTimeUtc": "<TS>"', text)
text = re.sub(r"Run started:[^\n]*", "Run started:<TS>", text)
text = re.sub(
    r'"version": "[^"]*",(\s*)"semanticVersion": "[^"]*"',
    r'"version": "<VER>",\1"semanticVersion": "<VER>"',
    text,
)
text = re.sub(r"readthedocs\.io/en/[^/]+/", "readthedocs.io/en/X/", text)
text = re.sub(r"0x[0-9a-fA-F]+", "0x0", text)
text = text.replace(root, "<ROOT>")
sys.stdout.write(text)
PYEOF

normalize() {
  python3 "$NORM_PY" "$(pwd)"
}

if [ "$run_informational" -eq 1 ]; then
  # 1. Walk traces (informational: AST visit order / node positions).
  echo "=== walk traces (informational) ==="
  ok=0; bad=0
  for f in $(find "${targets[@]}" -name '*.py' | sort); do
    n=$(echo "$f" | tr '/' '_')
    "$PYDUMP" scripts/dump_walk.py "$f" > "$OUT/$n.walk.py" 2>/dev/null || continue
    BANDITRS_PYTHON_COMPAT=3.11 "$RS_BANDIT" --dump-walk "$f" > "$OUT/$n.walk.rs" 2>/dev/null || true
    if diff -q "$OUT/$n.walk.py" "$OUT/$n.walk.rs" > /dev/null; then ok=$((ok+1)); else bad=$((bad+1)); echo "WALK DIFF: $f"; fi
  done
  echo "walk traces: identical=$ok different=$bad"

  # 2. Aggregate report formats over each target as a whole (informational).
  echo "=== aggregate formats (informational) ==="
  for t in "${targets[@]}"; do
    n=$(echo "$t" | tr '/' '_')
    for fmt in json txt csv xml yaml custom sarif html; do
      "$PY_BANDIT" -r -f "$fmt" "$t" 2>/dev/null | normalize > "$OUT/$n.$fmt.py" || true
      BANDITRS_PYTHON_COMPAT=3.11 "$RS_BANDIT" -r -f "$fmt" "$t" 2>/dev/null | normalize > "$OUT/$n.$fmt.rs" || true
      if diff -q "$OUT/$n.$fmt.py" "$OUT/$n.$fmt.rs" > /dev/null; then
        echo "$fmt identical: $t"
      else
        echo "$fmt DIFF: $t (see $OUT/$n.$fmt.*)"
      fi
    done
  done
fi

# 3. Primary check: file-by-file JSON reports, whitelist-gated exit code.
echo "=== file-by-file JSON (authoritative) ==="
identical=0
expected=0
unexpected=0
unexpected_files=()
while IFS= read -r f; do
  py_out="$("$PY_BANDIT" "$f" -f json 2>/dev/null | normalize || true)"
  rs_out="$(BANDITRS_PYTHON_COMPAT=3.11 "$RS_BANDIT" "$f" -f json 2>/dev/null | normalize || true)"
  if [ "$py_out" = "$rs_out" ]; then
    identical=$((identical+1))
  elif is_whitelisted "$f"; then
    expected=$((expected+1))
    echo "JSON DIFF (whitelisted): $f"
  else
    unexpected=$((unexpected+1))
    unexpected_files+=("$f")
    echo "JSON DIFF (unexpected): $f"
  fi
done < <(find "${targets[@]}" -name '*.py' | sort)

total=$((identical+expected+unexpected))
echo "file-by-file JSON: identical=$identical whitelisted=$expected unexpected=$unexpected total=$total"

if [ "$unexpected" -gt 0 ]; then
  echo "diff_against_python.sh: $unexpected unexpected diff(s):" >&2
  printf '  %s\n' "${unexpected_files[@]}" >&2
  exit 1
fi

echo "diff_against_python.sh: zero unexpected diff"
