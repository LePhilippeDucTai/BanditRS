# BanditRS

Réimplémentation en Rust pur de [bandit](https://github.com/PyCQA/bandit), l'analyseur statique de sécurité
pour code Python : mêmes tests (B1xx–B7xx, blacklists B3xx/B4xx), mêmes options de ligne de commande, mêmes
formats de sortie (json, yaml, csv, xml, html, sarif, txt, screen, custom), mêmes codes de sortie — mais
nettement plus rapide (parseur de [ruff](https://github.com/astral-sh/ruff), analyse parallèle par fichier :
~70× plus rapide que bandit Python sur un scan complet de la bibliothèque standard CPython 3.11).

**État : terminé (v0.2.0).** Les trois exécutables (`bandit`, `bandit-baseline`, `bandit-config-generator`)
sont implémentés, testés et validés par différentiel contre bandit Python. Les 273 tests de la suite Python
ont un homologue Rust (350 tests au total : 67 unitaires + 283 d'intégration, dont le corpus golden), tous
actifs et au vert, zéro stub. La parité est prouvée à la fois par différentiel contre Python
(`scripts/diff_against_python.sh` : zéro diff inattendu sur `examples/` et sur la bibliothèque standard
Python 3.11, 672 fichiers) et, sans dépendre d'un interpréteur Python, par rejeu du corpus golden committé
(`cargo test --test golden`). Performance mesurée : ~19× plus rapide que bandit Python sur `examples/`, ~50×
sur un scan complet de la stdlib 3.11, avec garde-fou de régression (`scripts/bench_regression.sh`). Voir
`PLAN.md` pour l'état d'avancement détaillé et l'architecture ; `docs/spec/` pour la spécification de
référence ; `DEVIATIONS.md` pour les écarts délibérés par rapport à bandit Python.

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

**Développement.** La suite de tests de bandit Python (273 tests) a servi de spécification d'acceptation :
chaque test a un homonyme Rust dans `tests/` (un fichier miroir par fichier Python). `scripts/wp_status.sh`
vérifie qu'il ne reste aucun stub `#[ignore]`. Le plan de développement parallèle qui a mené à cet état — lots
de travail pour sous-agents, jalons, benchmarks — reste documenté dans `docs/plan/README.md` à titre
d'historique.

```bash
scripts/wp_status.sh                 # progression du port de la suite de tests
cargo bench --bench e2e              # benchmarks criterion
scripts/bench_vs_python.sh           # comparaison Python vs Rust (référence : /home/user/.pyenv-bandit)
```

Licence Apache-2.0. Les fichiers de `examples/` et les spécifications de tests dérivent du projet bandit (PyCQA).
