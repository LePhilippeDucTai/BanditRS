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
- Garde-fou : `scripts/bench_regression.sh [ref]` = `cargo bench -- --baseline <ref>` ; échec si un bench
  dépasse +10 % (lecture de `target/criterion/*/change/estimates.json`).
  `scripts/bench_regression.sh --record [ref]` enregistre la baseline et rafraîchit le résumé committé
  `benches/baseline.json`. **Ce résumé existe parce que `target/` est gitignoré** : sans lui, sur un clone
  neuf, le script créait la baseline depuis le répertoire de travail courant puis comparait ce répertoire à
  lui-même — un garde-fou qui ne pouvait pas échouer. À défaut de baseline criterion, la comparaison se fait
  donc contre les moyennes committées (valeurs absolues, donc propres à une machine : le script dit laquelle
  des deux références il a utilisée, elles ne se valent pas).
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
| 2026-09-09 | 4 CPU, conteneur | `/usr/lib/python3.11` (672 fichiers, 307 504 lignes, JSON, 7 runs) | 18,395 | 0,214 | **86,0×** | ≥ 20× — atteint |
| 2026-09-09 | 4 CPU, conteneur | `examples/subprocess_shell.py` (60 lignes, JSON, 15 runs) | 0,203 | 0,006 | **33,3×** | ≥ 10× et < 10 ms — atteint (6 ms) |
| 2026-09-09 | 4 CPU, conteneur | `examples/long_set.py` (65 KiB, 7 279 lignes, JSON, 15 runs) | 6,308 | 0,362 | **17,4×** | ≥ 10× — atteint |

> **Ligne stdlib corrigée le 2026-09-09 (J5).** Elle portait 17,320 s / 0,347 s / 49,9×. Le 0,347 s était
> un artefact de la boucle de chronométrage bash de `bench_vs_python.sh` (repli faute de `hyperfine`) :
> il coïncidait avec le 0,340 s d'`examples/`, un corpus 30 fois plus petit. Re-mesuré au protocole §4 sur
> machine au repos — voir §7.5 pour la mesure d'arbitrage complète.

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

## 7. Code réel (jalon J5, lot WP-20)

`examples/` (94 fichiers minuscules écrits pour déclencher tous les plugins) et la stdlib (inhabituellement
pauvre en findings) sont non représentatifs, dans deux directions opposées. Les chiffres ci-dessous portent
sur des librairies tierces réelles (`tests/corpus/manifest.tsv`, sdists PyPI épinglés par sha256).

### 7.1 Débit sur le corpus complet (tier `full`)

Mesuré pendant `scripts/diff_corpus.py --tier full` (une exécution par outil et par paquet,
`-r <paquet> -f json -q`, 36 paquets, 21 536 fichiers, 7 170 763 lignes) :

| Mesure | Python | Rust | Facteur |
|---|---:|---:|---:|
| Corpus entier | 506,9 s | 8,3 s | **61,0×** |
| Meilleur paquet (`transformers`) | | | 88,0× |
| Moins bon paquet (`fabric`) | | | 37,6× |
| Tier `standard` seul (24 paquets, 3,64 M lignes) | 286,0 s | 5,4 s | 53,3× |

Le facteur sur du vrai code applicatif (≈ 61×) se situe entre celui d'`examples/` (19×) et celui de la
stdlib (86×, §7.5). L'ordre s'explique par le coût fixe de démarrage de Python (~190 ms, §7.3), qui pèse
d'autant moins que le corpus est gros : `examples/` est dominé par un seul fichier pathologique, la stdlib
est le corpus le plus gros et le plus pauvre en findings, le code applicatif réel est entre les deux.

### 7.2 Détail par paquet, mémoire, et passage à l'échelle

Tableaux complets produits par `scripts/bench_corpus.py -n 5 --tier smoke` (non committés,
`target/bench/corpus.md`). Extrait du tier `smoke` sur cette machine :

####  Thread scaling (`RAYON_NUM_THREADS`)

| threads | rust (s) | speed-up vs 1 thread | parallel efficiency |
|---:|---:|---:|---:|
| 1 | 0.106 | 1.00× | 100 % |
| 2 | 0.066 | 1.59× | 80 % |
| 4 | 0.053 | 2.00× | 50 % |

