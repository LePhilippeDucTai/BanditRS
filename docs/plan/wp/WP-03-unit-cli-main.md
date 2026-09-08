# WP-03 — `bandit.cli.main` : options ini, `_log_option_source`, codes de sortie

**Agent** : `banditrs-wp-high` · **Vague** A · **Fichier de test** : `tests/unit_cli_main.rs` · **Stubs** : 19 (+1 non portable)
**Python** : `tests/unit/cli/test_main.py`, `bandit/cli/main.py` · **Rust** : `src/cli/main.rs`, `src/cli/argparse.rs`

## Objectif

Rendre testables unitairement les briques de `cli/main.rs` (aujourd'hui privées) et couvrir les codes
de sortie de `main()` par des exécutions du binaire dans un répertoire temporaire. Les tests Python
utilisent `mock` pour forcer des branches ; on remplace chaque mock par la situation réelle qui déclenche
la même branche (colonne « Adaptation »).

## Fichiers

- Possédés : `tests/unit_cli_main.rs`, `src/cli/main.rs`, `src/cli/argparse.rs`.
- Lus : `docs/spec/cli_formatters_tests.md` §A.4, A.6, A.11, C.4 ; `src/log.rs` (`set_level`, `level`,
  `with_buffer`) ; `src/pycompat/configparser.rs` (comportement sur un répertoire).

## API à exposer dans `src/cli/main.rs` (noms = attributs Python)

```rust
/// `_get_options_from_ini(ini_path, target)` : `Ok(None)` sans fichier, `Ok(Some(map))` avec la
/// section `[bandit]`, `Err(MultipleIniFiles(paths))` quand plusieurs `.bandit` sont trouvés
/// (main journalise `Multiple .bandit files found - scan separately or choose one with --ini\n\t<a>, <b>`
/// et sort avec 2).
pub fn get_options_from_ini(ini_path: Option<&str>, targets: &[String])
    -> Result<Option<IndexMap<String, String>>, MultipleIniFiles>;

/// `_log_option_source(default_val, arg_val, ini_val, option_name)` avec la véracité Python
/// (`Some("")` est faux).
pub fn log_option_source(default_val: Option<&str>, arg_val: Option<&str>, ini_val: Option<&str>,
    option_name: &str) -> Option<String>;

/// `_init_logger(log_level, log_format)`.
pub fn init_logger(level: log::Level, log_format: Option<&str>);
```
Garder `main()` inchangé dans son comportement ; il appelle ces fonctions.

## Tests

| Test | Mise en place / adaptation | Attendu |
|---|---|---|
| `test_init_logger` | `init_logger(Level::Info, None)` | `log::level() == Level::Info` et `log::enabled(Level::Info)` |
| `test_init_logger_debug_mode` | `init_logger(Level::Debug, None)` | `log::level() == Level::Debug` |
| `test_get_options_from_ini_no_ini_path_no_target` | `(None, &[])` | `Ok(None)` |
| `test_get_options_from_ini_empty_directory_no_target` | `(Some(<tmpdir>), &[])` — un répertoire comme `ini_path` | `Ok(None)` (Python : `parse_ini_file` échoue → warning → `None`) |
| `test_get_options_from_ini_no_ini_path_no_bandit_files` | `(None, &[<tmpdir>])` | `Ok(None)` |
| `test_get_options_from_ini_no_ini_path_multi_bandit_files` | `<tmp>/.bandit` et `<tmp>/second_config_directory/.bandit` (contenu = `bandit_config_content`) | `Err(MultipleIniFiles)` ; **et** le binaire `bandit <tmp>` sort avec 2 et stderr contient `Multiple .bandit files found` |
| `test_log_option_source_arg_val` | `(None, "file", "vuln")` et `("default", "file", "vuln")` | `Some("file")` |
| `test_log_option_source_ini_value` | `(None, None, "vuln")` | `Some("vuln")` |
| `test_log_option_source_ini_val_with_str_default_and_no_arg_val` | `("file", "file", "vuln")` | `Some("vuln")` |
| `test_log_option_source_no_values` | `(None, None, None)` | `None` |
| `test_main_config_unopenable` | cwd = tmp sans `bandit.yaml` ; `bandit -c bandit.yaml test` | rc 2 ; stderr contient `bandit.yaml : Could not read config file.` |
| `test_main_invalid_config` | `bandit.yaml` = `- [ something` | rc 2 ; stderr contient `Error parsing file.` |
| `test_main_handle_ini_options` | `bandit.yaml` = `bandit_config_content` ; `.bandit` = `[bandit]\nexclude = /tmp\nskips = skip_test\ntests = some_test\n` ; `bandit -c bandit.yaml --ini .bandit test` | rc 2 ; stderr contient `No tests would be run, please check the profile.` (les ids inconnus ne font que des warnings, cf. `extension_loader.validate_profile`) |
| `test_main_profile_not_found` | `bandit.yaml` ; `bandit -c bandit.yaml -p bad test` | rc 2 ; stderr contient `Unable to find profile (bad) in config file: bandit.yaml` |
| `test_main_baseline_ioerror` | `bandit.yaml` ; `base.json` est un **répertoire** (ouverture impossible) ; `bandit -c bandit.yaml -b base.json test` | rc 2 ; stderr contient `Could not open baseline report: base.json` |
| `test_main_invalid_output_format` | `bandit.yaml` + `base.json` (= `bandit_baseline_content`) ; `bandit -c bandit.yaml -b base.json -f csv test` | rc 2 ; stderr contient `Baseline must be used with one of the following formats` |
| `test_main_exit_with_results` | `bandit.yaml` ; cible `examples/os_system.py` (chemin absolu) ; `-o output` | rc 1 ; fichier `output` créé |
| `test_main_exit_with_no_results` | cible `examples/okay.py` ; `-o output` | rc 0 |
| `test_main_exit_with_results_and_with_exit_zero_flag` | cible `examples/os_system.py` ; `-o output --exit-zero` | rc 0 |

`bandit_config_content` et `bandit_baseline_content` : recopier verbatim depuis `test_main.py` (lignes 15–47).
`test_init_extensions` : non portable (pas de `extension_loader`), documenté dans l'inventaire.

## Critères d'acceptation

- `cargo test --test unit_cli_main` : 19 tests verts, exécutables en parallèle : les deux tests
  `init_logger` partagent l'état global du logger → les sérialiser avec un `static LOCK: Mutex<()>` ;
  les tests « main » utilisent `Command::current_dir(tmp)` et jamais `set_current_dir`.
- `cargo test --test runtime` toujours vert (mêmes chemins de code).
- Porte de qualité ; inventaire mis à jour.

## Pièges

- `-o output` avec `nargs="?"` : `-o` suivi d'une valeur → fichier ; la cible doit venir **avant** `-o`
  dans la ligne de commande (comme dans le test Python) pour ne pas être avalée.
- `ConfigError` s'affiche `"{path} : {message}"` (cf. `tests/runtime.rs::test_nonexistent_config`).
- Un `.bandit` trouvé automatiquement (sans `--ini`) est cherché par `os.walk` **dans les cibles** ; avec
  `--ini` explicite, aucune recherche.
