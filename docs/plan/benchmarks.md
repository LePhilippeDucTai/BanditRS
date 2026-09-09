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

Campagne WP-15 : `2026-09-09`, conteneur 4 CPU (`Linux 6.18.44-fc-v24`), `bandit 0.0.1.dev49`
(`/home/user/.pyenv-bandit`, Python 3.11.15) vs `target/release/bandit 0.1.0` (Rust, LTO fat,
`codegen-units = 1`). `hyperfine` n'est pas installé sur cette machine : `scripts/bench_vs_python.sh`
est retombé sur sa boucle bash de secours (médiane de N runs). Tableaux bruts :
`target/bench/vs_python.md` (§4.2) et `target/bench/vs_python_monofile.md` (§4.3, non committés —
`target/` est ignoré).

| Date | Machine | Cible | Python (s) | Rust (s) | Facteur | Objectif §2 |
|---|---|---|---:|---:|---:|---|
| 2026-09-09 | 4 CPU, conteneur | `examples/` (94 fichiers `*.py`, 9 534 lignes, JSON, 7 runs) | 6,464 | 0,340 | **19,0×** | ≥ 15× — atteint |
| 2026-09-09 | 4 CPU, conteneur | `/usr/lib/python3.11` (672 fichiers, 307 504 lignes, JSON, 7 runs) | 17,320 | 0,347 | **49,9×** | ≥ 20× — atteint |
| 2026-09-09 | 4 CPU, conteneur | `examples/subprocess_shell.py` (60 lignes, JSON, 15 runs) | 0,203 | 0,006 | **33,3×** | ≥ 10× et < 10 ms — atteint (6 ms) |
| 2026-09-09 | 4 CPU, conteneur | `examples/long_set.py` (65 KiB, 7 279 lignes, JSON, 15 runs) | 6,308 | 0,362 | **17,4×** | ≥ 10× — atteint |

Note : `examples/` compte 94 fichiers `*.py` avec le protocole (`find -name '*.py'`), contre 96 cités en
§2/PLAN.md — l'écart vient de deux fichiers `__init__.py` vides ; sans effet sur la mesure.

Mémoire de pointe (stdlib, `-r -q -f json /usr/lib/python3.11`) : `/usr/bin/time -v` n'est pas installé
sur cette machine (paquet absent) ; mesuré à la place avec `resource.getrusage(RUSAGE_CHILDREN).ru_maxrss`
(un processus Python dédié par mesure, pour éviter que la valeur cumulée d'un `RUSAGE_CHILDREN` compte le
maximum de plusieurs enfants) :

| Outil | RSS max (MiB) | Objectif §2 |
|---|---:|---|
| Python (`bandit`) | 75,4 | — |
| Rust (`target/release/bandit`) | 32,6 | ≤ Python — atteint (−57 %) |

Garde-fou de régression (`scripts/bench_regression.sh`) : démonstration `baseline = HEAD, comparaison =
HEAD` sur les 7 benchs de `benches/e2e.rs` — écarts entre −5,6 % et +9,5 % (bruit de mesure sur cette
machine partagée), aucun > +10 %, `OK: no bench regressed by more than +10%`.

Tous les objectifs de §2 sont atteints avec le protocole complet (à re-mesurer si la machine change).

## 6. Pistes d'optimisation (à n'ouvrir qu'après mesure)

`perf` n'est pas installé sur cette machine (paquet absent, pas de droit root pour l'ajouter) ; profil
pris avec `valgrind --tool=callgrind` (compte les instructions retirées `Ir`, pas des cycles temps réel,
mais les proportions relatives restent un bon guide) sur `target/release/bandit -r -q -f json
/usr/lib/python3.11`, puis `callgrind_annotate --auto=no`. Top 5 (coût propre, hors inlining du compilateur
release qui peut légèrement redistribuer les proportions) :

| # | Fonction | Part `Ir` |
|---|---|---:|
| 1 | `core::nosec::NosecLines::for_range` | 30,8 % |
| 2 | `plugins::trojansource::trojansource` | 20,4 % |
| 3 | `<&str as core::str::pattern::Pattern>::is_contained_in` | 6,0 % |
| 4 | `__memcmp_avx2_movbe` (libc) | 5,0 % |
| 5 | `core::scan::scan_file` | 4,0 % |

