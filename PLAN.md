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
miroir par fichier de test Python) = **341 tests, tous actifs et au vert, zéro `#[ignore]`**
(`scripts/wp_status.sh --check` → 0). **Jalon J1 atteint le 2026-09-09** : les 273 tests de la suite Python
ont un homologue Rust homonyme (263 portés — 225 à l'identique, 38 adaptés — et 10 non portables, justifiés
dans `docs/plan/test-inventory.md` et `DEVIATIONS.md` #8). La table complète des exemples upstream
(comptes de sévérité/confiance) correspond bit à bit à bandit Python.

Différentiel complet rejoué en fin de vague A2 (2026-09-09) via `scripts/diff_against_python.sh examples` :
traces de parcours **91/91 identiques**, et `examples/` × {json, txt, csv, xml, yaml, custom, sarif, html}
**sans aucun diff inexpliqué**. Les seuls écarts subsistants sont documentés : DEVIATIONS.md #5 (adresses
mémoire `<ast.List object at 0x…>` de `tarfile_extractall`), #10 (scalaire double-quoted non replié en YAML),
#11 (ancres/alias `&id001`/`*id001` de PyYAML, basées sur l'identité d'objet Python, non reproduites), plus
les champs volatils (`generated_at`, `Run started:`, version de l'outil et URL de doc — cf. #12) et la barre
de progression `Working…` que le bandit Python écrit sur la sortie. Depuis WP-01, `mark_safe_*` ne produit
plus de diff (`DeepAssignation` implémenté, DEVIATIONS.md #9 réécrite en conséquence).

| Jalon (plan parallèle) | Contenu | État |
|---|---|---|
| J0 | Restructuration : squelette miroir de la suite Python, `docs/plan/` (inventaire, 16 fiches de lots, playbook, benchmarks), agents `.claude/agents/banditrs-wp-*`, skill `banditrs-dispatch`, `benches/e2e.rs`, `scripts/{wp_status,bench_vs_python}.sh`, CI | **Fait** (2026-09-08) |
| J1 | Suite Python 100 % portée (WP-01 → WP-13, 176 stubs) | **Fait** (2026-09-09) : vague A1 (WP-02, 05, 07, 11, 12, 13) puis vague A2 (WP-01, 03, 04, 06, 09, 10, puis WP-08) — 176 stubs activés, 0 restant, `scripts/wp_status.sh --check` → 0 |
| J2 | Corpus golden + différentiel rejoué **en local** (WP-14 ; WP-16 suspendu, cf. ci-dessous) | à lancer (vague B) |
| J3 | Benchmarks, tableau Python vs Rust, garde-fou (WP-15) | à lancer (vague B) |

> **CI GitHub Actions désactivée le 2026-09-09 à la demande de l'utilisateur** (aucun coût souhaité). Le
> workflow est conservé, inerte, dans `.github/workflows/ci.yml.disabled` (GitHub ne lit que `*.yml`/`*.yaml`).
> La porte de qualité s'exécute **en local** via `scripts/check.sh`, qui reproduit à l'identique les trois jobs
> du workflow (build + tests + `wp_status.sh --check` ; rustfmt + clippy `-D warnings` ; compilation des benchs).
> À lancer avant chaque commit, et systématiquement avant un push. Pour réactiver un jour :
> `git mv .github/workflows/ci.yml.disabled .github/workflows/ci.yml`.

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
| M8 | `src/cli/{baseline,config_generator}.rs` | **Fait** ; testé manuellement (dépôt git jetable) et via différentiel contre Python — voir §5. `tests/baseline_functional.rs`/`tests/cli_tools.rs` restent des placeholders (§C.3/C.4/C.5/C.6 pas portés en tests automatisés) |
| M9 | Harnais différentiel exécuté ad hoc (voir §5) sur `examples/` (tous formats) : diffs restants tous expliqués par DEVIATIONS.md. `scripts/diff_against_python.sh` (script fichier) pas encore mis à jour/exécuté sur la stdlib | Essentiellement fait, script à finaliser |
| M10 | perf, README, clippy | **Fait** : `cargo clippy --all-targets -- -D warnings` propre, `cargo fmt --all`, benchmark (~70× sur stdlib), README à jour |

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

- DEVIATIONS.md #9 : `DeepAssignation` est complet depuis WP-01 ; ne subsistent que deux divergences volontaires (bugs Python non reproduits), sans impact sur la suite ni sur les fixtures.
- ~~`test_asserts`, `test_try_except_*`, `test_markupsafe_*`, `test_django_xss_*`~~ : **portés par WP-01**
  (helpers `manager_with`/`check_example_with` dans `tests/common/mod.rs`, qui construisent un `Manager` à
  partir d'un config/profil explicites au lieu de `manager_for_default()`).
- `apply_ini_options` (`src/cli/main.rs`) ne journalise « Using command line arg for selected targets » que si
  la clé `targets` est présente dans le `.bandit`, alors que Python l'émet dès que `args.targets` est fourni en
  ligne de commande, indépendamment de la clé ini. Écart pré-existant repéré lors de la revue de WP-03 (hors de
  son périmètre) : à corriger, ou à assumer avec une entrée `DEVIATIONS.md`.
- ~~`BanditConfig::profile()` ne fait pas la conversion legacy `blacklist_calls`/`blacklist_imports`~~ :
  **implémenté par WP-08** (`convert_legacy_config` + `validate`, appelés une fois depuis `new()`, avec
  l'inversion amont `bad_calls`/`bad_imports` conservée telle quelle). C'était le dernier `todo!()`
  fonctionnel du projet.
- **Piège découvert par WP-08, à connaître avant de toucher aux profils** : `Profile::blacklist` est un
  `Option<IndexMap<…>>` dont `TestSet::new` ne distingue que `Some` (« remplace la table intégrée ») et
  `None` (« utilise la table intégrée filtrée »). Un `Some(map vide)` y est donc lu comme « remplace par
  rien » et fait disparaître **silencieusement** tous les résultats de la blacklist intégrée pour un scan
  `-p <profil>` dont la section `profiles` ne référence ni `blacklist_calls` ni `blacklist_imports`
  (reproduit : `-p test_4 examples/xml_sax.py`, 8 résultats côté Python contre 0 côté Rust). La conversion
  legacy produit donc `None`, pas `Some(vide)`, quand elle n'a rien à ajouter — c'est le comportement de
  `if not blacklist:` en Python, où « absent » et « vide » sont indiscernables. Toute évolution de ce type
  doit préserver cette équivalence.
- `src/cli/baseline.rs::name_rev()` rend `master` là où Python (`commit.name_rev`) rend `<sha> master`, dans
  le message « Got current/parent commit: … ». Écart pré-existant confirmé lors de la revue de WP-04 (hors de
  son périmètre, aucun test de `test_baseline.py` ne l'observe) : à corriger, ou à assumer avec une entrée
  `DEVIATIONS.md`.
- Tests unitaires et fonctionnels : **tous portés** (J1 atteint, 0 stub — cf. §3 et
  `docs/plan/test-inventory.md`).

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
scripts/check.sh                                          # PORTE DE QUALITÉ LOCALE (remplace la CI) — à lancer avant tout push
scripts/check.sh fast                                     # idem sans la compilation des benchs (boucle de dév)
cargo build --release && cargo test --all-targets         # 341 tests (67 unitaires + 274 d'intégration), 0 ignoré
scripts/wp_status.sh                                      # stubs restants par lot (docs/plan/README.md)
cargo bench --bench e2e                                   # benchs criterion ; scripts/bench_vs_python.sh pour Python vs Rust
BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit --dump-walk examples/nosec.py   # trace Rust
/home/user/.pyenv-bandit/bin/python scripts/dump_walk.py examples/nosec.py           # trace Python
scripts/diff_against_python.sh examples                   # harnais différentiel (script à finaliser, cf. §5)
BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit -r examples -f json   # équivalent Rust
/home/user/.pyenv-bandit/bin/bandit -r examples -f json   # référence Python (venv à recréer si absent, cf. §2)
```
