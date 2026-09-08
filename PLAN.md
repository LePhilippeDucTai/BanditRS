# BanditRS — plan de réécriture de bandit en Rust (document de passation)

> Ce document permet de reprendre l'implémentation dans une nouvelle session. Lire dans l'ordre :
> 1. cette page (état, décisions, jalons, prochaines étapes) ;
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

`cargo build --all-targets` propre (0 warning bloquant ; une dizaine d'avertissements clippy mineurs, cf. §6,
nettoyage prévu en M10), `cargo test` : 39 tests unitaires + **78 tests fonctionnels** (`tests/functional.rs`,
plus de `#[ignore]`) tous au vert — la table complète des exemples upstream (comptes de sévérité/confiance)
correspond bit à bit à bandit Python.

| Jalon | Module(s) | État |
|---|---|---|
| M0 | `Cargo.toml`, `rust-toolchain.toml`, `src/lib.rs`, `src/log.rs`, `src/constants.rs`, `src/core/issue.rs`, `src/core/metrics.rs`, `src/pycompat/{datetime,fnmatch,splitlines,path,unicode_escape}.rs`, `src/core/utils.rs` | **Fait + tests** |
| M1 | `src/pycompat/encoding.rs` (PEP 263, BOM, latin-1, ascii, encoding_rs), `src/source/{file,parse}.rs` | **Fait + tests** |
| M2 | `src/ast/{mod,vnode,children,joined_str,positions,linerange,qualname,literal,walker,trace}.rs`, `src/core/context.rs`, `bandit --dump-walk`, `scripts/dump_walk.py` | **Fait + validé** : trace de parcours identique à bandit Python sur **94/94 exemples et 120/120 fichiers de la stdlib** (avec `BANDITRS_PYTHON_COMPAT=3.11`) |
| M3 | `src/core/blacklist.rs` (données + test B001), `src/core/registry.rs` (table des 42 plugins), `src/core/plugin_config.rs` (défauts + `from_config`), `src/core/nosec.rs`, `src/core/docs_utils.rs`, `src/core/test_set.rs` (`TestSet::new`), `src/core/tester.rs` (`Tester::run_tests`), `src/core/config.rs` (défauts + `get_option` ; **chargement fichier YAML/TOML et profils legacy restent stub**) | **Fait** (sauf chargement de fichier de config, voir M7) |
| M4 | `src/plugins/*.rs` — **42/42 plugins implémentés** (voir docs/spec/plugins.md) ; `django_mark_safe` (B703) a une limitation connue sur `DeepAssignation` (DEVIATIONS.md #9), sans impact sur la suite de base | **Fait** |
| M5 | `src/core/discover.rs::discover_files`, `src/core/scan.rs::scan_file` (`catch_unwind`, nosec depuis les tokens), `src/core/manager.rs::run_tests` (parallèle via `rayon`) | **Fait** ; `tests/common/mod.rs::check_example`/`check_metrics` implémentés, **78 tests fonctionnels au vert** |
| M6 | `src/formatters/*.rs` (csv/custom/html/json/sarif/screen/text/xml/yaml + `mod.rs::output_results`), `src/pycompat/{pyformat,csv,json,yaml_emit,html,xml,urlquote}.rs` | **Fait** (sauf `pycompat::yaml_load`/`configparser`, nécessaires pour M7) ; **11 tests fonctionnels** (`tests/formatters.rs`, §C.7) au vert |
| M7 | `src/cli/{argparse,main}.rs` (**stubs**) ; chargement de `BanditConfig` depuis un fichier YAML/TOML (reste de M3) ; `src/pycompat/{yaml_load,configparser}.rs` (**stubs**) | À faire (prochaine étape) |
| M8 | `src/cli/{baseline,config_generator}.rs` (**stubs**) | À faire |
| M9 | `scripts/diff_against_python.sh` (écrit, à exécuter quand la CLI existe) | À faire |
| M10 | perf, README, clippy, push | À faire |

Chaque stub porte un commentaire de module décrivant précisément ce qu'il doit faire et renvoie au paragraphe
de spec correspondant. Chercher `todo!(` et `TODO(M` pour la liste exhaustive (restant : formatters M6, CLI M7,
baseline/config-generator M8, chargement de fichier de config M7).

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

## 5. Prochaines étapes détaillées (dans l'ordre)

M3, M4 et M5 sont **faits** (moteur de tests, 42 plugins, discover/scan/manager) — voir §3 pour le détail et
DEVIATIONS.md #9 pour la seule limitation connue (`django_mark_safe`/`DeepAssignation`). Restent explicitement
hors de la suite de base (M4, config spécifique) : `test_asserts`, `test_try_except_*`, `test_markupsafe_*`,
`test_django_xss_*` (profil `exclude B308`) — nécessitent le chargement de config personnalisée (M7) pour être
exercés avec des valeurs non-défaut ; `mark_safe_secure.py`/`mark_safe_insecure.py` dépendent en plus de la
limitation DeepAssignation ci-dessus.

Note pour la suite : le chargement de `BanditConfig` depuis un fichier (YAML/TOML) et `profile(name)` (legacy)
sont encore des stubs dans `core/config.rs` (`BanditConfig::new(Some(path))`, `todo!()`) ; nécessaires pour M7
(`-c`/`-p`) et pour les tests M4 ci-dessus. À faire : YAML via `pycompat::yaml_load` (à écrire : `saphyr-parser`
+ résolveurs PyYAML 1.1) ou TOML (`toml` crate, `[tool.bandit]`) ; `validate()` ; `convert_legacy_config()`
(garder l'inversion `bad_calls`/`bad_imports`, testée) ; `profile(name)`. Tests : `tests/unit/core/test_config.py`.

### M6 — formatters (`docs/spec/cli_formatters_tests.md` partie B) : json/txt/screen d'abord (tests runtime), puis
custom (`pycompat::pyformat`), csv, xml, html (templates verbatim à recopier depuis `bandit/formatters/html.py:171-323`),
yaml (`pycompat::yaml_emit`), sarif. Tests : `tests/formatters.rs` (§C.7).

### M7 — CLI : `cli/argparse.rs` (doc de module), `cli/main.rs` (flux §A.11 ; `fn run(argv) -> i32` testable ;
`.bandit` INI via `pycompat::configparser`), `--dump-walk` conservé comme option cachée. Tests : `tests/runtime.rs`,
`tests/cli_tools.rs` (§C.4).

### M8 — `bandit-baseline` (git CLI) et `bandit-config-generator` ; tests §C.3, §C.5, §C.6.

### M9 — harnais différentiel : `scripts/diff_against_python.sh` (walk traces + JSON + tous formats sur
`examples/` puis `/usr/lib/python3.11`) ; objectif zéro diff sur `examples/` (normalisation `generated_at` et
version de doc) ; les écarts restants doivent être expliqués par `DEVIATIONS.md` ou la grammaire (ruff accepte du 3.12+).

### M10 — `cargo clippy --all-targets -- -D warnings`, `cargo fmt`, benchmark
(`time bandit -r /usr/lib/python3.11` Python vs Rust `--release`, objectif ≥ 20×), README, commit/push.

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
- Ordre des issues d'un même nœud = ordre `setup.cfg` du registre, B001 en dernier.
- clippy : 10 avertissements mineurs restants (`if` imbriqués, doc list indentation) à nettoyer en M10.

## 7. Commandes utiles

```bash
cargo build --release && cargo test                       # unités + 78 fonctionnels (tests/functional.rs)
BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit --dump-walk examples/nosec.py   # trace Rust
/home/user/.pyenv-bandit/bin/python scripts/dump_walk.py examples/nosec.py           # trace Python
scripts/diff_against_python.sh examples                   # harnais différentiel complet (après M7)
/home/user/.pyenv-bandit/bin/bandit -r examples -f json   # référence Python
```