Les deux premiers postes (51 % du budget instructions) sont dans des fichiers hors propriété WP-15
(`src/core/nosec.rs`, `src/core/tester.rs`, `src/plugins/trojansource.rs`) — pistes chiffrées, **non
appliquées** ici :

1. **`NosecLines::for_range` (30,8 %, `core/nosec.rs` + `core/tester.rs`)** : `Tester::run_tests` (appelé
   pour chaque nœud visité, pas seulement les nœuds `Call`) fait `nosec.for_range(context.linerange)`, qui
   itère ligne par ligne (`range.iter().find_map(...)`) avec un hit `FxHashMap` par ligne. Quand
   `linerange` couvre un gros corps de fonction (héritée de l'ancêtre `FunctionDef`/`ClassDef` pour de
   nombreux types de nœuds, cf. `ast::linerange`), chaque nœud interne refait un balayage O(taille du
   corps) — comportement quadratique en profondeur d'imbrication × taille de fonction. Piste : ne calculer
   `for_range` qu'une fois par `(nœud, plage)` réellement nécessaire (mémoïser par `LineRange` déjà vue
   dans le nœud englobant), ou limiter la plage scannée au strict nécessaire de `utils.get_nosec` côté
   Python (une seule ligne dans l'immense majorité des cas réels). Gain estimé : ce poste passant de 31 %
   à un coût marginal libérerait potentiellement 25–30 % du temps CPU sur un corpus riche en fonctions
   longues (stdlib) — à confirmer par une mesure avant/après une fois implémenté par le lot propriétaire.
2. **`trojansource` (20,4 %, `plugins/trojansource.rs`)** : ce test `File` boucle sur les 10 caractères
   `BIDI_CHARACTERS` et, pour chacun, refait un `line.chars().position(...)` sur chaque ligne — 10 passes
   complètes du fichier par appel au lieu d'une. Piste : une seule passe par ligne testant l'appartenance
   du caractère courant à `BIDI_CHARACTERS` (`matches!` ou table de recherche), qui devrait diviser ce
   poste par ~10 sans changer le résultat (premier caractère bidi trouvé, mêmes ligne/colonne).
3. **Allocateur** : `mimalloc` en allocateur global (souvent −10 à −30 % sur les charges riches en petites
   allocations ; `_int_malloc`/`_int_free`/`malloc` de la libc pèsent ~3,3 % cumulés dans ce profil). Une
   ligne dans `Cargo.toml` (fichier gelé), aucun risque de parité ; à mesurer par le lot qui possède
   `Cargo.toml`.
4. **Lecture + décodage** : éviter la double copie texte/octets dans `source::SourceFile` (index de lignes
   calculé une fois, `snippet_line` paresseux).
5. **`SourceStore` global** : contention du verrou sous rayon ; retourner les sources par fichier
   (`FileOutcome`) plutôt que via un cache global.
6. **Regex des plugins** : vérifier qu'elles sont toutes `LazyLock`/`OnceLock` (jamais compilées par nœud).
7. **Walker** : `wants(kind)` court-circuite déjà la construction de contexte ; le profil ci-dessus montre
   que le coût dominant du walker n'est pas `linerange`/`qualname` eux-mêmes mais leur consommation par
   `for_range` (piste 1) — remesurer cette hypothèse séparément une fois la piste 1 traitée.
8. **Formatters** : sérialisation directe dans le `Write` plutôt qu'en `String` intermédiaire (JSON/SARIF
   sur la stdlib produisent plusieurs Mo). Coût mesuré isolément par `benches/e2e.rs`
   (`format_json_examples` ≈ 3,8 ms, `format_sarif_examples` ≈ 6,7 ms sur `examples/`, hors E/S disque) :
   modéré face aux deux postes ci-dessus, à rouvrir seulement après eux.
9. **Démarrage** : la latence mono-fichier est bornée par l'exec ; vérifier qu'aucune table statique n'est
   construite inutilement au démarrage (`registry`, blacklists) — profil `perf stat` (indisponible ici,
   à refaire sur une machine qui a `perf`).

Interdits : toute optimisation qui change l'ordre des issues, les positions, ou le texte des sorties (la
parité prime), sauf à passer par une entrée `DEVIATIONS.md` validée par l'utilisateur.
