# WP-02 — Scénarios fonctionnels `bandit -b` (baseline)

**Agent** : `banditrs-wp-medium` · **Vague** A · **Fichier de test** : `tests/functional_baseline.rs` · **Stubs** : 7
**Python** : `tests/functional/test_baseline.py` · **Rust** : aucun fichier `src/` (tests via le binaire)

## Objectif

Rejouer les 7 scénarios de comparaison à une baseline en pilotant le binaire `bandit`
(`env!("CARGO_BIN_EXE_bandit")`), exactement comme le test Python pilote `bandit` en sous-processus.
Aucune modification de `src/` n'est attendue ; si un scénario échoue, c'est un bug de parité à
**rapporter** (le baseline appartient à WP-06/WP-03), sauf s'il est trivialement dans un fichier possédé.

## Fichiers

- Possédés : `tests/functional_baseline.rs` (helpers inclus dans ce fichier).
- Lus : `docs/spec/cli_formatters_tests.md` §C.3, `src/cli/main.rs` (flux `-b`), `src/formatters/text.rs`.

## Helpers (dans le fichier de test)

```rust
/// `_create_baseline(paired)` : tmpdir ; copie `examples/<value>` → `<tmp>/<key>` ;
/// `bandit -r [--ignore-nosec] -f json -o <tmp>/baseline_report.json <tmp>` ; puis copie
/// `examples/<key>` par-dessus `<tmp>/<key>`. Retourne (tmpdir, rc de la création).
fn create_baseline(pairs: &[(&str, &str)], ignore_nosec: bool) -> (tempfile::TempDir, i32);
/// `_run_bandit_baseline` : `bandit -r [--ignore-nosec] -b <baseline> <tmp>` → (stdout, rc).
fn run_baseline(dir: &Path, ignore_nosec: bool) -> (String, i32);
```
Ordre des arguments comme en Python : `["-r", ("--ignore-nosec")?, "-f", "json", "-o", <report>, <dir>]`
et `["-r", ("--ignore-nosec")?, "-b", <report>, <dir>]`. Python ne lit que **stdout** (le formatter
`txt` est sélectionné car stdout n'est pas un TTY) ; ne pas concaténer stderr.

## Constantes (verbatim du test Python)

```
"Total lines of code: 12"   "Total lines of code: 9"   "Total lines of code: 1"
"Total lines skipped (#nosec): 0"   "Total lines skipped (#nosec): 3"
"Files skipped (0):"   "No issues identified."
"Issue: [B317:blacklist]"   "Issue: [B506:yaml_load]"   "Issue: [B602:subprocess_popen_with_shell_equals_true]"
c1 = "subprocess.Popen('/bin/ls *', shell=True)"      c2 = c1 + " # nosec"
c3 = "y = yaml.load(temp_str)"                         c4 = c3 + " # nosec"
c5 = "xml.sax.make_parser()"                           c6 = c5 + " # nosec"
```

## Scénarios

| Test | `<key>` ← `<value>` | `--ignore-nosec` | rc création | rc run | sous-chaînes attendues dans stdout |
|---|---|---|---|---|---|
| `test_no_new_candidates` | `new_candidates-all.py` ← `new_candidates-all.py` | non | 1 | 0 | loc 12, nosec 3, skipped 0, no issues |
| `test_no_existing_no_new_candidates` | `okay.py` ← `okay.py` | non | 0 | 0 | loc 1, nosec 0, skipped 0, no issues |
| `test_no_existing_with_new_candidates` | `new_candidates-all.py` ← `new_candidates-none.py` | non | 0 | 1 | loc 12, nosec 3, skipped 0, B317, B506, B602, c1, c3, c5 |
| `test_existing_and_new_candidates` | `new_candidates-all.py` ← `new_candidates-some.py` | non | 1 | 1 | loc 12, nosec 3, skipped 0, B317, B506, c3, c5 |
| `test_no_new_candidates_include_nosec` | `new_candidates-all.py` ← `new_candidates-all.py` | oui | 1 | 0 | loc 12, nosec 0, skipped 0, no issues |
| `test_new_candidates_include_nosec_only_nosecs` | `new_candidates-nosec.py` ← `new_candidates-none.py` | oui | 0 | 1 | loc 9, nosec 0, skipped 0, B317, B506, B602, c2, c4, c6 |
| `test_new_candidates_include_nosec_new_nosecs` | `new_candidates-all.py` ← `new_candidates-none.py` | oui | 0 | 1 | loc 12, nosec 0, skipped 0, B317, B506, B602, c1…c6 |

## Critères d'acceptation

- `cargo test --test functional_baseline` : 7 tests, 0 ignoré, 0 échec, en parallèle (chaque test a son
  propre `TempDir`, pas de `set_current_dir`).
- Porte de qualité ; lignes WP-02 de l'inventaire → « porté ».

## Pièges

- Les chemins des fichiers copiés apparaissent dans la sortie (`Location: <tmp>/…`) : ne pas asserter
  sur des chemins, seulement sur les sous-chaînes ci-dessus.
- `examples/` se trouve via `env!("CARGO_MANIFEST_DIR")` (les tests Python supposent le cwd du dépôt).
- Le rc de la création de baseline vaut 1 quand des issues sont trouvées (c'est le rc normal de `bandit`).
