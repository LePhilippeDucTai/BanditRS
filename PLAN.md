# BanditRS — plan de réécriture de bandit en Rust (document de passation)

> Ce document permet de reprendre l'implémentation dans une nouvelle session. Lire dans l'ordre :
> 0. **`docs/plan/README.md`** — le plan de développement parallèle (TDD : port des 273 tests Python,
>    performance, 16 lots de travail pour sous-agents) qui remplace le §5 « prochaines étapes » ;
> 1. cette page (état, décisions, jalons, architecture, pièges) ;
> 2. `docs/spec/core.md` (sémantique exacte du cœur Python) ;
> 3. `docs/spec/plugins.md` (les 42 plugins + blacklists, messages/regex/défauts verbatim) ;
> 4. `docs/spec/cli_formatters_tests.md` (CLI, formatters, suite de tests = spécification d'acceptation) ;
> 5. `DEVIATIONS.md` (écarts délibérés).
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
- Corpus de test supplémentaire : `/usr/lib/python3.11` (stdlib).
- Fixtures : `examples/` (copie verbatim de `bandit/examples`, 96 fichiers ; 2 non-UTF-8 : `trojansource_latin1.py`
  latin-1, `nonsense2.py` binaire).

## 3. État d'avancement (ce qui est FAIT et VALIDÉ)

`cargo build --all-targets`, `cargo clippy --all-targets -- -D warnings` et `cargo fmt --check` propres.
`cargo test --all-targets` : **67 tests unitaires** (`src/`) + **274 tests d'intégration** (`tests/`, un fichier
miroir par fichier de test Python) dont **118 stubs `#[ignore]`** nommés comme les tests Python restant à porter
(`scripts/wp_status.sh`) ; les **223 tests actifs** sont au vert — la table complète des exemples upstream
(comptes de sévérité/confiance) correspond bit à bit à bandit Python.

