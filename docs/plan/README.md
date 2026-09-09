# BanditRS — plan de développement parallèle (TDD + performance)

> Document maître de la phase « industrialisation » de BanditRS. Il remplace la section « prochaines
> étapes » de `PLAN.md` (qui reste le document d'architecture et d'état) et se lit dans l'ordre :
> 1. ce fichier (objectifs, jalons, lots de travail, protocole d'exécution) ;
> 2. [`test-inventory.md`](test-inventory.md) — les 273 tests Python et leur miroir Rust, statut par statut ;
> 3. [`agent-playbook.md`](agent-playbook.md) — la procédure qu'un sous-agent suit pour livrer un lot ;
> 4. [`wp/WP-xx-*.md`](wp/) — une fiche par lot : fichiers possédés, tests à porter avec valeurs
>    attendues, critères d'acceptation ;
> 5. [`benchmarks.md`](benchmarks.md) — protocole et objectifs de performance.

## 0. Résumé

> **Clôture du plan (J5, 2026-09-09) : voir `PLAN.md` §5.** Le tableau ci-dessous a été mis à jour à la
> fin de la vague C (J5), qui a ajouté trois tests d'intégration (le rejeu de la matrice CLI) et corrigé
> six divergences réelles trouvées par cette matrice.

| | État au 2026-09-09 (fin de la vague C, jalon J5) |
|---|---|
| Moteur | 42 plugins, 9 formatters, 3 exécutables, parité bit à bit validée par différentiel (`PLAN.md` §3) |
| Suite Rust | 67 tests unitaires (`src/`) + 287 tests d'intégration (`tests/`, dont `tests/golden.rs` et `tests/cli_matrix.rs`) — **354 tests, tous actifs et verts, 0 stub** |
| Suite Python de référence | 273 tests (`/home/user/bandit` @ `1d3053d`), tous au vert (`pytest`, Python 3.11) |
| Couverture du port | **263 tests portés** (225 à l'identique, 38 adaptés), 0 restant, 10 non portables |
| Parité sans Python | Corpus golden committé (`tests/golden/**`, examples × 8 formats + 86 fixtures JSON) et surface CLI enregistrée (`tests/cli_matrix/**`, 88 invocations), rejoués par `cargo test --test golden` et `--test cli_matrix` ; `scripts/diff_against_python.sh` : 0 diff inattendu sur `examples/` (94/94) et la stdlib 3.11 (672/672) (WP-14, J2 ; WP-18, J5) |
| Parité sur code réel | **36/36 paquets identiques** sur 21 536 fichiers, 7 170 763 lignes, 130 730 issues (issues, `errors[]` et bloc `metrics`) ; **85/88 invocations CLI identiques** (stdout + stderr + code de sortie), 3 écarts documentés ; 60/75 identifiants déclenchés par le corpus, les 15 restants par `examples/` → 0 non exercé (WP-17→19, J5 — rapport `docs/drop-in-parity.md`) |
| Performance (4 CPU) | `examples/` : **19,0×** ; stdlib 3.11 (672 fichiers, 307 k lignes) : **86,0×** (chiffre arbitré par une 3ᵉ campagne, cf. `benchmarks.md` §7.5) ; corpus réel (21 536 fichiers) : **61,0×** ; démarrage à froid sur un fichier d'une ligne : 191 ms → 2,9 ms ; mémoire RSS Rust −57 % vs Python ; garde-fou de régression (`scripts/bench_regression.sh`, +10 %, baseline committée `benches/baseline.json`) (WP-15, J3 ; WP-20, J5 — détails `benchmarks.md` §5 et §7) |

Trois objectifs pour la phase qui s'ouvre, dans cet ordre de priorité :

1. **TDD complet** — chaque test de la suite Python a un test Rust homonyme au vert (jalon J1).
2. **Parité prouvée sans Python** — un corpus « golden » généré une fois avec bandit Python et rejoué par `cargo test`, sans Python installé (J2).
3. **Performance mesurée et gardée** — benchmarks reproductibles, tableau Python vs Rust, garde-fou de régression (J3).

Le tout exécuté **en parallèle par des sous-agents Sonnet 5** : 16 lots de travail (*work packages*, WP) à
fichiers disjoints, chacun confié à un agent avec un niveau d'effort adapté, fusionnés par la session
principale (l'orchestrateur) après une passe de vérification identique pour tous. Un quatrième objectif —
**drop-in prouvé sur du code réel et sur toute la surface d'options** (J5, lots WP-17 → WP-21) — a été
ajouté ensuite sur décision utilisateur et mené **en série** par l'orchestrateur, ce qui porte le plan à
21 lots au total (cf. §3 et la note qui suit le tableau des jalons).

## 1. Le modèle mental (analogie du chantier)

- La **suite de tests Python est le cahier des charges** : 273 exigences vérifiables. Nous ne ré-inventons
  pas les critères d'acceptation, nous les *traduisons*.
- Les **stubs `#[ignore]`** sont les cases à cocher du cahier des charges : un stub par exigence, portant
  exactement le nom Python. `scripts/wp_status.sh` compte les cases restantes ; `--check` échoue tant
  qu'il en reste une.
- Chaque **WP est un lot du chantier** avec ses propres pièces (fichiers) : deux lots ne travaillent jamais
  dans la même pièce, donc pas de conflit de fusion. Les pièces communes (`Cargo.toml`, `src/lib.rs`,
  `tests/common/mod.rs`…) sont **gelées** ou réservées à un seul lot.
- L'**orchestrateur est le maître d'œuvre** : il lance les lots, réceptionne chacun avec la même
  inspection (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --all-targets`, différentiel si
  la sortie change) et fusionne dans un ordre fixé.
- La **référence Python installée** (`/home/user/.pyenv-bandit`) est l'*étalon* : quand un doute subsiste
  sur un comportement, on l'exécute plutôt que de le deviner.

## 2. Règles d'or (valables pour tous les lots)

1. **Le code Python fait foi.** `docs/spec/*.md` résume ; `/home/user/bandit/bandit/**.py` décide. Tout
   écart volontaire va dans `DEVIATIONS.md` (entrée numérotée) — sinon c'est un bug.
2. **Un test Python = un test Rust homonyme**, dans le fichier miroir de `test-inventory.md`. On ne
   supprime jamais un stub ; on retire `#[ignore]`, on garde le nom. Un test jugé non portable est
   argumenté dans l'inventaire (et `DEVIATIONS.md` #8), jamais silencieusement omis.
3. **Rouge → vert → refactor.** Activer le stub, le voir échouer pour la bonne raison, implémenter,
   le voir passer. Les valeurs attendues sont recopiées **verbatim** depuis le test Python (chaînes,
   comptes, codes de sortie).
4. **Propriété exclusive des fichiers** (§5). Hors de ses fichiers, un lot ne modifie rien ; s'il a
   besoin d'une API qui appartient à un autre lot, il la demande dans son rapport (l'orchestrateur
   arbitre) ou utilise ce qui existe (`pub` déjà exposé).
5. **Aucune modification de `Cargo.toml`/`Cargo.lock`** : toutes les dépendances de développement
   nécessaires sont déjà déclarées (`criterion`, `roxmltree`, `scraper`, `tempfile`, `pretty_assertions`).
6. **Porte de qualité identique pour tous** (cf. `agent-playbook.md` §4) : `cargo fmt --all --check`,
   `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` — zéro échec, zéro nouveau
   `#[ignore]` (sauf stub d'un autre lot laissé intact).
7. **Différentiel obligatoire** dès qu'un lot touche une sortie observable (plugin, formatter, positions,
   config) : comparaison contre la référence Python sur les fixtures concernées, `BANDITRS_PYTHON_COMPAT=3.11`.
8. **Commits petits, préfixés `WP-xx:`**, sans nom de modèle ni d'outil d'IA dans les messages ou le code.
9. **Rapport de fin standardisé** (`agent-playbook.md` §6) : branche, SHA, stubs activés, API ajoutée,
   écarts, blocages. Sans rapport, pas de fusion.

## 3. Jalons

| Jalon | Contenu | Critère de sortie | Lots |
|---|---|---|---|
| **J0 — Restructuration** (fait) | Squelette miroir de la suite Python (176 stubs), inventaire, fiches de lots, agents, benchs, CI | `cargo test --all-targets` vert, `scripts/wp_status.sh` opérationnel | — |
| **J1 — Suite Python 100 % portée** ✅ **Fait (2026-09-09)** | Tous les stubs activés et verts ; `DeepAssignation` (WP-01) et conversion legacy `convert_legacy_config` (WP-08) implémentées | Atteint : `scripts/wp_status.sh --check` → 0 stub ; 341 tests verts (67 + 274) ; inventaire à jour ; différentiel à zéro diff inexpliqué | WP-01 → WP-13 |
| **J2 — Parité prouvée sans Python** ✅ **Fait (2026-09-09)** | Corpus golden (examples × 8 formats + 86 fixtures stdlib) committé, test Rust de rejeu, script de régénération | Atteint : `cargo test --test golden` rejoue le golden sans Python installé (9 tests) ; `scripts/diff_against_python.sh` finalisé et exécuté sur `examples/` (94/94) et la stdlib (672/672), zéro diff inattendu | WP-14 |
| **J3 — Performance mesurée et gardée** ✅ **Fait (2026-09-09)** | Benchs criterion complets, tableau Python vs Rust (examples, stdlib, gros fichier, mono-fichier), garde-fou de régression, profil et premières optimisations *mesurées* | Atteint : `docs/plan/benchmarks.md` §5 rempli, tous les objectifs §2 dépassés (examples 19,0×, stdlib 49,9×, mono-fichier 17,4-33,3×, mémoire −57 %) ; `scripts/bench_regression.sh` en place ; profil §6 (2 pistes chiffrées non appliquées, hors propriété WP-15). *Le 49,9× de la stdlib a été invalidé en J5 : il mesurait le plancher de la boucle de chronométrage, pas le scan ; la valeur retenue est 86,0× (`benchmarks.md` §7.5).* | WP-15 |
| **J4 — Consolidation** ✅ **Fait (2026-09-09)** | `PLAN.md` réécrit (état final), README, version `0.2.0`, éventuellement publication | Décision utilisateur : consolidation sans publication externe | orchestrateur |
| **J5 — Drop-in prouvé sur du code réel** ✅ **Fait (2026-09-09)** | Corpus de librairies tierces épinglé, matrice différentielle CLI (stdout + stderr + code de sortie), rapport de preuve, benchmarks sur code réel, CI de parité | Atteint : **36/36 paquets identiques** sur 21 536 fichiers et 7,17 M lignes (130 730 issues, `errors[]` et `metrics` compris) ; **85/88 invocations CLI identiques** (3 écarts documentés : #10, #18, #19) ; 60/75 identifiants déclenchés par le corpus + 15 par `examples/` = 0 non exercé ; 6 divergences réelles trouvées et corrigées, 3 écarts (#1, #14, #15) résorbés | WP-17 → WP-21 |

Ordre recommandé : **J1 d'abord et en priorité absolue** (c'est la demande « test-driven »), J2 et J3 peuvent
démarrer en parallèle de J1 car leurs fichiers sont disjoints, mais on les fusionne après J1 pour garder
un signal CI lisible.

> **J4 close le plan parallèle** (portage de la suite Python, corpus golden, benchmarks). **J5** a été
> ouvert ensuite sur décision utilisateur, avec une question différente : non plus « la suite de tests
> passe-t-elle ? » mais « l'outil est-il substituable **sur du vrai code et sur toute sa surface
> d'options** ? ». Il a été mené en série, pas en vague d'agents, d'où une fiche par livrable
> (`wp/WP-17…21`) plutôt qu'un dispatch parallèle. Une suite éventuelle demande une nouvelle décision.

## 4. Tableau de dispatch des lots

Effort = valeur du champ `effort` de l'agent (`.claude/agents/banditrs-wp-<effort>.md`, modèle Sonnet 5).
« Tests » = stubs à activer (+ tests partiels à renforcer). Les dépendances sont des *ordres de fusion*
préférés, pas des blocages : tous les lots partent de la même branche d'intégration.

| WP | Lot | Agent / effort | Tests | Fichiers `src/` possédés | Dépend de | Vague |
|---|---|---|---:|---|---|---|
| [WP-01](wp/WP-01-functional-config-profiles.md) | Tests fonctionnels à config/profil + `DeepAssignation` | `banditrs-wp-high` | 11 | `src/plugins/django_xss.rs` | — | A |
| [WP-02](wp/WP-02-functional-baseline.md) | Scénarios `bandit -b` (7) via le binaire | `banditrs-wp-medium` | 7 | — | — | A |
| [WP-03](wp/WP-03-unit-cli-main.md) | `cli/main` : ini, `_log_option_source`, codes de sortie | `banditrs-wp-high` | 19 | `src/cli/main.rs`, `src/cli/argparse.rs` | — | A |
| [WP-04](wp/WP-04-unit-cli-baseline.md) | `bandit-baseline` avec dépôts git temporaires | `banditrs-wp-high` | 12 | `src/cli/baseline.rs` | — | A |
| [WP-05](wp/WP-05-unit-cli-config-generator.md) | `bandit-config-generator` | `banditrs-wp-low` | 6 | `src/cli/config_generator.rs`, `src/core/plugin_config.rs` | — | A |
| [WP-06](wp/WP-06-unit-core-manager.md) | `Manager` : découverte, baseline, sorties | `banditrs-wp-high` | 20 | `src/core/manager.rs`, `src/core/discover.rs` | — | A |
| [WP-07](wp/WP-07-unit-core-util.md) | `utils` : qualname, call name, linerange, ini | `banditrs-wp-medium` | 26 | `src/core/utils.rs`, `src/pycompat/configparser.rs` | — | A |
| [WP-08](wp/WP-08-unit-core-config.md) | `BanditConfig` YAML/TOML + conversion legacy | `banditrs-wp-high` | 26 | `src/core/config.rs` | — | A |
| [WP-09](wp/WP-09-unit-core-context.md) | `Context` sur de vrais extraits (helper de parcours) | `banditrs-wp-high` | 17 | `src/core/context.rs`, `src/ast/walker.rs` | — | A |
| [WP-10](wp/WP-10-unit-core-test-set.md) | `TestSet` / blacklists avec le registre réel | `banditrs-wp-high` | 13 | `src/core/test_set.rs`, `src/core/blacklist.rs`, `src/core/registry.rs` | — | A |
| [WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md) | `Issue`, `report_issue`, `docs_utils` | `banditrs-wp-low` | 12 | `src/core/issue.rs`, `src/core/docs_utils.rs` | — | A |
| [WP-12](wp/WP-12-unit-formatters-text-screen.md) | Formatters `text`/`screen` (chaînes exactes, baseline) | `banditrs-wp-medium` | 6 (+1 partiel) | `src/formatters/text.rs`, `src/formatters/screen.rs` | — | A |
| [WP-13](wp/WP-13-unit-formatters-structured.md) | Formatters csv/custom/html/json/sarif/xml/yaml | `banditrs-wp-medium` | 1 (+4 partiels) | `src/formatters/{csv,custom,html,json,sarif,xml,yaml,mod}.rs` | — | A |
| [WP-14](wp/WP-14-golden-differential.md) | Corpus golden + test de rejeu + harnais différentiel finalisé | `banditrs-wp-medium` | nouveaux | `scripts/`, `tests/golden*` | fusion après WP-01 | B |
| [WP-15](wp/WP-15-benchmarks.md) | Benchmarks, tableau Python vs Rust, garde-fou | `banditrs-wp-medium` | nouveaux | `benches/`, `scripts/bench_*` | — | B |
| ~~[WP-16](wp/WP-16-ci.md)~~ | ~~CI GitHub Actions~~ — **suspendu le 2026-09-09** (décision utilisateur : aucun coût, validation locale uniquement via `scripts/check.sh`) | — | — | `.github/` | — | — |
| [WP-17](wp/WP-17-corpus.md) | Corpus de code réel épinglé par sha256 (4 tiers) | orchestrateur | nouveaux | `scripts/corpus.py`, `tests/corpus/**` | — | C |
| [WP-18](wp/WP-18-cli-matrix.md) | Matrice différentielle CLI : stdout, stderr, code de sortie (88 cas) | orchestrateur | nouveaux | `scripts/cli_matrix.py`, `tests/cli_matrix**` | — | C |
| [WP-19](wp/WP-19-parity-report.md) | Différentiel corpus + rapport de preuve | orchestrateur | nouveaux | `scripts/diff_corpus.py`, `scripts/parity_report.py`, `docs/drop-in-parity.md` | WP-17, WP-18 | C |
| [WP-20](wp/WP-20-bench-corpus.md) | Benchmarks sur code réel, mémoire, threads, démarrage à froid | orchestrateur | nouveaux | `scripts/bench_corpus.py`, `benches/e2e.rs`, `benchmarks.md` §7 | WP-17 | C |
| [WP-21](wp/WP-21-integration.md) | Porte de qualité, CI de parité, documentation | orchestrateur | — | `scripts/check.sh`, `.github/**`, docs | WP-17…20 | C |

Répartition des efforts : 7 lots `high` (refactors de testabilité ou sémantique à compléter), 6 `medium`
(ports mécaniques mais volumineux ou nécessitant un parseur), 3 `low` (ports directs). Si le nombre
d'agents simultanés est limité, lancer la vague A en deux salves : **A1** = WP-02, 05, 07, 11, 12, 13 (rapides,
donnent vite des fusions) puis **A2** = WP-01, 03, 04, 06, 08, 09, 10. Les cinq lots de la vague C
(WP-17 → WP-21, jalon J5) n'ont pas d'effort d'agent : ils ont été menés **en série par l'orchestrateur**,
parce qu'ils se lisent les uns les autres (le rapport dépend du corpus et de la matrice) et qu'ils
touchent au même jeu de scripts.

## 5. Propriété des fichiers

Principe : un fichier a **un seul** lot propriétaire ; les autres lots le lisent mais ne l'éditent pas.

| Fichier(s) | Propriétaire | Remarque |
|---|---|---|
| `tests/<miroir>.rs` | le WP de l'inventaire | un fichier de test par lot, jamais partagé |
| `tests/common/mod.rs` | WP-01 | helpers fonctionnels (`check_example`, `manager_for_default`…) |
| `tests/common/formatters.rs` | WP-13 | fixture commune des formatters ; WP-12 met ses helpers dans ses propres fichiers |
| `src/core/config.rs` | WP-08 | les autres lots passent par `BanditConfig::default()` + mutation de `raw` |
| `src/core/plugin_config.rs` | WP-05 | WP-01 lit seulement (`from_config`) |
| `src/core/manager.rs`, `src/core/discover.rs` | WP-06 | |
| `src/core/context.rs`, `src/ast/**` | WP-09 | helper de test `with_contexts` |
| `src/core/test_set.rs`, `src/core/blacklist.rs`, `src/core/registry.rs` | WP-10 | WP-08 construit des `BlacklistEntry` par littéral de struct (champs `pub`), sans toucher au fichier |
| `src/core/issue.rs`, `src/core/docs_utils.rs`, `src/core/metrics.rs` | WP-11 | |
| `src/core/utils.rs`, `src/pycompat/configparser.rs` | WP-07 | |
| `src/formatters/text.rs`, `src/formatters/screen.rs` | WP-12 | |
| `src/formatters/{csv,custom,html,json,sarif,xml,yaml,mod}.rs`, `src/pycompat/{csv,html,xml,yaml_emit,json,pyformat,urlquote}.rs` | WP-13 | |
| `src/cli/main.rs`, `src/cli/argparse.rs` | WP-03 | |
| `src/cli/baseline.rs` | WP-04 | |
| `src/cli/config_generator.rs` | WP-05 | |
| `src/plugins/django_xss.rs` | WP-01 | `DeepAssignation` complet (DEVIATIONS #9) |
| `src/plugins/*` (autres) | **gelé** | un bug de plugin découvert par un lot → rapport à l'orchestrateur |
| `benches/**`, `scripts/bench_*` | WP-15 | |
| `scripts/diff_against_python.sh`, `scripts/gen_golden.sh`, `tests/golden/**`, `tests/golden.rs` | WP-14 | |
| `.github/**` | WP-16 | |
| `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/log.rs`, `src/constants.rs`, `src/source/**`, `src/pycompat/*` (hors ci-dessus), `rust-toolchain.toml` | **gelés** | modification par l'orchestrateur seulement |
| `scripts/corpus.py`, `tests/corpus/**` | WP-17 | manifeste épinglé par sha256 ; le corpus lui-même n'est jamais committé |
| `scripts/cli_matrix.py`, `tests/cli_matrix/**`, `tests/cli_matrix.rs` | WP-18 | golden rejouable sans Python |
| `scripts/diff_corpus.py`, `scripts/parity_report.py`, `docs/drop-in-parity.md` | WP-19 | le rapport est **généré**, jamais édité à la main |
| `scripts/bench_corpus.py` | WP-20 | `benches/**` et `scripts/bench_*` restent à WP-15, rouverts pour J5 |
| `docs/plan/test-inventory.md` | chaque lot, **ses lignes uniquement** (colonne Statut → « porté ») | conflits triviaux, résolus par l'orchestrateur |
| `DEVIATIONS.md` | append-only, numéro suivant | |
| `PLAN.md`, `docs/plan/README.md` | orchestrateur | |

> **Validation : CI active + porte locale.** La CI a été désactivée le 2026-09-09 puis réactivée avec le
> wheel multi-plateforme ; elle vit dans `.github/workflows/ci.yml` (dépôt public, runners standard
> gratuits). `scripts/check.sh` exécute les mêmes vérifications sur la machine de dév, et
> `scripts/check.sh parity` y ajoute le différentiel face au bandit Python de référence.

## 6. Protocole d'exécution (orchestrateur)

Le skill [`banditrs-dispatch`](../../.claude/skills/banditrs-dispatch/SKILL.md) automatise ces étapes.

1. **Point de départ** : branche d'intégration à jour et verte (`cargo test --all-targets`), `git status` propre.
2. **Lancement d'une vague** : pour chaque lot, un appel `Agent` dans le *même* message (exécution
   concurrente), avec `subagent_type` = `banditrs-wp-<effort>` (tableau §4), `isolation: "worktree"` et
   le prompt de `agent-playbook.md` §7 (« Implémente le lot WP-xx… »). L'agent crée sa branche
   `wp/WP-xx-<slug>` dans son worktree.
3. **Réception** : à chaque rapport, l'orchestrateur vérifie que le rapport est complet, puis fusionne
   dans l'ordre **low → medium → high** (les petits lots d'abord : ils réduisent la surface des conflits
   suivants) : `git merge --no-ff wp/WP-xx-<slug>`, puis la porte de qualité (§2.6) et
   `scripts/wp_status.sh`. Un lot rouge après fusion est **renvoyé** à son agent (`SendMessage`) avec
   la sortie de la commande en échec ; l'orchestrateur ne corrige pas lui-même sauf conflit trivial.
4. **Fin de vague** : différentiel complet (`scripts/diff_against_python.sh examples`), mise à jour de
   `PLAN.md` §3 (tableau des jalons) et de la synthèse de `test-inventory.md`, commit, push.
5. **Vague suivante** ou jalon J4.

Garde-fous : jamais de `--force` ; jamais de fusion avec un test rouge « à corriger plus tard » ; un
stub désactivé par un lot qui n'en est pas propriétaire est refusé.

## 7. Définition de « terminé »

**Pour un lot** : tous ses stubs activés et verts ; ses tests partiels renforcés ; porte de qualité
verte ; différentiel sans nouveau diff ; inventaire (ses lignes) et `DEVIATIONS.md` (si besoin) à jour ;
rapport de fin livré ; aucun fichier hors propriété modifié.

**Pour J1** : `scripts/wp_status.sh --check` retourne 0 ; `cargo test --all-targets` ≥ 341 tests verts
(67 unitaires + 274 d'intégration, tous actifs) ; les 10 tests non portables sont listés dans l'inventaire
avec leur justification ; `PLAN.md` §3 marque J1.

## 8. Risques et parades

| Risque | Parade |
|---|---|
| Deux lots ont besoin de la même API `pub` | Les fiches pré-affectent les API (ex. `Manager` reste à WP-06 ; WP-12 n'a besoin que de champs déjà `pub`). Sinon : rapport → l'orchestrateur ajoute l'API sur la branche d'intégration et relance. |
| Un test « adapté » perd le sens du test Python | Chaque fiche explicite l'adaptation (mock → fixture réelle) et l'assertion équivalente ; l'agent cite la ligne Python dans le doc-commentaire du test Rust. |
| Régression de parité invisible par les tests unitaires | Règle 7 (différentiel) + WP-14 (golden rejoué par `cargo test`, donc couvert par `scripts/check.sh`). |
| Un lot « high » n'aboutit pas dans la session | Les commits partiels restent sur sa branche ; la fiche sert de reprise ; l'orchestrateur relance un agent avec `SendMessage` ou un nouvel agent sur la même branche. |
| Dérive de `PLAN.md` | Seul l'orchestrateur l'édite, à chaque fin de vague. |
| Benchmarks non reproductibles d'une machine à l'autre | Pas de mesure automatisée : `scripts/check.sh` compile les benchs seulement ; mesures locales committées dans `benchmarks.md` avec la machine et la date ; garde-fou criterion en local (`scripts/bench_regression.sh`). |

## 9. Fichiers de ce plan

```
docs/plan/
  README.md            ce document
  test-inventory.md    273 tests Python → miroir Rust, statut, lot (généré, puis maintenu à la main)
  agent-playbook.md    procédure d'un sous-agent + prompt de lancement
  benchmarks.md        objectifs, protocole, résultats de performance
  wp/WP-01 … WP-16     fiches de lots
.claude/agents/banditrs-wp-{low,medium,high}.md   agents Sonnet 5 par niveau d'effort
.claude/skills/banditrs-dispatch/SKILL.md          skill d'orchestration (lancer une vague, fusionner)
scripts/wp_status.sh                               tableau de bord des stubs (--check pour la CI)
scripts/bench_vs_python.sh                         comparaison Python vs Rust (median de N runs)
benches/e2e.rs                                     benchs criterion (squelette)
.github/workflows/ci.yml                           CI : test / lint / benchs compilent
```
