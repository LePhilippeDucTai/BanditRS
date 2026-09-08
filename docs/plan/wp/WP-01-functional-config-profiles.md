# WP-01 — Tests fonctionnels à configuration/profil + `DeepAssignation`

**Agent** : `banditrs-wp-high` · **Vague** A · **Fichier de test** : `tests/functional.rs` · **Stubs** : 11
**Python** : `tests/functional/test_functional.py` (lignes 622–929) · **Rust** : `src/plugins/django_xss.rs`, `tests/common/mod.rs`

## Objectif

Porter les 11 tests fonctionnels qui ne passent pas par `manager_for_default()` : ils exigent une
configuration de plugin, un profil d'exclusion, ou inspectent les issues (positions, code, baseline).
Le seul vrai travail d'implémentation est `DeepAssignation` (B703) : la version Rust ne suit que les
affectations directes (DEVIATIONS.md #9) alors que `test_django_xss_insecure` attend le comportement
complet. À la fin du lot, l'entrée #9 de `DEVIATIONS.md` est **supprimée** (comportement identique).

## Fichiers

- Possédés : `tests/functional.rs`, `tests/common/mod.rs`, `src/plugins/django_xss.rs`.
- Lus seulement : `src/core/config.rs` (`BanditConfig.raw` est `pub`, `ConfigValue` est un enum public :
  on mute la map sans nouvelle API), `src/core/plugin_config.rs` (`PluginConfigs::from_config` lit les
  sections `assert_used`, `try_except_pass`, `try_except_continue`, `markupsafe_xss`), `src/core/manager.rs`
  (`get_issue_list`, `populate_baseline`, `skipped`, `baseline`), `src/core/issue.rs` (`get_code`, `linerange`).
- Interdits : tout le reste (en particulier `plugin_config.rs` → WP-05, `manager.rs` → WP-06).

## Helpers à ajouter dans `tests/common/mod.rs`

```rust
/// `BanditManager(b_conf, "file")` + `BanditTestSet(config=b_conf, profile=profile)`.
pub fn manager_with(config: BanditConfig, profile: Profile) -> Manager;
/// `BanditConfig()` dont `config[section] = value` (le `b_conf.config["markupsafe_xss"] = {...}` Python).
pub fn config_with_section(section: &str, value: ConfigValue) -> BanditConfig;
/// `check_example` sur un manager fourni (remise à zéro de `scores` entre deux appels).
pub fn check_example_with(mgr: &mut Manager, name: &str, severity: Counts, confidence: Counts);
```
`ConfigValue::Map(IndexMap<String, ConfigValue>)` : `indexmap` est une dépendance du crate, donc
utilisable depuis `tests/`.

## Tests à porter (comptes = `[UNDEFINED, LOW, MEDIUM, HIGH]`, sévérité puis confiance)

| Test | Mise en place | Attendu |
|---|---|---|
| `test_nonsense` | `run_example("nonsense.py")` (manager par défaut) | `mgr.skipped.len() == 1` |
| `test_django_xss_secure` | profil `Profile { exclude: {"B308"} }`, config par défaut ; `mark_safe_secure.py` | `[0,0,0,0]` / `[0,0,0,0]` |
| `test_django_xss_insecure` | même profil ; `mark_safe_insecure.py` | `[0,0,29,0]` / `[0,0,0,29]` |
| `test_asserts` | `assert_used: {skips: []}` → `assert.py` ; puis `{skips: ["*assert.py"]}` ; puis `{}` (section présente, vide) | `[0,1,0,0]`/`[0,0,0,1]` ; zéros ; `[0,1,0,0]`/`[0,0,0,1]` |
| `test_try_except_continue` | `try_except_continue: {check_typed_exception: true}` puis `false` ; `try_except_continue.py` | `[0,3,0,0]`/`[0,0,0,3]` ; `[0,2,0,0]`/`[0,0,0,2]` |
| `test_try_except_pass` | idem avec `try_except_pass` / `try_except_pass.py` | `[0,3,0,0]`/`[0,0,0,3]` ; `[0,2,0,0]`/`[0,0,0,2]` |
| `test_markupsafe_markup_xss_extend_markup_names` | `markupsafe_xss: {extend_markup_names: ["webhelpers.html.literal"]}` ; `markupsafe_markup_xss_extend_markup_names.py` | `[0,0,2,0]` / `[0,0,0,2]` |
| `test_markupsafe_markup_xss_allowed_calls` | `markupsafe_xss: {allowed_calls: ["bleach.clean"]}` ; `markupsafe_markup_xss_allowed_calls.py` | `[0,0,1,0]` / `[0,0,0,1]` |
| `test_multiline_code` | `run_example("multiline_statement.py")` ; `issues = mgr.get_issue_list(Rank::Low, Rank::Low)` (défauts Python `sev_level=LOW, conf_level=LOW`) | `skipped.len()==0` ; `files_list.len()==1` et se termine par `multiline_statement.py` ; 3 issues ; `issues[0].fname` finit par `examples/multiline_statement.py` ; `issues[0].lineno == 1`, `linerange.to_vec() == [1]`, `get_code(-1, false)` contient `"subprocess"` ; `issues[1].lineno == 5`, linerange `[3,4,5,6]`, code contient `"shell=True"` ; `issues[2].lineno == 11`, linerange `[8..=13]`, code contient `"shell=True"` |
| `test_code_line_numbers` | `run_example("binding.py")` ; `code_lines = issues[0].get_code(-1, false).lines()` ; `lineno = issues[0].lineno` | `code_lines[0]` commence par `"{lineno-1} "`, `[1]` par `"{lineno} "`, `[2]` par `"{lineno+1} "` (Python compare les 2 premiers caractères de `"%i " % n`) |
| `test_baseline_filter` | `mgr.populate_baseline(json)` avec le JSON ci-dessous puis `run_example("flask_debug.py")` | `mgr.baseline.len() == 1` ; `get_issue_list(Rank::Low, Rank::Low)` est la variante baseline **et** vide (`is_empty()`) |

JSON de `test_baseline_filter` (le `filename` doit être **le même chemin absolu** que celui passé à
`discover_files`, i.e. `example_path("flask_debug.py")`) :

```json
{"results": [{"code": "...", "filename": "<abs>/examples/flask_debug.py",
  "issue_confidence": "MEDIUM", "issue_severity": "HIGH",
  "issue_cwe": {"id": 94, "link": "https://cwe.mitre.org/data/definitions/94.html"},
  "issue_text": "A Flask app appears to be run with debug=True, which exposes the Werkzeug debugger and allows the execution of arbitrary code.",
  "line_number": 10, "col_offset": 0, "line_range": [10],
  "test_name": "flask_debug_true", "test_id": "B201"}]}
```

## `DeepAssignation` (B703, `bandit/plugins/django_xss.py`)

Porter **intégralement** la classe Python (`is_assigned_in`, `is_assigned`, `evaluate_var`,
`evaluate_call`, `transform2call`, `check_risk`) : parcours de `Expr`, `FunctionDef`, `With`, `Try`
(`body`/`handlers`/`orelse`/`finalbody`), `If`/`For`/`While` (`body`/`orelse`), `Assign` (cibles `Name`
**et** `Tuple`/`List` — déballage), `AugAssign`, et la remontée `ignore_nodes`. Les 29 issues attendues
sur `mark_safe_insecure.py` et les 0 sur `mark_safe_secure.py` sont la spécification ; vérifier en plus
par différentiel :

```bash
for f in mark_safe mark_safe_insecure mark_safe_secure; do
  diff <(/home/user/.pyenv-bandit/bin/bandit examples/$f.py -f json 2>/dev/null | python3 -m json.tool) \
       <(BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit examples/$f.py -f json 2>/dev/null | python3 -m json.tool)
done
```
(zéro diff attendu, y compris `line_number`/`col_offset`). Supprimer ensuite DEVIATIONS.md #9 et
renuméroter **n'est pas** nécessaire : remplacer le texte de #9 par « (supprimé : parité complète depuis WP-01) ».

## Critères d'acceptation

- `cargo test --test functional` : 86 tests, 0 ignoré, 0 échec.
- Différentiel ci-dessus à zéro diff ; `scripts/diff_against_python.sh examples` sans nouveau diff.
- Porte de qualité (`agent-playbook.md` §4) ; lignes WP-01 de `docs/plan/test-inventory.md` → « porté ».

## Pièges

- `check_example` Python remet `scores = []` avant chaque appel : faire de même dans `check_example_with`.
- `test_asserts` `{}` : section présente mais sans `skips` → même résultat que `skips: []`.
- `get_issue_list` retourne un enum (`IssueList::List` / `IssueList::Baseline`) : itérer via `.issues()`.
- Les positions de f-strings n'interviennent pas ici, mais lancer le différentiel avec
  `BANDITRS_PYTHON_COMPAT=3.11` par réflexe (`PLAN.md` §1).