Différentiel complet rejoué après la vague A1 (2026-09-08) : `examples/*.py` × {json, csv, xml, yaml, html,
txt, sarif} = **637 comparaisons, zéro diff inexpliqué** (les 34 diffs restants sont couverts par
DEVIATIONS.md #5 `tarfile_extractall`, #9 `mark_safe_*`, #11 ancres YAML ; champs volatils normalisés :
`generated_at`, `Run started:`, `endTimeUtc`, version de l'outil, URL de doc — cf. #12).

| Jalon (plan parallèle) | Contenu | État |
|---|---|---|
| J0 | Restructuration : squelette miroir de la suite Python, `docs/plan/` (inventaire, 16 fiches de lots, playbook, benchmarks), agents `.claude/agents/banditrs-wp-*`, skill `banditrs-dispatch`, `benches/e2e.rs`, `scripts/{wp_status,bench_vs_python}.sh`, CI | **Fait** (2026-09-08) |
| J1 | Suite Python 100 % portée (WP-01 → WP-13, 176 stubs) | **en cours** : vague A1 livrée (WP-02, 05, 07, 11, 12, 13 — 58 stubs activés, 176 → 118) ; reste la vague A2 (WP-01, 03, 04, 06, 08, 09, 10) |
| J2 | Corpus golden + différentiel en CI (WP-14, WP-16) | à lancer (vague B) |
| J3 | Benchmarks, tableau Python vs Rust, garde-fou (WP-15) | à lancer (vague B) |

| Jalon | Module(s) | État |
|---|---|---|
| M0 | `Cargo.toml`, `rust-toolchain.toml`, `src/lib.rs`, `src/log.rs`, `src/constants.rs`, `src/core/issue.rs`, `src/core/metrics.rs`, `src/pycompat/{datetime,fnmatch,splitlines,path,unicode_escape}.rs`, `src/core/utils.rs` | **Fait + tests** |
| M1 | `src/pycompat/encoding.rs` (PEP 263, BOM, latin-1, ascii, encoding_rs), `src/source/{file,parse}.rs` | **Fait + tests** |
| M2 | `src/ast/{mod,vnode,children,joined_str,positions,linerange,qualname,literal,walker,trace}.rs`, `src/core/context.rs`, `bandit --dump-walk`, `scripts/dump_walk.py` | **Fait + validé** : trace de parcours identique à bandit Python sur **94/94 exemples et 120/120 fichiers de la stdlib** (avec `BANDITRS_PYTHON_COMPAT=3.11`) |
| M3 | `src/core/blacklist.rs` (données + test B001), `src/core/registry.rs` (table des 42 plugins), `src/core/plugin_config.rs` (défauts + `from_config`), `src/core/nosec.rs`, `src/core/docs_utils.rs`, `src/core/test_set.rs` (`TestSet::new`), `src/core/tester.rs` (`Tester::run_tests`), `src/core/config.rs` (défauts + `get_option` ; **chargement fichier YAML/TOML et profils legacy restent stub**) | **Fait** (sauf chargement de fichier de config, voir M7) |
| M4 | `src/plugins/*.rs` — **42/42 plugins implémentés** (voir docs/spec/plugins.md) ; `django_mark_safe` (B703) a une limitation connue sur `DeepAssignation` (DEVIATIONS.md #9), sans impact sur la suite de base | **Fait** |
| M5 | `src/core/discover.rs::discover_files`, `src/core/scan.rs::scan_file` (`catch_unwind`, nosec depuis les tokens), `src/core/manager.rs::run_tests` (parallèle via `rayon`) | **Fait** ; `tests/common/mod.rs::check_example`/`check_metrics` implémentés, **78 tests fonctionnels au vert** |
| M6 | `src/formatters/*.rs` (csv/custom/html/json/sarif/screen/text/xml/yaml + `mod.rs::output_results`), `src/pycompat/{pyformat,csv,json,yaml_emit,html,xml,urlquote}.rs` | **Fait** ; **11 tests fonctionnels** (`tests/formatters.rs`, §C.7) au vert |
| M7 | `src/cli/{argparse,main}.rs` (parseur maison + flux §A.11), `BanditConfig::new` (YAML via `pycompat::yaml_load` + TOML via `toml`), `BanditConfig::profile()` (conversion nom→id, pas la conversion legacy `blacklist_calls`/`blacklist_imports`), `src/pycompat/{yaml_load,configparser}.rs` | **Fait** ; **9 tests fonctionnels** (`tests/runtime.rs`) au vert |
| M8 | `src/cli/{baseline,config_generator}.rs` | **Fait** ; testé manuellement (dépôt git jetable) et via différentiel contre Python — voir §5. `tests/baseline_functional.rs`/`tests/cli_tools.rs` restent des placeholders (§C.3/C.4/C.5/C.6 pas portés en tests automatisés) |
| M9 | Harnais différentiel exécuté ad hoc (voir §5) sur `examples/` (tous formats) : diffs restants tous expliqués par DEVIATIONS.md. `scripts/diff_against_python.sh` (script fichier) pas encore mis à jour/exécuté sur la stdlib | Essentiellement fait, script à finaliser |
| M10 | perf, README, clippy | **Fait** : `cargo clippy --all-targets -- -D warnings` propre, `cargo fmt --all`, benchmark (~70× sur stdlib), README à jour |

Chaque stub restant porte un commentaire de module. Chercher `todo!(` pour la liste exhaustive (restant : la
conversion legacy `blacklist_calls`/`blacklist_imports` de `convert_legacy_config`, cf. §5).

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
tests/              common (helpers), functional (table complète des comptes attendus), runtime, baseline_functional, cli_tools, formatters
scripts/            dump_walk.py (trace Python), diff_against_python.sh (harnais différentiel)
docs/spec/          core.md, plugins.md, cli_formatters_tests.md
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

## 5. Prochaines étapes

**Les prochaines étapes sont désormais décrites, lot par lot, dans `docs/plan/README.md`** (jalons J1–J3,
tableau de dispatch, propriété des fichiers, protocole de fusion) et `docs/plan/wp/WP-01…16`. Ce qui suit
reste comme rappel des limitations connues qui motivent ces lots (chacune est reprise dans une fiche) :

M0–M9 sont **faits** dans les grandes lignes (moteur de tests, 42 plugins, formatters, CLI complet — `bandit`,
`bandit-baseline`, `bandit-config-generator`). Limitations connues restantes (aucune ne bloque la suite de base) :

- DEVIATIONS.md #9 : `django_mark_safe`/`DeepAssignation` (limitation connue, sans impact sur la suite de base).
- `test_asserts`, `test_try_except_*`, `test_markupsafe_*`, `test_django_xss_*` (profil `exclude B308`) : hors de
  la suite fonctionnelle de base (`tests/functional.rs`), nécessiteraient un `BanditTestSet` construit avec un
  profil/config spécifique plutôt que `manager_for_default()` — pas encore portés en tests Rust.
- `BanditConfig::profile()` fait la conversion nom→id (`registry::get_test_id`) mais pas encore
  `convert_legacy_blacklist_data`/`convert_legacy_blacklist_tests` (l'ancien format `blacklist_calls`/
  `blacklist_imports` avec inversion `bad_calls`/`bad_imports` — DEVIATIONS.md note l'inversion à garder telle
  quelle) : `todo!()` implicite (pas de config de test l'exerçant pour l'instant). Tests de référence :
  `tests/unit/core/test_config.py`.
- Tests unitaires et fonctionnels restants : voir `docs/plan/test-inventory.md` (176 stubs `#[ignore]` dans
  `tests/functional_baseline.rs`, `tests/unit_cli_*.rs`, `tests/unit_core_*.rs`, `tests/unit_formatters_*.rs`) —
  §C.3 (WP-02), §C.4 (WP-03), §C.5 (WP-04), §C.6 (WP-05), §C.7 (WP-12/13), §C.8–C.16 (WP-06…11).

### Harnais différentiel — méthode et résultat (fait ad hoc, à refaire via `scripts/diff_against_python.sh`)

Venv de référence recréé (`python3 -m venv /home/user/.pyenv-bandit && .../pip install -e "/home/user/bandit[toml,yaml,sarif]"`,
+ `pip install sarif_om jschema-to-python`, absents de `bandit[sarif]` sur cet environnement). Comparaison
`bandit <file> -f <fmt>` (tous formats) Python vs `BANDITRS_PYTHON_COMPAT=3.11 bandit <file> -f <fmt>` Rust sur
les 96 fichiers de `examples/` (hors `nonsense2.py`, binaire) : **zéro diff** hors déviations documentées
(#5 adresses mémoire, #9 DeepAssignation, #10 pliage double-quoted, #11 ancres/alias YAML — voir DEVIATIONS.md).
Bugs réels trouvés et corrigés pendant cette passe :
- `registry::PLUGINS` était trié dans l'ordre `setup.cfg`, alors que `stevedore`/`importlib.metadata` charge les
  entry points **triés alphabétiquement par nom** (vérifié empiriquement sur `extension_loader.MANAGER.plugins`) —
  affecte l'ordre des issues d'un même nœud quand plusieurs plugins matchent (ex. B602/B607 sur un même `Call`).
- `sarif.rs` : `get_code` doit toujours utiliser `max_lines=3` (le formatter Python appelle `issue.as_dict()` sans
  argument, donc `-n/--number` n'a aucun effet sur le SARIF) ; `region.endLine` reproduit le bug Python
  `line_range[1]` (index 1 de la liste, pas la vraie dernière ligne) ; `region.snippet` reproduit l'indexation
  négative Python (`snippet_lines[idx]` avec `idx < 0` → depuis la fin) quand `lineno` du plugin diffère de
  `linerange[0]` ; `to_uri` doit normaliser via `PurePath.as_posix()` (`pycompat::path::posix_normalize`, retire
  un `./` initial) avant de percent-encoder.

À refaire avant de clore M9 : mettre à jour `scripts/diff_against_python.sh` pour automatiser cette comparaison
(actuellement faite via une boucle shell ad hoc) et l'exécuter aussi sur `/usr/lib/python3.11` (`BANDITRS_PYTHON_COMPAT=3.11`).

### M10 — `cargo clippy --all-targets -- -D warnings` (63 avertissements à nettoyer, aucun bloquant), `cargo fmt`,
benchmark (`time bandit -r /usr/lib/python3.11` Python vs Rust `--release`, objectif ≥ 20×), README, commit/push.

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
- clippy : 63 avertissements mineurs restants (`if` imbriqués, doc list indentation, lifetimes explicites) à
  nettoyer en M10 ; aucun n'est bloquant (`#[deny(clippy::approx_constant)]` est le seul deny actif, déjà propre).

## 7. Commandes utiles

```bash
cargo build --release && cargo test --all-targets         # 67 unitaires + 274 d'intégration (176 stubs ignorés)
scripts/wp_status.sh                                      # stubs restants par lot (docs/plan/README.md)
cargo bench --bench e2e                                   # benchs criterion ; scripts/bench_vs_python.sh pour Python vs Rust
BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit --dump-walk examples/nosec.py   # trace Rust
/home/user/.pyenv-bandit/bin/python scripts/dump_walk.py examples/nosec.py           # trace Python
scripts/diff_against_python.sh examples                   # harnais différentiel (script à finaliser, cf. §5)
BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit -r examples -f json   # équivalent Rust
/home/user/.pyenv-bandit/bin/bandit -r examples -f json   # référence Python (venv à recréer si absent, cf. §2)
```
