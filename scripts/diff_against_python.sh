#!/usr/bin/env bash
# Differential test harness: compare BanditRS with the reference Python bandit.
#
# Usage: scripts/diff_against_python.sh [target ...]   (default: examples/)
# Requires: a venv with the reference bandit installed, e.g.
#   python3 -m venv .venv-bandit && .venv-bandit/bin/pip install -e /home/user/bandit[toml,yaml,sarif]
#   export PY_BANDIT=.venv-bandit/bin/bandit
set -euo pipefail
PY_BANDIT=${PY_BANDIT:-/home/user/.pyenv-bandit/bin/bandit}
RS_BANDIT=${RS_BANDIT:-target/release/bandit}
OUT=${OUT:-target/diff}
mkdir -p "$OUT"
targets=("$@")
[ ${#targets[@]} -eq 0 ] && targets=(examples)

# 1. Walk traces (Python 3.11 reference -> 3.11 position policy on the Rust side).
PYDUMP=${PYDUMP:-/home/user/.pyenv-bandit/bin/python}
ok=0; bad=0
for f in $(find "${targets[@]}" -name '*.py' | sort); do
  n=$(echo "$f" | tr '/' '_')
  "$PYDUMP" scripts/dump_walk.py "$f" > "$OUT/$n.walk.py" 2>/dev/null || continue
  BANDITRS_PYTHON_COMPAT=3.11 "$RS_BANDIT" --dump-walk "$f" > "$OUT/$n.walk.rs" 2>/dev/null || true
  if diff -q "$OUT/$n.walk.py" "$OUT/$n.walk.rs" > /dev/null; then ok=$((ok+1)); else bad=$((bad+1)); echo "WALK DIFF: $f"; fi
done
echo "walk traces: identical=$ok different=$bad"

# 2. JSON reports (normalise generated_at and the docs version).
normalise() { sed -E 's/"generated_at": "[^"]*"/"generated_at": "X"/; s#readthedocs.io/en/[^/]+/#readthedocs.io/en/X/#'; }
for t in "${targets[@]}"; do
  n=$(echo "$t" | tr '/' '_')
  "$PY_BANDIT" -r -f json "$t" 2>/dev/null | normalise > "$OUT/$n.json.py" || true
  BANDITRS_PYTHON_COMPAT=3.11 "$RS_BANDIT" -r -f json "$t" 2>/dev/null | normalise > "$OUT/$n.json.rs" || true
  if diff -q "$OUT/$n.json.py" "$OUT/$n.json.rs" > /dev/null; then echo "JSON identical: $t"; else echo "JSON DIFF: $t (see $OUT/$n.json.*)"; fi
  for fmt in txt csv xml yaml custom sarif html; do
    "$PY_BANDIT" -r -f $fmt "$t" 2>/dev/null | normalise > "$OUT/$n.$fmt.py" || true
    BANDITRS_PYTHON_COMPAT=3.11 "$RS_BANDIT" -r -f $fmt "$t" 2>/dev/null | normalise > "$OUT/$n.$fmt.rs" || true
    if diff -q "$OUT/$n.$fmt.py" "$OUT/$n.$fmt.rs" > /dev/null; then echo "$fmt identical: $t"; else echo "$fmt DIFF: $t"; fi
  done
done
