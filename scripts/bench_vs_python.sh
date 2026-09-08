#!/usr/bin/env bash
# Wall-clock comparison BanditRS (release) vs the reference Python bandit.
# Protocol: docs/plan/benchmarks.md. Owned by WP-15.
#
# Usage: scripts/bench_vs_python.sh [-n RUNS] [-f FORMAT] [target ...]
#   default targets: examples/ and /usr/lib/python3.11 (skipped when absent)
#   RUNS   number of timed runs per tool per target (default 5, median reported)
#   FORMAT bandit output format used for both tools (default json)
# Env: PY_BANDIT (default /home/user/.pyenv-bandit/bin/bandit),
#      RS_BANDIT (default target/release/bandit, built if missing),
#      OUT (default target/bench/vs_python.md).
# Uses `hyperfine` when installed (more robust statistics), else a bash loop.
set -euo pipefail
cd "$(dirname "$0")/.."

RUNS=5; FORMAT=json
while getopts "n:f:" opt; do
  case $opt in
    n) RUNS=$OPTARG ;;
    f) FORMAT=$OPTARG ;;
    *) echo "usage: $0 [-n RUNS] [-f FORMAT] [target ...]" >&2; exit 2 ;;
  esac
done
shift $((OPTIND - 1))
targets=("$@")
if [ ${#targets[@]} -eq 0 ]; then
  targets=(examples)
  [ -d /usr/lib/python3.11 ] && targets+=(/usr/lib/python3.11)
fi

PY_BANDIT=${PY_BANDIT:-/home/user/.pyenv-bandit/bin/bandit}
RS_BANDIT=${RS_BANDIT:-target/release/bandit}
OUT=${OUT:-target/bench/vs_python.md}
mkdir -p "$(dirname "$OUT")"
[ -x "$RS_BANDIT" ] || cargo build --release --quiet
[ -x "$PY_BANDIT" ] || { echo "reference bandit not found: $PY_BANDIT (see PLAN.md §2)" >&2; exit 2; }

# median of N wall-clock seconds for a command (bash fallback when hyperfine is absent)
median_secs() {
  local times=()
  for _ in $(seq "$RUNS"); do
    local s e
    s=$(date +%s.%N); "$@" >/dev/null 2>&1 || true; e=$(date +%s.%N)
    times+=("$(echo "$e - $s" | bc -l)")
  done
  printf '%s\n' "${times[@]}" | sort -n | awk '{a[NR]=$1} END {print (NR%2 ? a[(NR+1)/2] : (a[NR/2]+a[NR/2+1])/2)}'
}

{
  echo "# BanditRS vs bandit (Python) — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
  echo "- runs per measurement: $RUNS (median) ; format: \`-f $FORMAT\` ; host: $(nproc) CPU(s), $(uname -sr)"
  echo "- python: \`$PY_BANDIT\` ($("$PY_BANDIT" --version 2>&1 | head -1)) ; rust: \`$RS_BANDIT\` ($("$RS_BANDIT" --version 2>&1 | head -1))"
  echo
  echo "| target | files (*.py) | lines | python (s) | rust (s) | speed-up |"
  echo "|---|---:|---:|---:|---:|---:|"
} > "$OUT"

for t in "${targets[@]}"; do
  nfiles=$(find "$t" -name '*.py' | wc -l)
  nlines=$(find "$t" -name '*.py' -print0 | xargs -0 cat 2>/dev/null | wc -l)
  if command -v hyperfine >/dev/null; then
    json=$(mktemp)
    hyperfine -N -w 1 -r "$RUNS" -i --export-json "$json" \
      "$PY_BANDIT -r -q -f $FORMAT $t" "$RS_BANDIT -r -q -f $FORMAT $t" >/dev/null 2>&1
    py=$(python3 -c "import json,sys;print(json.load(open('$json'))['results'][0]['median'])")
    rs=$(python3 -c "import json,sys;print(json.load(open('$json'))['results'][1]['median'])")
    rm -f "$json"
  else
    py=$(median_secs "$PY_BANDIT" -r -q -f "$FORMAT" "$t")
    rs=$(median_secs "$RS_BANDIT" -r -q -f "$FORMAT" "$t")
  fi
  ratio=$(echo "$py / $rs" | bc -l)
  printf '| `%s` | %d | %d | %.3f | %.3f | %.1f× |\n' "$t" "$nfiles" "$nlines" "$py" "$rs" "$ratio" | tee -a "$OUT"
done
echo "written: $OUT"
