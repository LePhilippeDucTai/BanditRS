# Performance : objectifs, protocole, résultats

BanditRS doit être **mesurablement** plus rapide que bandit Python, et le rester. Ce document fixe ce
qu'on mesure, comment, et les seuils. Le lot [WP-15](wp/WP-15-benchmarks.md) l'exécute et remplit §5.

## 1. Ce qu'on mesure (et pourquoi)

Analogie : comparer deux voitures ne se résume pas à la vitesse de pointe ; il faut le 0–100 (latence
de démarrage), la consommation (mémoire) et l'autoroute (débit sur un gros corpus). Quatre métriques :

| Métrique | Définition | Pourquoi |
|---|---|---|
| **Débit corpus** | temps mur de `bandit -r <corpus> -f json -q` | le cas d'usage CI (stdlib 3.11 : 672 fichiers, 307 k lignes) |
| **Latence mono-fichier** | temps mur de `bandit <fichier> -f json -q` | le cas d'usage éditeur / pre-commit ; dominé par le démarrage (import Python vs binaire statique) |
| **Coût par ligne** | débit corpus ÷ lignes analysées | compare des corpus de tailles différentes |
| **Mémoire de pointe** | `Maximum resident set size` de `/usr/bin/time -v` | parallélisme rayon = plus de mémoire ; à surveiller |

Chaque mesure = médiane de N ≥ 5 exécutions, après 1 exécution de chauffe, sur une machine au repos,
binaire `--release` (LTO fat, `codegen-units = 1`, déjà configuré dans `Cargo.toml`).

## 2. Objectifs (seuils de J3)

| Cas | Objectif | Statut au 2026-09-08 |
|---|---|---|
| stdlib CPython 3.11 (`/usr/lib/python3.11`) | **≥ 20×** Python | ~70× mesuré ad hoc (`PLAN.md` M10), à re-mesurer avec le protocole |
| `examples/` (96 petits fichiers) | ≥ 15× | 17,7× (6,74 s → 0,38 s, 4 CPU) |
| un fichier moyen (`examples/subprocess_shell.py`) | ≥ 10× et < 10 ms | à mesurer |
| le plus gros fichier (`examples/long_set.py`, 65 KiB) | ≥ 10× | à mesurer |
| mémoire de pointe stdlib | ≤ Python | à mesurer |
| régression | aucun bench criterion > +10 % vs la baseline committée | garde-fou à créer (WP-15) |

Les objectifs sont volontairement en-deçà des mesures ad hoc : ils servent de **plancher garanti**, pas de
record. Les dépasser ne dispense pas d'écrire le résultat dans §5.

## 3. Outils

- `scripts/bench_vs_python.sh [-n RUNS] [-f FORMAT] [cibles…]` — compare la référence
  `/home/user/.pyenv-bandit/bin/bandit` et `target/release/bandit`, médiane de N runs, `hyperfine` si
  disponible (sinon boucle bash), écrit `target/bench/vs_python.md` (tableau à coller en §5).
- `cargo bench --bench e2e` — benchs criterion (`benches/e2e.rs`) : `scan_examples_dir`,
  `scan_subprocess_shell_py`, `parse_long_set_py`. Rapport HTML dans `target/criterion/`.
- Garde-fou (à créer, WP-15) : `scripts/bench_regression.sh` = `cargo bench -- --save-baseline <ref>`
  sur la branche d'intégration puis `cargo bench -- --baseline <ref>` sur la branche testée ; échec si
  un bench dépasse +10 % (lecture de `target/criterion/*/change/estimates.json`).
- Profil : `perf record -g target/release/bandit -r /usr/lib/python3.11 -q -f json` +
  `perf report`, ou `cargo flamegraph` si installé. Mémoire : `/usr/bin/time -v`.

## 4. Protocole d'une campagne (WP-15)

1. `cargo build --release` ; `nproc`, `uname -sr`, version des deux outils → en-tête du tableau.
2. `scripts/bench_vs_python.sh -n 7` (cibles par défaut : `examples/`, `/usr/lib/python3.11`).
3. Mono-fichier : `scripts/bench_vs_python.sh -n 15 examples/subprocess_shell.py examples/long_set.py`.
4. Mémoire : `/usr/bin/time -v <outil> -r /usr/lib/python3.11 -q -f json` pour chaque outil.
5. `cargo bench --bench e2e -- --save-baseline j3` ; committer un résumé (moyennes, pas le dossier `target/`).
6. Profil `perf` sur la stdlib ; noter les 5 premières fonctions dans §6.
7. Remplir §5 et §6 ; toute optimisation ensuite se fait **après** une mesure et se conclut par une
   re-mesure (« pas de mesure, pas d'optimisation »).

## 5. Résultats

_À remplir par WP-15 (tableau produit par `scripts/bench_vs_python.sh`, plus mémoire et mono-fichier)._

| Date | Machine | Cible | Python (s) | Rust (s) | Facteur | Mémoire Py / Rs (MiB) |
|---|---|---|---:|---:|---:|---|
| 2026-09-08 | 4 CPU, conteneur | `examples/` (96 fichiers, JSON) | 6,74 | 0,38 | 17,7× | — |

## 6. Pistes d'optimisation (à n'ouvrir qu'après mesure)

Ordonnées par rapport gain probable / risque de perte de parité :

1. **Allocateur** : `mimalloc` en allocateur global (souvent −10 à −30 % sur les charges riches en petites
   allocations). Une ligne, aucun risque de parité ; à mesurer.
2. **Lecture + décodage** : éviter la double copie texte/octets dans `source::SourceFile` (index de lignes
   calculé une fois, `snippet_line` paresseux).
3. **`SourceStore` global** : contention du verrou sous rayon ; retourner les sources par fichier
   (`FileOutcome`) plutôt que via un cache global.
4. **Regex des plugins** : vérifier qu'elles sont toutes `LazyLock`/`OnceLock` (jamais compilées par nœud).
5. **Walker** : `wants(kind)` court-circuite déjà la construction de contexte ; mesurer la part du calcul
   de `linerange`/`qualname` et la mettre en cache par nœud si > 10 %.
6. **Formatters** : sérialisation directe dans le `Write` plutôt qu'en `String` intermédiaire (JSON/SARIF
   sur la stdlib produisent plusieurs Mo).
7. **Démarrage** : la latence mono-fichier est bornée par l'exec ; vérifier qu'aucune table statique n'est
   construite inutilement au démarrage (`registry`, blacklists) — profil `perf stat`.

Interdits : toute optimisation qui change l'ordre des issues, les positions, ou le texte des sorties (la
parité prime), sauf à passer par une entrée `DEVIATIONS.md` validée par l'utilisateur.