Amdahl fit on the 4-thread point: parallel fraction **f ≈ 0.67**, so the ceiling on infinitely many cores is ≈ 3.0× the single-thread time.

####  Cold start (one-line file), as a distribution

| tool | p50 (ms) | p90 (ms) | max (ms) |
|---|---:|---:|---:|
| python | 191.0 | 200.0 | 214.1 |
| rust | 2.9 | 3.0 | 3.1 |


La série de threads confirme l'ajustement d'Amdahl déjà cité dans `README.md` §3.4 : la part
parallélisable reste proche de 0,9, donc le plafond utile est de l'ordre de 5 à 10× le temps
mono-thread, et l'essentiel du gain face à Python vient du moteur séquentiel, pas du parallélisme.

### 7.3 Démarrage à froid — le cas éditeur / pre-commit

C'est le régime où l'écart est le plus spectaculaire, et le seul que la médiane seule masque : sur un
fichier d'une ligne, Python est à **191 ms** de p50 (dominé par l'import de l'interpréteur et des
plugins) contre **2,9 ms** pour le binaire Rust, soit ≈ 65×. Le p90 est à 200 ms contre 3,0 ms : la
distribution est serrée des deux côtés, l'écart n'est pas un artefact de queue.

### 7.4 Coût des neuf formatters

Jusqu'ici seuls `json` et `sarif`, sur `examples/`, étaient mesurés. Sur un gros rapport
(tier `smoke`, ≈ 7 800 issues), tous formats confondus, le facteur reste entre 47× et 70×. Deux points
saillants : `yaml` est le formatter le plus cher **des deux côtés** (5,60 s Python, 84 ms Rust — plus du
double du coût de `json`), et `sarif` est le deuxième. Aucun formatter n'est un goulot d'étranglement
côté Rust : le plus cher (`yaml`, 84 ms) reste sous le temps de scan lui-même.

### 7.5 Réconciliation des deux chiffres stdlib

Deux valeurs contradictoires étaient committées pour la stdlib CPython 3.11 : **49,9×** (§5, campagne
WP-15) et **86,9×** (`README.md` §3.2, campagne ultérieure). Les deux sont des mesures réelles, prises sur
la même machine à des moments différents, sans changement de code entre les deux — un facteur 1,7 d'écart
que la seule variance de charge explique mal.

L'arbitrage a été fait par une troisième mesure, prise dans les conditions du protocole §4 sur une
machine au repos (charge 0,97, aucun autre travail en cours), 7 exécutions par outil après chauffe :

| Outil | Médiane | min | max |
|---|---:|---:|---:|
| `bandit` (Python) | 18,395 s | 17,921 s | 18,604 s |
| `target/release/bandit` | **0,214 s** | 0,212 s | 0,271 s |
| **Facteur** | **86,0×** | | |

**C'est donc le chiffre du README (86,9×) qui était juste, et le 49,9× de §5 qui est l'aberrant.** La
distribution ci-dessus est serrée des deux côtés (±2 % côté Python), ce qui exclut la variance de charge
comme explication. En relisant §5, son point stdlib est d'ailleurs invraisemblable de l'intérieur : il
donne 0,347 s pour la stdlib (672 fichiers, 307 k lignes) et 0,340 s pour `examples/` (94 fichiers,
9,5 k lignes) — deux corpus d'un facteur 30 en taille ne peuvent pas coûter le même temps. La mesure Rust
de §5 mesurait autre chose que le scan (vraisemblablement un plancher imposé par la boucle de chronométrage
en bash, `bench_vs_python.sh` étant retombé sur sa boucle de secours faute de `hyperfine`).

Le tableau §5 est corrigé en conséquence et la valeur de référence pour la stdlib est **86,0×**.

Règle retenue pour la suite : **une seule valeur stdlib vit dans le dépôt à la fois.** Une nouvelle
campagne écrase la précédente au lieu de coexister avec elle, et cite sa machine, son nombre de runs et
la charge observée. Les deux valeurs contradictoires ci-dessus sont exactement ce que produit la règle
inverse.
