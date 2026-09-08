# BanditRS

Réimplémentation en Rust pur de [bandit](https://github.com/PyCQA/bandit), l'analyseur statique de sécurité
pour code Python : mêmes tests (B1xx–B7xx, blacklists B3xx/B4xx), mêmes options de ligne de commande, mêmes
formats de sortie (json, yaml, csv, xml, html, sarif, txt, screen, custom), mêmes codes de sortie — mais
nettement plus rapide (parseur de [ruff](https://github.com/astral-sh/ruff), analyse parallèle par fichier).

**État : en cours de développement.** Voir `PLAN.md` pour l'état d'avancement, l'architecture et les prochaines
étapes ; `docs/spec/` pour la spécification de référence ; `DEVIATIONS.md` pour les écarts délibérés.

```bash
rustup update stable          # rustc >= 1.96 requis
cargo build --release
cargo test
target/release/bandit --dump-walk examples/nosec.py   # trace de parcours AST (outil de développement)
```

Licence Apache-2.0. Les fichiers de `examples/` et les spécifications de tests dérivent du projet bandit (PyCQA).
