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

`cargo build --all-targets` propre (0 warning), `cargo test` : 39 tests unitaires au vert ; les tests
d'intégration sont des squelettes `#[ignore]` (78 cas fonctionnels prêts).

| Jalon | Module(s) | État |
|---|---|---|
| M0 | `Cargo.toml`, `rust-toolchain.toml`, `src/lib.rs`, `src/log.rs`, `src/constants.rs`, `src/core/issue.rs`, `src/core/metrics.rs`, `src/pycompat/{datetime,fnmatch,splitlines,path,unicode_escape}.rs`, `src/core/utils.rs` | **Fait + tests** |
| M1 | `src/pycompat/encoding.rs` (PEP 263, BOM, latin-1, ascii, encoding_rs), `src/source/{file,parse}.rs` | **Fait + tests** |
| M2 | `src/ast/{mod,vnode,children,joined_str,positions,linerange,qualname,literal,walker,trace}.rs`, `src/core/context.rs`, `bandit --dump-walk`, `scripts/dump_walk.py` | **Fait + validé** : trace de parcours identique à bandit Python sur **94/94 exemples et 120/120 fichiers de la stdlib** (avec `BANDITRS_PYTHON_COMPAT=3.11`) |
| M3 | `src/core/blacklist.rs` (données complètes, test B001 **stub**), `src/core/registry.rs` (table complète des 42 plugins), `src/core/plugin_config.rs` (défauts complets, parsing **stub**), `src/core/nosec.rs` (parsing des commentaires fait, tests), `src/core/docs_utils.rs` (fait + tests), `src/core/test_set.rs` (**stub**), `src/core/tester.rs` (**stub**), `src/plugins/*.rs` (42 **stubs** `todo!()`), `src/core/config.rs` (modèle + `get_option` faits, chargement/legacy **stub**) | En cours |
| M4 | plugins | À faire |
| M5 | `src/core/discover.rs` (helpers faits + tests, `discover_files` **stub**), `src/core/scan.rs` (**stub**), `src/core/manager.rs` (modèle + baseline faits, `run_tests` **stub**) | À faire |
| M6 | `src/formatters/*.rs` (**stubs** documentés), `src/pycompat/{pyformat,csv,json,yaml_load,yaml_emit,configparser,html,xml,urlquote}.rs` (**stubs** documentés) | À faire |
| M7 | `src/cli/{argparse,main}.rs` (**stubs**) | À faire |
| M8 | `src/cli/{baseline,config_generator}.rs` (**stubs**) | À faire |
| M9 | `scripts/diff_against_python.sh` (écrit, à exécuter quand la CLI existe) | À faire |
| M10 | perf, README, clippy, push | À faire |

Chaque stub porte un commentaire de module décrivant précisément ce qu'il doit faire et renvoie au paragraphe
de spec correspondant. Chercher `todo!(` et `TODO(M` pour la liste exhaustive.

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

### M3 — moteur de tests (objectif : `imports.py`, `skip.py`, `nosec.py`, `multiline_statement.py` au vert)
1. `core/config.rs` : `BanditConfig::new(Some(path))` — YAML via `pycompat::yaml_load` (à écrire : `saphyr-parser`
   + résolveurs PyYAML 1.1) ou TOML (`toml` crate, `[tool.bandit]`) ; `validate()` ; `convert_legacy_config()`
   (garder l'inversion `bad_calls`/`bad_imports`) ; `profile(name)`. Tests : `tests/unit/core/test_config.py`.
2. `core/plugin_config.rs::from_config` : lire chaque section (`config.get_option(key)`) → structs typées
   (`ListOpt::{Missing,Null,Items}`, bool/int `Result<_, PyErr>`).
3. `core/test_set.rs::TestSet::new` : `_get_filter` (règles B001), plugins filtrés (ordre registre),
   `BlacklistTable::builtin().filtered(...)` ou données legacy du profil, `tests[kind]` = plugins puis
   `TestRef::Blacklist` pour Call/Import/ImportFrom. Tests : `tests/unit/core/test_test_set.py` (registre injectable).
4. `core/blacklist.rs::blacklist` : règles d'appariement (doc de module). Pour `Import`/`ImportFrom`, itérer
   `stmt.names` (`context.node.as_stmt()`), `prefix = module + "."` seulement pour `ImportFrom` avec module.
5. `core/tester.rs::Tester::run_tests` : boucle sur `test_set.get_tests(kind)`, appel plugin/blacklist,
   décoration de l'`Issue` (`IssueDraft` → `Issue` avec `fname = ctx.file.name`, `source = self.source.clone()`),
   porte nosec (`nosec.get(draft.lineno)` ∪ `nosec.for_range(ctx.linerange)`), `metrics.nosec`/`skipped_tests`,
   `Scores::note`. Le nom de test d'un plugin = `PluginDef::func_name` ; pour B001 = `"blacklist"`.
6. `core/nosec.rs` : déjà fonctionnel ; brancher la construction depuis `parsed.tokens()` (`TokenKind::Comment`,
   `&text[tok.range()]`, ligne = `file.line_index(tok.start().to_u32())`) dans `scan.rs`.
7. Premiers plugins : B101, B102, B104, B108, B110, B112 (spec `docs/spec/plugins.md`), B404/B403 via B001.

### M4 — tous les plugins (`docs/spec/plugins.md`), puis `tests/functional.rs` complet (retirer les `#[ignore]`)
Ordre conseillé : injection_shell (B602–B607), general_hardcoded_password (B105–B107), injection_sql (B608 :
`concat_string` sur la chaîne de `BinOp` via `ctx.ancestors`), insecure_ssl_tls, weak_cryptographic_key,
crypto_request/request_without_timeout, hashlib, yaml_load/pytorch/huggingface, tarfile, snmp, paramiko/ssh,
jinja2 (parcours BFS du sous-arbre de l'appel), mako, django_sql_injection, django_xss (le plus complexe :
`DeepAssignation` sur `parent.body`), markupsafe, logging_config, trojansource (contexte `File`,
`str_splitlines`, index en caractères). Ajouter les tests à config (`test_asserts`, `test_try_except_*`,
`test_markupsafe_*`, `test_django_xss_*` avec profil `exclude B308`).

### M5 — manager
`discover::discover_files` (règles §1.2 de core.md), `scan::scan_file` (pipeline dans la doc de module, `catch_unwind`),
`Manager::run_tests` (`rayon::par_iter` sur `files_list`, `log::with_buffer` par fichier, fusion dans l'ordre,
`-` → `<stdin>`, `metrics.insert`, `metrics.aggregate()`), `SourceStore::global().insert` pour les snippets.
Tests : `test_multiline_code`, `test_nonsense`, `test_metric_gathering`, `test_baseline_filter`, `tests/unit/core/test_manager.py`.

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
cargo build --release && cargo test                       # unités
cargo test --test functional -- --ignored                 # fonctionnels (une fois M3–M5 faits, retirer les #[ignore])
BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit --dump-walk examples/nosec.py   # trace Rust
/home/user/.pyenv-bandit/bin/python scripts/dump_walk.py examples/nosec.py           # trace Python
scripts/diff_against_python.sh examples                   # harnais différentiel complet (après M7)
/home/user/.pyenv-bandit/bin/bandit -r examples -f json   # référence Python
```
