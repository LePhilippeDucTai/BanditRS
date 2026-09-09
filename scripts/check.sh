#!/usr/bin/env bash
# Porte de qualité locale — miroir de .github/workflows/ci.yml, à lancer avant
# de pousser pour obtenir le même verdict sans attendre la CI.
#
# Reproduit à l'identique les quatre jobs du workflow :
#   test   — build de toutes les cibles + suite complète + tableau de bord des stubs
#   lint   — rustfmt + clippy avec -D warnings
#   bench  — compilation des benchs criterion (sans mesure)
#   python — construction du wheel + fumée des quatre commandes (mode « python »)
#   parity — corpus réel + matrice CLI face au bandit Python de référence
#            (mode « parity » ; demande le réseau et l'exécutable de référence)
#
# Usage :
#   scripts/check.sh            # tout sauf le wheel et la parité (rapide à répéter)
#   scripts/check.sh fast       # sans les benchs (boucle de développement)
#   scripts/check.sh python     # uniquement le job « python » (wheel + parité)
#   scripts/check.sh parity     # uniquement le job « parity » (réseau requis)
#   scripts/check.sh all        # tout — à lancer avant un push
#
# Le mode par défaut n'appelle pas le réseau : la matrice CLI et le corpus golden
# sont rejoués par `cargo test` (tests/cli_matrix.rs, tests/golden.rs) sans aucune
# installation Python. Le mode « parity » est celui qui va réellement interroger
# le bandit de référence.
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
if [ "$MODE" != "python" ]; then
  run "build (toutes cibles)"      cargo build --all-targets --locked
  run "suite de tests"             cargo test --all-targets --locked
  run "stubs restants (wp_status)" ./scripts/wp_status.sh --check
fi

# --- job « lint » ---------------------------------------------------------
if [ "$MODE" != "python" ]; then
  run "rustfmt"                    cargo fmt --all --check
  run "clippy (-D warnings)"       cargo clippy --all-targets --locked -- -D warnings
fi

# --- job « bench » --------------------------------------------------------
if [ "$MODE" != "fast" ] && [ "$MODE" != "python" ]; then
  run "compilation des benchs"   cargo bench --no-run --locked
fi

# --- job « python » -------------------------------------------------------
# Reproduit .github/workflows/ci.yml (job « python ») et y ajoute une
# vérification que la CI ne fait pas : la sortie du binaire empaqueté dans le
# wheel doit être *identique* à celle du binaire construit par cargo. C'est la
# garantie que l'empaquetage n'a rien changé au comportement.
python_job() {
  local venv; venv=$(mktemp -d)/venv
  python3 -m venv "$venv" || return 1
  "$venv/bin/pip" install --quiet --upgrade pip || return 1
  "$venv/bin/pip" install --quiet . || return 1

  "$venv/bin/bandit" --version                        || return 1
  "$venv/bin/banditrs" --version                      || return 1
  "$venv/bin/bandit-baseline" --help > /dev/null      || return 1
  "$venv/bin/bandit-config-generator" --show-defaults > /dev/null || return 1
  "$venv/bin/bandit" -r examples/ --exit-zero > /dev/null         || return 1
  "$venv/bin/python" -m banditrs --version            || return 1
  "$venv/bin/python" -c 'import banditrs; banditrs.find_bandit_bin()' || return 1

  # Parité wheel <-> cargo, sur l'intégralité du corpus examples/.
  # `generated_at` est un horodatage à la seconde (src/formatters/json.rs) : il
  # diffère entre deux exécutions et doit être filtré, sinon le test est
  # instable au lieu d'être informatif.
  cargo build --release --locked --bin bandit || return 1
  diff <("$venv/bin/bandit" -r examples/ -f json 2>/dev/null | grep -v '"generated_at"') \
       <(target/release/bandit  -r examples/ -f json 2>/dev/null | grep -v '"generated_at"') \
    || return 1

  rm -rf "$(dirname "$venv")"
}

if [ "$MODE" = "python" ] || [ "$MODE" = "all" ]; then
  run "wheel python (pip install + fumée + parité)" python_job
fi

# --- job « parity » : face au bandit Python de référence -------------------
# Le tier « smoke » du corpus (8 paquets, ~570 fichiers) tient en quelques
# secondes une fois téléchargé ; c'est le sous-ensemble prévu pour cette porte.
# Les tiers « standard » et « full » sont des campagnes, lancées à la demande.
parity_job() {
  local py_bandit=${PY_BANDIT:-/home/user/.pyenv-bandit/bin/bandit}
  if [ ! -x "$py_bandit" ]; then
    echo "bandit de référence introuvable : $py_bandit (voir PLAN.md §2)" >&2
    return 1
  fi
  # Exporter la référence, et ne pas se contenter de la résoudre pour soi :
  # sans cela, chaque script enfant repartait de SA propre valeur par défaut.
  # Sur la machine de dév tous ces défauts existent, donc la porte passait ;
  # en CI ils n'existent pas, et le job « parity » échouait sur un chemin en
  # dur — porte verte, CI rouge, six runs de suite (cf. PLAN.md §6). La porte
  # emprunte désormais le même chemin de résolution que `.github/workflows/ci.yml`.
  export PY_BANDIT="$py_bandit"
  export PY_BANDIT_BIN="$(dirname "$py_bandit")"
  cargo build --release --locked || return 1
  python3 scripts/corpus.py fetch  --tier smoke || return 1
  python3 scripts/corpus.py verify --parse --tier smoke || return 1
  python3 scripts/diff_corpus.py   --tier smoke || return 1
  python3 scripts/cli_matrix.py    diff || return 1
}

if [ "$MODE" = "parity" ] || [ "$MODE" = "all" ]; then
  run "parité (corpus réel + matrice CLI vs bandit Python)" parity_job
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
