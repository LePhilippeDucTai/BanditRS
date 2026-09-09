# BanditRS — plan de réécriture de bandit en Rust (document de passation)

> **Projet terminé (v0.2.0, jalon J5, 2026-09-09).** Les six jalons (J0–J5) sont atteints. J4 clôturait le
> plan parallèle initial ; **J5 a été ouvert ensuite sur décision utilisateur** avec une question
> différente — non plus « la suite de tests passe-t-elle ? » mais « l'outil est-il substituable *sur du
> vrai code et sur toute sa surface d'options* ? » — et il est lui aussi atteint (§5). Ce document reste
> la référence pour reprendre le projet dans une nouvelle session (état, architecture, pièges, écarts).
> Lire dans l'ordre :
> 0. **`docs/plan/README.md`** — le plan de développement parallèle (TDD : port des 273 tests Python,
>    performance, code réel ; 21 lots de travail), conservé à titre d'historique du chantier ;
> 1. cette page (état, décisions, jalons, architecture, pièges) ;
> 2. `docs/spec/core.md` (sémantique exacte du cœur Python) ;
> 3. `docs/spec/plugins.md` (les 42 plugins + blacklists, messages/regex/défauts verbatim) ;
> 4. `docs/spec/cli_formatters_tests.md` (CLI, formatters, suite de tests = spécification d'acceptation) ;
> 5. `DEVIATIONS.md` (écarts délibérés) ;
> 6. `docs/drop-in-parity.md` (la preuve du remplacement drop-in, **générée** par `scripts/parity_report.py`).
> Le dépôt Python de référence est `/home/user/bandit` (@ `1d3053d`) ; lire le fichier Python
> correspondant **avant** de porter chaque module (les specs résument, le code Python fait foi).

## 1. Objectif et décisions validées avec l'utilisateur

- Réimplémentation **100 % Rust** de bandit (analyseur statique de sécurité pour code Python) : vrai redesign
  idiomatique et optimisé (pas de dicts dynamiques, pas d'état global mutable, parallélisme par fichier),
  **remplacement direct** (mêmes options, mêmes sorties, mêmes codes de sortie, mêmes résultats), et
  **couverture de toute la suite de tests** de bandit portée en Rust.
- Parseur : crates de ruff (`ruff_python_parser`/`ruff_python_ast`/`ruff_text_size`/`ruff_source_file` 0.0.12),
  rustc ≥ 1.96 (toolchain `stable` = 1.98.1 installée via `rustup update stable`).
- Fidélité : parité stricte sur tout ce que les tests observent ; **bugs connus corrigés** seulement quand aucun
  test n'en dépend (liste dans `DEVIATIONS.md`).
- Nommage : package `banditrs`, exécutables `bandit`, `bandit-baseline`, `bandit-config-generator`.
- Version affichée = version du crate ; URLs de doc `https://bandit.readthedocs.io/en/latest/...` (`DOCS_VERSION`).
- Politique de positions `PyCompat` (`src/ast/mod.rs`) : défaut = sémantique CPython ≥ 3.12 (plages réelles des
  constantes de f-strings) ; `BANDITRS_PYTHON_COMPAT=3.11` reproduit CPython 3.11 (utilisé par le harnais
  différentiel car la référence installée est Python 3.11.15).

## 2. Environnement

- Rust : `rustup` (stable 1.98.1), `cargo build`/`cargo test` fonctionnent ; crates.io accessible.
- Référence Python : venv `/home/user/.pyenv-bandit` avec `bandit` installé en editable depuis `/home/user/bandit`
  (`bandit 0.0.1.dev49`, extras toml/yaml/sarif). À recréer si absent :
  `python3 -m venv /home/user/.pyenv-bandit && /home/user/.pyenv-bandit/bin/pip install -e "/home/user/bandit[toml,yaml,sarif]"`.
- Corpus de test supplémentaire : `/usr/lib/python3.11` (stdlib) et, depuis J5, un corpus de code réel —
  38 sdists PyPI épinglés par sha256 dans `tests/corpus/manifest.tsv` (4 tiers : `smoke`, `standard`, `full`,
  `frontier`), téléchargés et vérifiés par `scripts/corpus.py fetch`, **jamais committés**.
- Fixtures : `examples/` (copie verbatim de `bandit/examples`, 96 fichiers ; 2 non-UTF-8 : `trojansource_latin1.py`
  latin-1, `nonsense2.py` binaire).

## 3. État d'avancement (ce qui est FAIT et VALIDÉ)

`cargo build --all-targets`, `cargo clippy --all-targets -- -D warnings` et `cargo fmt --check` propres.
`cargo test --all-targets` : **67 tests unitaires** (`src/`) + **287 tests d'intégration** (`tests/`, un fichier
miroir par fichier de test Python, plus `tests/golden.rs` (9 tests, WP-14) qui rejoue le corpus golden sans
Python et `tests/cli_matrix.rs` (3 tests, WP-18) qui rejoue les 88 invocations CLI enregistrées) = **354 tests,
tous actifs et au vert, zéro `#[ignore]`** (`scripts/wp_status.sh --check` → 0).
**Jalon J1 atteint le 2026-09-09** : les 273 tests de la suite Python
ont un homologue Rust homonyme (263 portés — 225 à l'identique, 38 adaptés — et 10 non portables, justifiés
dans `docs/plan/test-inventory.md` et `DEVIATIONS.md` #8). La table complète des exemples upstream
(comptes de sévérité/confiance) correspond bit à bit à bandit Python. **Jalons J2 et J3 également atteints le
2026-09-09** (vague B, WP-14 et WP-15 — cf. tableau ci-dessous) : parité prouvée sans Python installé (corpus
golden rejoué par `cargo test`) et performance mesurée avec garde-fou de régression. **Jalon J5 atteint le
2026-09-09** (vague C, WP-17 → WP-21) : parité prouvée sur du **code réel** (36/36 paquets PyPI) et sur la
**surface d'options complète** (88 invocations, stdout + stderr + code de sortie) — cf. §5.

Différentiel complet rejoué en fin de vague A2 (2026-09-09) via `scripts/diff_against_python.sh examples` :
traces de parcours **91/91 identiques**, et `examples/` × {json, txt, csv, xml, yaml, custom, sarif, html}
**sans aucun diff inexpliqué**. Les seuls écarts subsistants sont documentés : DEVIATIONS.md #5 (adresses
mémoire `<ast.List object at 0x…>` de `tarfile_extractall`), #10 (scalaire double-quoted non replié en YAML),
#11 (ancres/alias `&id001`/`*id001` de PyYAML, basées sur l'identité d'objet Python, non reproduites), plus
les champs volatils (`generated_at`, `Run started:`, version de l'outil et URL de doc — cf. #12) et la barre
de progression `Working…` que le bandit Python écrit sur la sortie. Depuis WP-01, `mark_safe_*` ne produit
plus de diff (`DeepAssignation` implémenté, DEVIATIONS.md #9 réécrite en conséquence). **Depuis J5, ce
paragraphe ne décrit plus que le plus ancien des quatre harnais** : la matrice CLI (WP-18) et le différentiel
sur code réel (WP-19) portent la preuve beaucoup plus loin, et ont résorbé #1 au passage — le `nosec` à ids
multiples n'est plus un écart, la regex Python est portée telle quelle.

**Correctif post-J4 (2026-09-09, réécriture du `README.md`).** L'audit de la surface CLI mené pour la
section « parité » du README a mis au jour un écart non documenté sur `bandit-baseline` : `--help`/`-h`
n'était pas géré (l'exigence `targets` s'appliquait d'abord, d'où « the following arguments are required »
et une sortie 2 au lieu de l'aide et d'une sortie 0), et les deux erreurs levées par le parseur lui-même
(cible manquante, `-f` hors `choices`) partaient dans le `LOG` (`[  ERROR ] …` sur stdout) au lieu de la
forme `argparse` (ligne `usage:` + `bandit-baseline: error: …` sur stderr). Aucun test de la suite Python
ne l'observe (`argparse` appartient à la bibliothèque standard), d'où l'angle mort. Corrigé dans
`src/cli/baseline.rs` (`USAGE`, `HELP`, `argparse_error()`, court-circuit `-h`/`--help` dans `main()`) ;
les quatre invocations sont désormais identiques octet pour octet à Python, code de sortie compris, et
verrouillées par `tests/unit_cli_baseline.rs::test_argparse_help_and_errors` — **le seul test Rust sans
homologue Python** de la suite. Les deux autres exécutables (`bandit`, `bandit-config-generator`) ont été
vérifiés au passage : `--help` correct, jeu d'options identique (27 options longues), exclusivité `-v`/`-q`
rejetée avec le même code 2.

| Jalon (plan parallèle) | Contenu | État |
|---|---|---|
| J0 | Restructuration : squelette miroir de la suite Python, `docs/plan/` (inventaire, 16 fiches de lots, playbook, benchmarks), agents `.claude/agents/banditrs-wp-*`, skill `banditrs-dispatch`, `benches/e2e.rs`, `scripts/{wp_status,bench_vs_python}.sh`, CI | **Fait** (2026-09-08) |
| J1 | Suite Python 100 % portée (WP-01 → WP-13, 176 stubs) | **Fait** (2026-09-09) : vague A1 (WP-02, 05, 07, 11, 12, 13) puis vague A2 (WP-01, 03, 04, 06, 09, 10, puis WP-08) — 176 stubs activés, 0 restant, `scripts/wp_status.sh --check` → 0 |
| J2 | Corpus golden + différentiel rejoué **en local** (WP-14 ; WP-16 suspendu, cf. ci-dessous) | **Fait** (2026-09-09) : corpus `tests/golden/**` committé (examples × 8 formats + 86 fixtures JSON) rejoué sans Python par `cargo test --test golden` (9 tests) ; `scripts/diff_against_python.sh` finalisé (mode fichier-par-fichier JSON autoritaire) et exécuté sur `examples/` (94/94 identiques) et la stdlib 3.11 (672/672 identiques), zéro diff inattendu |
| J3 | Benchmarks, tableau Python vs Rust, garde-fou (WP-15) | **Fait** (2026-09-09) : campagne complète dans `docs/plan/benchmarks.md` §5 (examples 19,0×, subprocess_shell.py 33,3×, long_set.py 17,4×, mémoire Rust −57 % ; la stdlib y figurait à 49,9×, chiffre **invalidé en J5** — il mesurait le plancher de la boucle de chronométrage, pas le scan ; la valeur retenue est **86,0×**, cf. `benchmarks.md` §7.5) — tous les objectifs §2 dépassés ; `scripts/bench_regression.sh` en place (garde-fou +10 %) ; profil `valgrind --callgrind` en §6 (2 pistes d'optimisation chiffrées non appliquées, hors propriété WP-15) |
| J4 | Consolidation : `PLAN.md` réécrit en état final, `README.md` à jour, version `0.2.0` | **Fait** (2026-09-09) : décision utilisateur — consolidation sans publication externe. Porte de qualité (`scripts/check.sh`) verte avant et après (351 tests, clippy 0 avertissement, fmt propre) ; version du crate passée à `0.2.0` (`Cargo.toml`/`Cargo.lock`) ; `README.md` reflète l'état final ; `docs/plan/README.md` §3 mis à jour. **J4 est le dernier jalon du plan parallèle** ; aucun J5 n'est défini — toute suite (publication, nouvelles fonctionnalités hors périmètre bandit) demande une nouvelle décision utilisateur et un nouveau plan. |
| J5 | Drop-in prouvé sur du code réel et sur toute la surface d'options (WP-17 → WP-21) | **Fait** (2026-09-09) : corpus de 38 sdists PyPI épinglés par sha256 (`tests/corpus/manifest.tsv`, 4 tiers) ; différentiel par paquet — **36/36 identiques** sur 21 536 fichiers, 7 170 763 lignes, 130 730 issues, `errors[]` et bloc `metrics` compris ; matrice CLI de 88 invocations comparant **stdout + stderr + code de sortie** — **85/88 identiques**, 3 écarts documentés (#10, #18, #19) et rejouables sans Python (`cargo test --test cli_matrix`) ; **6 divergences réelles trouvées et corrigées**, 3 écarts résorbés (#1, #14, #15) ; couverture 60/75 identifiants par le corpus + 15 par `examples/` = 0 non exercé ; benchmarks sur code réel (61,0×, démarrage à froid 191 ms → 2,9 ms) et baseline de régression committée (`benches/baseline.json`) ; rapport généré `docs/drop-in-parity.md` ; CI `parity` + `parity-nightly`. |

> **CI GitHub Actions : active** (`.github/workflows/ci.yml`). Elle avait été désactivée le 2026-09-09
> pour des raisons de coût, puis réactivée avec le wheel multi-plateforme ; le dépôt étant public, les
> runners standard sont gratuits. Six jobs : `test`, `lint`, `bench` (compilation seule), `python`
> (wheel), `parity` (matrice CLI + corpus `smoke` face au bandit de référence) et `parity-nightly`
> (tier `standard` + contrôle de dérive du corpus golden, hebdomadaire).
> `scripts/check.sh` reproduit les mêmes vérifications en local — à lancer avant chaque commit, et
> systématiquement avant un push.

| Jalon | Module(s) | État |
|---|---|---|
| M0 | `Cargo.toml`, `rust-toolchain.toml`, `src/lib.rs`, `src/log.rs`, `src/constants.rs`, `src/core/issue.rs`, `src/core/metrics.rs`, `src/pycompat/{datetime,fnmatch,splitlines,path,unicode_escape}.rs`, `src/core/utils.rs` | **Fait + tests** |
| M1 | `src/pycompat/encoding.rs` (PEP 263, BOM, latin-1, ascii, encoding_rs), `src/source/{file,parse}.rs` | **Fait + tests** |
| M2 | `src/ast/{mod,vnode,children,joined_str,positions,linerange,qualname,literal,walker,trace}.rs`, `src/core/context.rs`, `bandit --dump-walk`, `scripts/dump_walk.py` | **Fait + validé** : trace de parcours identique à bandit Python sur **94/94 exemples et 120/120 fichiers de la stdlib** (avec `BANDITRS_PYTHON_COMPAT=3.11`) |
| M3 | `src/core/blacklist.rs` (données + test B001), `src/core/registry.rs` (table des 42 plugins), `src/core/plugin_config.rs` (défauts + `from_config`), `src/core/nosec.rs`, `src/core/docs_utils.rs`, `src/core/test_set.rs` (`TestSet::new`), `src/core/tester.rs` (`Tester::run_tests`), `src/core/config.rs` (défauts + `get_option` ; chargement fichier et profils legacy complétés en M7/WP-08) | **Fait** |
| M4 | `src/plugins/*.rs` — **42/42 plugins implémentés** (voir docs/spec/plugins.md) ; `django_mark_safe` (B703) : `DeepAssignation` porté intégralement par WP-01 (try/with/for/while/ExceptHandler, déballage de tuple) — deux divergences résiduelles où BanditRS est plus strict que Python, documentées DEVIATIONS.md #9 | **Fait** |
| M5 | `src/core/discover.rs::discover_files`, `src/core/scan.rs::scan_file` (`catch_unwind`, nosec depuis les tokens), `src/core/manager.rs::run_tests` (parallèle via `rayon`) | **Fait** ; `tests/common/mod.rs::check_example`/`check_metrics` implémentés, **78 tests fonctionnels au vert** |
| M6 | `src/formatters/*.rs` (csv/custom/html/json/sarif/screen/text/xml/yaml + `mod.rs::output_results`), `src/pycompat/{pyformat,csv,json,yaml_emit,html,xml,urlquote}.rs` | **Fait** ; **11 tests fonctionnels** (`tests/formatters.rs`, §C.7) au vert |
| M7 | `src/cli/{argparse,main}.rs` (parseur maison + flux §A.11), `BanditConfig::new` (YAML via `pycompat::yaml_load` + TOML via `toml`), `BanditConfig::profile()` (conversion nom→id ; conversion legacy `blacklist_calls`/`blacklist_imports` ajoutée par WP-08), `src/pycompat/{yaml_load,configparser}.rs` | **Fait** ; **9 tests fonctionnels** (`tests/runtime.rs`) au vert |
| M8 | `src/cli/{baseline,config_generator}.rs` | **Fait** ; couvert par `tests/functional_baseline.rs` (7 scénarios `bandit -b`, WP-02), `tests/unit_cli_baseline.rs` (12 tests sur dépôts git temporaires, WP-04) et `tests/unit_cli_config_generator.rs` (WP-05), plus les cas `baseline_*`/`configgen_*` de la matrice CLI (WP-18) |
| M9 | Harnais différentiels | **Fait** ; quatre harnais complémentaires, cf. §5 : `diff_against_python.sh` (fichier par fichier — `examples/` 94/94, stdlib 672/672), `cargo test --test golden` (rejeu sans Python), `cli_matrix.py` (88 invocations, stdout + stderr + code de sortie), `diff_corpus.py` (36/36 paquets PyPI, 21 536 fichiers) |
| M10 | perf, README, clippy | **Fait** : `cargo clippy --all-targets -- -D warnings` propre, `cargo fmt --all`, benchmark (**86,0×** sur la stdlib, 61,0× sur le corpus réel), README à jour |

Plus aucun `todo!()` fonctionnel : la conversion legacy `blacklist_calls`/`blacklist_imports`
(`convert_legacy_config`), dernier stub du projet, a été implémentée par WP-08 (cf. §5).

## 4. Architecture (rappel) et invariants à respecter

```
src/
  lib.rs            ré-exports ; DOCS_VERSION, VERSION, AUTHOR
  log.rs            "[module]\tLEVEL\tmessage" sur stderr ; niveau global ; LogBuffer thread-local (with_buffer) pour la parallélisation
  constants.rs      Rank (UNDEFINED/LOW/MEDIUM/HIGH, poids 1/3/5/10), EXCLUDE, LOG_FORMAT_STRING
  pycompat/         ports exacts de comportements Python (datetime, fnmatch, splitlines, path, encoding, unicode_escape ; stubs : pyformat, csv, json, yaml_load, yaml_emit, configparser, html, xml, urlquote)
  source/           SourceFile (texte décodé, index de lignes, colonnes en OCTETS, snippet_line = linecache), SourceStore (cache global), parse (ruff)
  ast/              VNode (nœud virtuel Copy), NodeKind, children (ordre _fields CPython + siblings), JoinedStrView (constantes f-string fusionnées), positions (def/class décorés, elif, *args, générateur seul arg), linerange, qualname, literal (PyValue), walker (itératif), trace
  core/             issue, metrics, context (API Python 1:1), utils, blacklist, registry, plugin_config, nosec, docs_utils, config, test_set, tester, discover, scan, manager
  plugins/          un module par fichier Python (fonctions fn(&Context, &PluginConfigs) -> Result<Option<IssueDraft>, PyErr>)
  formatters/       csv custom html json sarif screen text xml yaml (+ Output, output_results, default_format)
  cli/              argparse (maison, compatible argparse), main, baseline, config_generator
  bin/              bandit (--dump-walk déjà branché), bandit_baseline, bandit_config_generator
tests/              common (helpers), un fichier miroir par fichier de test Python (functional, runtime, functional_baseline,
                    unit_cli_*, unit_core_*, unit_formatters_*), golden (rejeu du corpus golden), cli_matrix (rejeu de la surface CLI),
                    golden/** et cli_matrix/** (sorties de référence committées), corpus/ (manifeste épinglé, corpus jamais committé)
scripts/            dump_walk.py (trace Python), diff_against_python.sh + gen_golden.sh (golden), cli_matrix.py (surface CLI),
                    corpus.py + diff_corpus.py + parity_report.py (code réel), bench_*.sh/py, check.sh (porte de qualité), wp_status.sh
docs/spec/          core.md, plugins.md, cli_formatters_tests.md
docs/               drop-in-parity.md (rapport généré), plan/ (plan parallèle, inventaire, fiches de lots, benchmarks)
```

Invariants clés (tous validés par la trace de parcours) :
- Le walker reproduit `generic_visit` : pré-ordre, ordre des champs CPython, `_bandit_parent`
  (= `Context::parent()` / `ancestors`), `_bandit_sibling` (`Context::sibling`), Module jamais visité mais présent
  comme ancêtre racine, namespace push/pop pour `FunctionDef` sync et `ClassDef`, aliases/imports enregistrés
  **avant** les tests du nœud, `visit_Str` uniquement hors docstring avec `linerange` du parent.
- `TestRunner` (`src/ast/walker.rs`) : `wants(kind)` évite de construire un contexte quand aucun test n'existe ;
  le tester (M3) implémente ce trait. `TraceRunner` sert au harnais.
- Positions = CPython (`Pos { lineno, col_offset, end_lineno, end_col_offset }`, colonnes en octets UTF-8).
- `LineRange { start, end }` inclusif ; `LineRange::EMPTY` = `[]` Python ; `[0,1]` pour le contexte `File`.
- `PyValue` (`src/ast/literal.rs`) = `_get_literal_value` : bool/None → chaînes `"True"/"False"/"None"` ;
  `py_eq` = `==` Python (`0 == 0.0`) ; `truthy` ; `py_str`/`py_repr` pour les messages.
- `PyErr` : là où Python lèverait (KeyError de config, IndexError, TypeError…), renvoyer `Err(PyErr)` → le tester
  journalise `Bandit internal error running: <test> on file <f> at line <n>: <err>` et n'émet rien.

## 5. État final et clôture (jalon J5, 2026-09-09)

Le plan parallèle (`docs/plan/README.md`) est **clos** : J0 → J5 sont tous atteints. J4 fermait le plan
initial (portage de la suite Python, corpus golden, benchmarks) ; **J5 a été ouvert ensuite sur décision
utilisateur**, avec une question que les harnais existants ne posaient pas. Ils partageaient en effet un
angle mort : `examples/`, la stdlib et le code source de bandit sont tous du code *écrit pour tester bandit*
ou déjà passé sous ses yeux, et tous comparaient une seule chose — le rapport `-f json` sur stdout, depuis
l'invocation par défaut. J5 demande : **l'outil est-il substituable sur du vrai code et
sur toute sa surface d'options ?**

M0–M9 sont **faits** intégralement (moteur de tests, 42 plugins, formatters, CLI complet — `bandit`,
`bandit-baseline`, `bandit-config-generator`). Aucun `todo!()` ne subsiste. Tests unitaires et fonctionnels :
**tous portés** (J1, 0 stub — cf. §3 et `docs/plan/test-inventory.md`).

### Ce que J5 a établi (WP-17 → WP-21)

| Preuve | Résultat | Reproduire |
|---|---|---|
| Code réel (38 sdists PyPI épinglés par sha256, 4 tiers ; jamais committés) | **36/36 paquets identiques** — 21 536 fichiers, 7 170 763 lignes, 130 730 issues. « Identique » = mêmes issues dans le même ordre (fichier, ligne, plage de colonnes, sévérité, confiance, message), même `errors[]`, même bloc `metrics` (donc mêmes fichiers découverts, mêmes comptes de lignes) | `scripts/corpus.py fetch --tier full` puis `scripts/diff_corpus.py --tier full` |
| Surface CLI (88 invocations : seuils, sélection de tests, profils, 9 formatters, découverte de cibles, `.bandit`, erreurs d'usage, baselines, les 3 exécutables) comparée sur **stdout + stderr + code de sortie** | **85/88 identiques** ; 3 écarts documentés (#10 ancres YAML, #18 dump `-d`, #19 libellé d'erreur YAML) | `scripts/cli_matrix.py diff` (live) ou `cargo test --test cli_matrix` (rejeu, sans Python) |
| Couverture réelle des identifiants | 60/75 déclenchés par le corpus, les 15 restants (telnetlib, famille XML, SNMP…) par `examples/` → **0 identifiant non exercé** | `docs/drop-in-parity.md` §3, qui les nomme un par un |
| Performance sur code réel | **61,0×** sur l'ensemble du corpus (506,9 s → 8,3 s) ; démarrage à froid sur un fichier d'une ligne : 191 ms → 2,9 ms (p50) | `scripts/bench_corpus.py -n 5` ; `benchmarks.md` §7 |

Le rapport `docs/drop-in-parity.md` est **généré** par `scripts/parity_report.py` à partir du JSON du
différentiel — jamais édité à la main. Le régénérer depuis les mêmes entrées doit rendre le fichier
committé octet pour octet (vérifié en clôture de J5).

**Six divergences réelles trouvées par la matrice CLI, toutes corrigées.** La plus grave : `-ll`/`-lll`/
`-ii`/`-iii` filtraient **un niveau trop bas**, parce que l'`action="count"` d'argparse incrémente *à partir
de* `default=1` (donc `-l` vaut 2, `-ll` vaut 3, que `RANKING[n - 1]` mappe sur LOW puis MEDIUM) alors que le
portage remettait le compteur à 0 à la première occurrence. `bandit -ll` étant l'invocation de CI la plus
courante, l'outil remontait silencieusement le mauvais ensemble d'issues, et **rien d'autre dans ce dépôt ne
l'observait** : la suite Python ne teste pas son propre parseur d'arguments. Les cinq autres :
`parser.print_usage()` écrit sur stdout et non stderr ; les textes `USAGE`/`--help` étaient une
approximation (ce sont désormais ceux de la référence, octet pour octet, à la largeur 80 colonnes
qu'argparse retient hors terminal) ; `Unknown test found in profile` est journalisé par `extension_loader`,
pas par `main` ; l'erreur d'exclusion mutuelle nomme `-q/--quiet/--silent` ; trois lignes de journal
manquaient (avertissement blacklist legacy, avertissement ini illisible, et l'info « cibles en ligne de
commande »).

**Trois écarts résorbés plutôt que justifiés** — la matrice a montré qu'ils n'avaient pas lieu d'être :
`DEVIATIONS.md` #1 (jetons `nosec` : la regex `NOSEC_COMMENT_TESTS` de Python est désormais portée telle
quelle, ponctuation ignorée et dernière répétition seule retenue comprises), #14 (`_log_option_source`) et
#15 (`commit.name_rev` rend `<sha> <nom>`). Il reste **16 écarts délibérés** sur les 19 numéros du fichier,
qui est append-only.

**Quatre écarts nouveaux, documentés** (`DEVIATIONS.md` #16 → #19) : version cible du parseur (#16 — ce
n'est pas un défaut de parité mais un réglage : avec `BANDITRS_PYTHON_COMPAT=3.11` la parité est totale, y
compris sur les paquets qui exigent 3.12, `errors[]` compris ; le tier `frontier` chiffre les deux régimes),
ordre des identifiants dans un log (#17 — c'est Python qui n'est pas reproductible, il joint un `set`),
dump `-d` (#18), libellé d'erreur de PyYAML (#19).

**Piège à connaître avant de toucher aux profils** (découvert par WP-08) : `Profile::blacklist` est un
`Option<IndexMap<…>>` dont `TestSet::new` ne distingue que `Some` (« remplace la table intégrée ») et `None`
(« utilise la table intégrée filtrée »). Un `Some(map vide)` y est donc lu comme « remplace par rien » et
fait disparaître **silencieusement** tous les résultats de la blacklist intégrée pour un scan `-p <profil>`
dont la section `profiles` ne référence ni `blacklist_calls` ni `blacklist_imports` (reproduit : `-p test_4
examples/xml_sax.py`, 8 résultats côté Python contre 0 côté Rust). La conversion legacy produit donc `None`,
pas `Some(vide)`, quand elle n'a rien à ajouter — c'est le comportement de `if not blacklist:` en Python, où
« absent » et « vide » sont indiscernables. Toute évolution de ce type doit préserver cette équivalence.

### Reprendre après J5

Ce dépôt n'a plus d'étapes planifiées. Une reprise éventuelle (publication du crate, nouvelles
fonctionnalités hors périmètre de bandit Python, mise à jour vers une version plus récente de bandit comme
référence) demande une nouvelle décision utilisateur et, si le travail est substantiel, un nouveau document
de plan (le modèle `docs/plan/README.md` — jalons, lots à fichiers disjoints, porte de qualité — peut être
réutilisé tel quel).

Deux garde-fous tournent tout seuls et méritent d'être lus avant de conclure quoi que ce soit :
`scripts/bench_regression.sh` (échec si un bench dépasse la baseline de +10 % ; depuis WP-20 une baseline
committée `benches/baseline.json` prend le relais quand `target/` est vide — sans elle, le script comparait
le dépôt à lui-même et validait n'importe quel ralentissement) et le job CI `parity-nightly`, qui régénère
le corpus golden face à l'amont et échoue sur `git diff --exit-code tests/golden` : c'est le détecteur de
**dérive amont**, le seul harnais qui remarque que c'est *bandit* qui a changé, pas BanditRS.

### Les quatre harnais différentiels, et ce que chacun voit

1. `scripts/diff_against_python.sh` (M9, finalisé par WP-14) — fichier par fichier, JSON autoritaire :
   `examples/` **94/94**, stdlib 3.11 **672/672**, traces de parcours **91/91**, zéro diff inattendu.
2. `cargo test --test golden` (WP-14) — rejeu du corpus golden committé, **sans Python** : c'est ce qui
   transforme la parité d'observation ponctuelle en **invariant de régression**.
3. `scripts/cli_matrix.py` + `cargo test --test cli_matrix` (WP-18) — la seule preuve qui regarde stderr et
   le code de sortie, et la seule qui balaie les options plutôt que l'invocation par défaut.
4. `scripts/diff_corpus.py` (WP-19) — le seul qui regarde du code que personne n'a écrit pour bandit. Il
   inverse la forme du premier : comparer un rapport agrégé par paquet, et ne bissecter fichier par fichier
   qu'à l'intérieur d'un paquet qui a réellement divergé (à ~0,2 s de démarrage Python par fichier, forker
   les deux binaires 21 536 fois coûterait plus d'une heure en lancements seuls).

Venv de référence (à recréer si absent) : `python3 -m venv /home/user/.pyenv-bandit && .../pip install -e
"/home/user/bandit[toml,yaml,sarif]"`, plus `pip install sarif_om jschema-to-python` (absents de
`bandit[sarif]` sur cet environnement).

Bugs réels trouvés par le harnais fichier-par-fichier lors de sa première passe (M9), conservés ici parce
qu'ils disent où se cachent les écarts de ce genre :
- `registry::PLUGINS` était trié dans l'ordre `setup.cfg`, alors que `stevedore`/`importlib.metadata` charge les
  entry points **triés alphabétiquement par nom** (vérifié empiriquement sur `extension_loader.MANAGER.plugins`) —
  affecte l'ordre des issues d'un même nœud quand plusieurs plugins matchent (ex. B602/B607 sur un même `Call`).
- `sarif.rs` : `get_code` doit toujours utiliser `max_lines=3` (le formatter Python appelle `issue.as_dict()` sans
  argument, donc `-n/--number` n'a aucun effet sur le SARIF) ; `region.endLine` reproduit le bug Python
  `line_range[1]` (index 1 de la liste, pas la vraie dernière ligne) ; `region.snippet` reproduit l'indexation
  négative Python (`snippet_lines[idx]` avec `idx < 0` → depuis la fin) quand `lineno` du plugin diffère de
  `linerange[0]` ; `to_uri` doit normaliser via `PurePath.as_posix()` (`pycompat::path::posix_normalize`, retire
  un `./` initial) avant de percent-encoder.

### M10 — `cargo clippy --all-targets -- -D warnings` (0 avertissement), `cargo fmt`, benchmark (`time bandit -r
/usr/lib/python3.11` Python vs Rust `--release`, objectif ≥ 20× — dépassé : **86,0×**, cf. J3 et
`benchmarks.md` §7.5), README, commit/push : **fait**.


## 6. Pièges connus (déjà rencontrés ou anticipés)

- ruff `ExprCall` n'a pas de champ `range` : passer par `Ranged::range()` (`use ruff_text_size::Ranged`).
- Positions CPython ≠ ruff : def/class décorés (mot-clé après le dernier décorateur), `elif` (clause → `If`
  imbriqué), `*args`/`**kwargs` (sans les étoiles), générateur non parenthésé seul argument (inclut les parenthèses
  de l'appel), `lambda` sans paramètres (nœud `arguments` vide, sans position). Tout est dans `src/ast/positions.rs`
  et `children.rs` ; vérifier toute modification avec `scripts/dump_walk.py` vs `bandit --dump-walk`.
- f-strings : CPython fusionne les segments littéraux adjacents (y compris entre parties concaténées) et émet le
  texte de debug `x=` comme constante ; les format specs sont des `JoinedStr` imbriqués (`VNode::FormatSpec`).
- `visit_Str` : `linerange` du **parent** (un défaut de fonction a pour parent `arguments`, sans position →
  min/max des lignes de début des enfants, plus le contournement « sibling »).
- La porte nosec utilise `nosec_lines[draft.lineno]` **avant** le défaut de lineno, puis la **première** ligne
  non-`None` de `linerange` (pas d'union entre lignes).
- `Issue.end_col_offset` est toujours écrasé par le contexte ; `col_offset` ne l'est que si le plugin ne l'a pas fixé.
- Un fichier en erreur de syntaxe garde ses métriques `loc` (sans `SEVERITY.*`) et est retiré de `files_list`.
- `get_code` : `lmax` exclusif ; lignes `linecache` terminées par `\n` (newlines universelles) sauf `<stdin>` (brut).
- JSON : `sort_keys`, indent 2, `ensure_ascii` (échapper ≥ 0x7f), pas de newline final ; SARIF : ordre d'insertion.
- Ordre des issues d'un même nœud = ordre **alphabétique du nom d'entry-point** (`stevedore`/`importlib.metadata`,
  PAS l'ordre `setup.cfg` — vérifié empiriquement, cf. §5), B001 en dernier.
- YAML (`pycompat::yaml_emit`) : pliage à 80 colonnes vérifié octet à octet contre PyYAML 6.0.1 (algorithme de
  `write_plain`/`write_single_quoted` dans `emitter.py`, cf. commentaires du module) ; ancres/alias PyYAML basées
  sur l'identité d'objet Python non reproduites (DEVIATIONS.md #11) ; double-quoted non replié (#10).
- SARIF : `get_code` toujours avec `max_lines=3` (indépendant de `-n`) ; `region.endLine` = `line_range[1]`
  (bug Python, pas la vraie fin) ; indexation négative Python reproduite pour `region.snippet` ; `to_uri` via
  `PurePath.as_posix()` (`pycompat::path::posix_normalize`).
- clippy : `cargo clippy --all-targets -- -D warnings` propre, 0 avertissement (nettoyé en M10/J4).
- **argparse `action="count"` part de `default=1`, pas de 0** : `-l` vaut 2 et `-ll` vaut 3, que
  `RANKING[n - 1]` mappe sur LOW puis MEDIUM. Remettre le compteur à 0 à la première occurrence décale tous
  les seuils d'un cran (bug trouvé par la matrice CLI, J5). De façon générale, tout défaut d'option se lit
  dans `argparse`, pas dans l'intuition.
- `parser.print_usage()` écrit sur **stdout**, `parser.error()` sur stderr : comparer les deux flux
  séparément, jamais leur concaténation.
- Pièges du harnais de la matrice CLI : `subprocess(text=True)` réécrit les CRLF que le module `csv` de
  Python émet (comparer en binaire), et `bandit-baseline` **relance `bandit` depuis le PATH** — chaque outil
  a donc besoin de son propre binaire en tête de PATH, sinon on compare un outil à lui-même.
- Garde-fou de benchmark : `target/` est gitignoré, donc sur un dépôt fraîchement cloné criterion n'a
  **aucune baseline** ; il en crée une silencieusement à partir du worktree courant et compare le dépôt à
  lui-même — « aucune régression » quel que soit le ralentissement. D'où `benches/baseline.json`, committé
  (WP-20), utilisé en repli et rafraîchi par `scripts/bench_regression.sh --record`.
- Comparer à une référence, c'est **aligner la version d'interpréteur** : `BANDITRS_PYTHON_COMPAT` fixe la
  version cible du parseur `ruff`, pas seulement les positions de f-strings. Sans cet alignement, BanditRS
  analyse normalement des fichiers que CPython 3.11 range dans `errors[]` et remonte donc *plus* d'issues
  (DEVIATIONS #16). Ce n'est pas un défaut de parité, c'est un réglage — et c'est la première chose à
  vérifier devant un diff sur un paquet récent.

## 7. Commandes utiles

```bash
scripts/check.sh                                          # PORTE DE QUALITÉ LOCALE (remplace la CI) — à lancer avant tout push
scripts/check.sh fast                                     # idem sans la compilation des benchs (boucle de dév)
cargo build --release && cargo test --all-targets         # 354 tests (67 unitaires + 287 d'intégration), 0 ignoré
scripts/wp_status.sh                                      # stubs restants par lot (docs/plan/README.md)
cargo bench --bench e2e                                   # benchs criterion ; scripts/bench_vs_python.sh pour Python vs Rust
scripts/bench_regression.sh [ref]                         # garde-fou de régression (J3, WP-15) : échec si un bench > +10 % vs la baseline
cargo test --test golden                                  # rejeu du corpus golden (J2, WP-14), sans Python installé
BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit --dump-walk examples/nosec.py   # trace Rust
/home/user/.pyenv-bandit/bin/python scripts/dump_walk.py examples/nosec.py           # trace Python
scripts/diff_against_python.sh examples                   # harnais différentiel finalisé (J2, WP-14) ; --stdlib pour /usr/lib/python3.11
scripts/check.sh parity                                   # + différentiel face à la référence (réseau requis) : matrice CLI et tier « smoke »
cargo test --test cli_matrix                              # rejeu des 88 invocations CLI enregistrées (J5, WP-18), sans Python
scripts/cli_matrix.py diff                                # la même matrice en direct face au bandit Python
scripts/corpus.py fetch --tier standard                   # corpus de code réel épinglé par sha256 (~200 Mo, jamais committé)
scripts/corpus.py verify --parse                          # précondition : tout le corpus est analysable par l'interpréteur de référence
scripts/diff_corpus.py --tier full --json target/parity/full.json   # différentiel par paquet (J5, WP-19)
scripts/parity_report.py --parity target/parity/full.json target/parity/frontier.json   # régénère docs/drop-in-parity.md
scripts/bench_corpus.py -n 5                              # vitesse, mémoire, montée en threads, démarrage à froid (J5, WP-20)
BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit -r examples -f json   # équivalent Rust
/home/user/.pyenv-bandit/bin/bandit -r examples -f json   # référence Python (venv à recréer si absent, cf. §2)
```
