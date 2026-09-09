#!/usr/bin/env bash
# Porte de qualité locale — remplace la CI GitHub (désactivée : voir
# .github/workflows/ci.yml.disabled et docs/plan/wp/WP-16-ci.md).
#
# Reproduit à l'identique les trois jobs du workflow :
#   test   — build de toutes les cibles + suite complète + tableau de bord des stubs
#   lint   — rustfmt + clippy avec -D warnings
#   bench  — compilation des benchs criterion (sans mesure)
#
# Usage :
#   scripts/check.sh            # tout
#   scripts/check.sh fast       # sans les benchs (boucle de développement)
#
# Sortie 0 = équivalent d'une CI verte. À lancer avant chaque commit et
# systématiquement avant un push.
set -uo pipefail
cd "$(dirname "$0")/.."

MODE=${1:-full}
FAILED=()

run() {
  local label=$1; shift
  printf '\n\033[1m▶ %s\033[0m\n' "$label"
  if "$@"; then
    printf '\033[32m  ✓ %s\033[0m\n' "$label"
  else
    printf '\033[31m  ✗ %s\033[0m\n' "$label"
    FAILED+=("$label")
  fi
}

# --- job « test » ---------------------------------------------------------
run "build (toutes cibles)"      cargo build --all-targets --locked
run "suite de tests"             cargo test --all-targets --locked
run "stubs restants (wp_status)" ./scripts/wp_status.sh --check

# --- job « lint » ---------------------------------------------------------
run "rustfmt"                    cargo fmt --all --check
run "clippy (-D warnings)"       cargo clippy --all-targets --locked -- -D warnings

# --- job « bench » --------------------------------------------------------
if [ "$MODE" != "fast" ]; then
  run "compilation des benchs"   cargo bench --no-run --locked
fi

# --- verdict --------------------------------------------------------------
echo
if [ ${#FAILED[@]} -eq 0 ]; then
  printf '\033[32m╭──────────────────────────────────────╮\n'
  printf '│  TOUT EST VERT — prêt à committer    │\n'
  printf '╰──────────────────────────────────────╯\033[0m\n'
  exit 0
else
  printf '\033[31m╭──────────────────────────────────────╮\n'
  printf '│  ÉCHEC — ne pas pousser               │\n'
  printf '╰──────────────────────────────────────╯\033[0m\n'
  printf 'Étapes en échec :\n'
  printf '  - %s\n' "${FAILED[@]}"
  exit 1
fi
