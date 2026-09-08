# WP-05 — `bandit-config-generator`

**Agent** : `banditrs-wp-low` · **Vague** A · **Fichier de test** : `tests/unit_cli_config_generator.rs` · **Stubs** : 6
**Python** : `tests/unit/cli/test_config_generator.py`, `bandit/cli/config_generator.py` · **Rust** : `src/cli/config_generator.rs`, `src/core/plugin_config.rs`

## Objectif

Exposer l'analyse des arguments et la génération des réglages par défaut, puis porter les 6 tests.

## Fichiers

- Possédés : `tests/unit_cli_config_generator.rs`, `src/cli/config_generator.rs`, `src/core/plugin_config.rs`.
- Lus : `docs/spec/cli_formatters_tests.md` §A.14, C.6.

## API à exposer (`src/cli/config_generator.rs`)

```rust
pub struct GenArgs { pub show_defaults: bool, pub output_file: Option<String>,
                     pub tests: Option<String>, pub skips: Option<String> }
/// `parse_args()` : `Err(code)` quand argparse sort (aide, erreur) — sans `-o` ni `--show-defaults`,
/// Python affiche l'aide et sort avec 1.
pub fn parse_args(argv: &[String]) -> Result<GenArgs, i32>;
/// `get_config_settings()` : `yaml.safe_dump(config, default_flow_style=False)` des `gen_config`
/// (= `PluginConfigs::defaults_yaml()`).
pub fn get_config_settings() -> String;
pub fn init_logger();
```

## Tests

| Test | Mise en place | Attendu |
|---|---|---|
| `test_init_logger` | `init_logger()` | `log::level() == Level::Info` (format `[%(levelname)5s]: %(message)s`, stdout) |
| `test_parse_args_no_defaults` | `parse_args(&[])` | `Err(1)` |
| `test_parse_args_show_defaults` | `["--show-defaults"]` | `Ok(a)` avec `a.show_defaults` |
| `test_parse_args_out_file` | `["--out", "dummyfile"]` | `a.output_file == Some("dummyfile")` |
| `test_get_config_settings` | comparer `get_config_settings()` au YAML de référence **généré par Python** : `/home/user/.pyenv-bandit/bin/bandit-config-generator --show-defaults` (le coller verbatim dans le test comme `const EXPECTED: &str`) | égalité stricte, octet à octet |
| `test_main_show_defaults` | `main(["--show-defaults"])` | retourne 0 (et imprime le YAML ; acceptable dans un test) |

## Critères d'acceptation

- `cargo test --test unit_cli_config_generator` : 6 tests verts (sérialiser `test_init_logger` avec un
  `Mutex` si d'autres tests touchent le logger global).
- `bandit-config-generator --show-defaults` inchangé (différentiel : `diff <(python…) <(rust…)` vide).
- Porte de qualité ; inventaire mis à jour.

## Pièges

- L'ordre des clés du YAML est celui de `yaml.safe_dump` (clés triées) et l'ordre des plugins celui de
  `extension_loader.MANAGER.plugins` (entry points triés par nom) — déjà reproduit par `defaults_yaml()`.
- `--out` et `-o` sont synonymes ; `-t`/`-s` sont des listes (`nargs="*"`).
