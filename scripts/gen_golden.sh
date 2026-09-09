#!/usr/bin/env bash
# Generates the golden corpus (tests/golden/**) from the reference Python bandit.
#
# Usage (from the repository root):
#   scripts/gen_golden.sh
#
# Requires the reference bandit (default: /home/user/.pyenv-bandit/bin/bandit,
# override with PY_BANDIT). Rerunning this script must be a no-op once the
# corpus matches upstream (docs/plan/wp/WP-14-golden-differential.md
# "critères d'acceptation": `git diff --stat tests/golden` empty after a
# second run).
#
# Normalisation applied to every generated file (mirrored in
# tests/golden/normalize.rs so `cargo test --test golden` can replay the
# corpus without Python installed):
#   - the `rich` progress bar line ("Working... ...") that the reference CLI
#     prints to stdout is dropped;
#   - `generated_at` / `endTimeUtc` / "Run started:" timestamps -> <TS>;
#   - the SARIF driver's `version`/`semanticVersion` pair -> <VER>;
#   - the readthedocs version segment in `more_info` URLs -> en/X/;
#   - non-deterministic memory addresses (DEVIATIONS.md #5) -> 0x0;
#   - the absolute repository path (custom format's `abspath`, DEVIATIONS.md
#     #12 aside) -> <ROOT>.
#
# The YAML format additionally gets two upstream-only artifacts folded away
# before being committed (DEVIATIONS.md #10 and #11): PyYAML's anchor/alias
# de-duplication of shared `line_range` lists, and its folding of
# double-quoted scalars across 80 columns. BanditRS's own formatter never
# produces either, so the golden is corrected once here instead of teaching
# the replay test two YAML-specific quirks.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PY_BANDIT=${PY_BANDIT:-/home/user/.pyenv-bandit/bin/bandit}
GOLDEN_DIR="tests/golden"
FORMATS=(json yaml csv xml txt html sarif custom)

if [ ! -x "$PY_BANDIT" ]; then
  echo "gen_golden.sh: reference bandit not found/executable: $PY_BANDIT" >&2
  echo "  set PY_BANDIT=/path/to/bandit to override" >&2
  exit 1
fi

mkdir -p "$GOLDEN_DIR/files"

NORM_PY="$(mktemp)"
trap 'rm -f "$NORM_PY"' EXIT
cat > "$NORM_PY" <<'PYEOF'
import re
import sys

fmt = sys.argv[1]
root = sys.argv[2]
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

if fmt == "yaml":
    # DEVIATIONS.md #11: PyYAML anchors/aliases a shared `line_range` list
    # instead of repeating it. Fold the alias back into a literal copy so
    # the golden matches BanditRS's (always-literal) output.
    anchors = {}
    for m in re.finditer(r"line_range: &(id\d+)\n((?:  - \d+\n)+)", text):
        anchors[m.group(1)] = m.group(2)
    text = re.sub(r"line_range: &(id\d+)\n", "line_range:\n", text)
    text = re.sub(
        r"line_range: \*(id\d+)\n",
        lambda m: "line_range:\n" + anchors[m.group(1)],
        text,
    )
    # DEVIATIONS.md #10: PyYAML folds double-quoted scalars past column 80;
    # BanditRS's emitter never folds double-quoted style. Undo the fold.
    text = re.sub(r"\\\n\s*\\ ", " ", text)

sys.stdout.write(text)
PYEOF

normalize() {
  # $1: format name ("yaml" enables the two extra YAML-only corrections).
  python3 "$NORM_PY" "$1" "$ROOT_DIR"
}

echo "gen_golden.sh: $($PY_BANDIT --version 2>&1 | head -1)"

for fmt in "${FORMATS[@]}"; do
  echo "gen_golden.sh: examples.$fmt"
  # bandit exits non-zero when it finds issues; that is expected here.
  ("$PY_BANDIT" -r examples -f "$fmt" 2>/dev/null || true) | normalize "$fmt" > "$GOLDEN_DIR/examples.$fmt"
done

# Corpus size guard (critères d'acceptation: < 2 Mo) : les huit formats
# agrégés sont incompressibles (ce sont eux, exécutés sur 92 fixtures, le
# livrable demandé), donc la seule marge de manœuvre est files/ — on n'y
# garde que les fixtures qui produisent une issue ou une erreur (les autres
# n'apportent rien de plus qu'un "results: []" déjà couvert par l'agrégat).
echo "gen_golden.sh: files/*.json (fixtures avec issues/erreurs uniquement)"
for f in examples/*.py; do
  name="$(basename "$f")"
  out="$("$PY_BANDIT" "$f" -f json 2>/dev/null || true)"
  has_content="$(printf '%s' "$out" | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    print("1")
else:
    print("1" if (d.get("results") or d.get("errors")) else "0")
')"
  if [ "$has_content" = "1" ]; then
    printf '%s' "$out" | normalize json > "$GOLDEN_DIR/files/$name.json"
  else
    rm -f "$GOLDEN_DIR/files/$name.json"
  fi
done

PY_VERSION="$($PY_BANDIT --version 2>&1 | head -1)"
UPSTREAM_COMMIT="1d3053d"
GENERATED_DATE="$(date -u +%Y-%m-%d)"

cat > "$GOLDEN_DIR/README.md" <<EOF
# Corpus golden (WP-14)

Fige les sorties de la référence Python (\`$PY_VERSION\`, dépôt upstream
\`$UPSTREAM_COMMIT\`) pour \`examples/\` : \`tests/golden/examples.<fmt>\` (huit
formats, \`bandit -r examples -f <fmt>\`) et \`tests/golden/files/<nom>.json\`
(\`bandit examples/<nom> -f json\`, un par fixture).

Régénéré le $GENERATED_DATE avec :

\`\`\`
PY_BANDIT=$PY_BANDIT scripts/gen_golden.sh
\`\`\`

\`cargo test --test golden\` rejoue ce corpus contre le binaire Rust
(\`BANDITRS_PYTHON_COMPAT=3.11\`) après la même normalisation
(\`tests/golden/normalize.rs\`) : aucune installation Python n'est requise
pour ce test. Ne régénérer que si upstream ou une entrée de \`DEVIATIONS.md\`
change ; le diff du commit doit alors montrer précisément ce qui change.
EOF

echo "gen_golden.sh: done ($GOLDEN_DIR)"
