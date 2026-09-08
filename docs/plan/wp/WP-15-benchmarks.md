# WP-15 — Benchmarks, comparaison Python vs Rust, garde-fou de régression

**Agent** : `banditrs-wp-medium` · **Vague** B · **Fichiers** : `benches/e2e.rs`, `scripts/bench_vs_python.sh`, `scripts/bench_regression.sh` (nouveau), `docs/plan/benchmarks.md` (§5–6)
**Référence** : `docs/plan/benchmarks.md` (objectifs, protocole, outils)

## Objectif

Exécuter le protocole de `benchmarks.md` §4, remplir les résultats (§5) et le profil (§6), et livrer le
garde-fou de régression. **Aucune optimisation** n'est attendue de ce lot sans mesure avant/après ; une
optimisation qui toucherait un fichier gelé (`Cargo.toml` pour `mimalloc`, `src/source/**`…) est
**proposée** dans le rapport avec ses chiffres, pas appliquée.

## Livrables

1. `benches/e2e.rs` complété : bench `scan_stdlib` (ignoré proprement si `/usr/lib/python3.11` absent :
   `if !path.exists() { return; }`), bench par formatter (`format_json_examples`, `format_sarif_examples`
   sur un `Manager` pré-rempli), bench `walk_only` (parse + walker sans plugins, via un `TestRunner`
   vide) pour isoler le coût des plugins. Groupes criterion avec `sample_size` réduit pour la stdlib.
2. `scripts/bench_regression.sh [ref]` : `git stash`-free — utilise deux répertoires `target/criterion`
   baselines : `cargo bench --bench e2e -- --save-baseline <ref>` sur la branche d'intégration (ou lit
   une baseline existante), puis `cargo bench --bench e2e -- --baseline <ref>` ; parse
   `target/criterion/<bench>/change/estimates.json` (`mean.point_estimate`) et échoue si > +10 %.
   Documenté dans `benchmarks.md` §3.
3. Campagne complète (§4) sur cette machine : `scripts/bench_vs_python.sh -n 7`, mono-fichier
   (`-n 15 examples/subprocess_shell.py examples/long_set.py`), mémoire (`/usr/bin/time -v`), profil
   `perf` (top 5 fonctions) → tableaux dans `benchmarks.md` §5 et §6.
4. Vérification des objectifs §2 ; pour chaque objectif non atteint : analyse (profil) et proposition
   chiffrée dans le rapport.

## Critères d'acceptation

- `cargo bench --no-run` compile ; `cargo bench --bench e2e` s'exécute en < 5 min hors stdlib.
- `scripts/bench_regression.sh` : exécution de démonstration (baseline = HEAD, comparaison = HEAD) → 0 %.
- `benchmarks.md` §5 contient au minimum : examples, stdlib, subprocess_shell.py, long_set.py, mémoire
  Python/Rust, avec date/machine/versions ; §6 le top 5 du profil.
- Porte de qualité (les benches sont compilés par `--all-targets` : clippy doit passer dessus).

## Pièges

- Mesurer avec le binaire `--release` uniquement ; `perf` nécessite `debug = true` dans `[profile.release]`
  pour des symboles lisibles — ne pas committer ce changement (fichier gelé), l'appliquer localement.
- La stdlib contient des fichiers en erreur de syntaxe (Python 2) : ils comptent dans les deux outils.
- `hyperfine` n'est pas installé ici : le script a une boucle bash de secours ; noter l'outil utilisé.
