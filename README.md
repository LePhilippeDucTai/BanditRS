# BanditRS

Réimplémentation en Rust pur de [bandit](https://github.com/PyCQA/bandit), l'analyseur statique de sécurité
pour code Python : mêmes tests (B1xx–B7xx, blacklists B3xx/B4xx), mêmes options de ligne de commande, mêmes
formats de sortie (json, yaml, csv, xml, html, sarif, txt, screen, custom), mêmes codes de sortie — mais
nettement plus rapide (parseur de [ruff](https://github.com/astral-sh/ruff), analyse parallèle par fichier :
~70× plus rapide que bandit Python sur un scan complet de la bibliothèque standard CPython 3.11).

**État : fonctionnel.** Les trois exécutables (`bandit`, `bandit-baseline`, `bandit-config-generator`) sont
implémentés et validés par différentiel contre bandit Python (voir `PLAN.md` §5) sur l'intégralité de
`examples/` (tous formats de sortie) et sur un scan complet de la bibliothèque standard Python 3.11 (1029
résultats, identiques bit à bit). Voir `PLAN.md` pour l'état d'avancement détaillé et l'architecture ;
`docs/spec/` pour la spécification de référence ; `DEVIATIONS.md` pour les écarts délibérés par rapport à
bandit Python.

```bash
rustup update stable          # rustc >= 1.96 requis
cargo build --release
cargo test --all-targets
cargo clippy --all-targets -- -D warnings

target/release/bandit -r mon_projet/                        # scanner un projet
target/release/bandit -r mon_projet/ -f json -o report.json # sortie JSON
target/release/bandit --help                                # toutes les options

target/release/bandit --dump-walk examples/nosec.py   # trace de parcours AST (outil de développement)
```

`bandit-config-generator` et `bandit-baseline` (nécessite `git` dans le `PATH`) fonctionnent comme leurs
homologues Python — voir `bandit-config-generator --help` / `bandit-baseline --help`.

**Développement.** La suite de tests de bandit Python (273 tests) est la spécification d'acceptation : chaque
test a un homonyme Rust dans `tests/` (un fichier miroir par fichier Python ; les tests pas encore portés sont
des stubs `#[ignore]`, comptés par `scripts/wp_status.sh`). Le plan de développement parallèle — lots de
travail pour sous-agents, jalons, benchmarks — est dans `docs/plan/README.md`.

```bash
scripts/wp_status.sh                 # progression du port de la suite de tests
cargo bench --bench e2e              # benchmarks criterion
scripts/bench_vs_python.sh           # comparaison Python vs Rust (référence : /home/user/.pyenv-bandit)
```

Licence Apache-2.0. Les fichiers de `examples/` et les spécifications de tests dérivent du projet bandit (PyCQA).
