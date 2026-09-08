# WP-08 — `BanditConfig` : YAML/TOML, `get_option`, conversion des configs legacy

**Agent** : `banditrs-wp-high` · **Vague** A · **Fichier de test** : `tests/unit_core_config.rs` · **Stubs** : 26 (16 YAML + 10 TOML)
**Python** : `tests/unit/core/test_config.py`, `bandit/core/config.py` · **Rust** : `src/core/config.rs`

## Objectif

Terminer `BanditConfig` : la conversion des configurations **legacy** (`profiles` avec
`blacklist_calls`/`blacklist_imports`, `bad_name_sets`/`bad_import_sets`) est le dernier `todo` fonctionnel
du projet (`PLAN.md` §5). Elle doit reproduire le code Python **y compris son inversion de paramètres**
(`convert_legacy_blacklist_tests(profiles, bad_imports, bad_calls)` est appelé avec `(…, bad_calls,
bad_imports)`), que `test_converted_blacklist_call_data` fige : le profil `blacklist_calls` reçoit les
données `telnet` (imports) et `blacklist_imports` reçoit `pickle` (calls). `DEVIATIONS.md` (préambule)
demande explicitement de conserver cette inversion.

## Fichiers

- Possédés : `tests/unit_core_config.rs`, `src/core/config.rs`.
- Lus : `docs/spec/core.md` §6 ; `src/core/blacklist.rs` (`BlacklistEntry` a tous ses champs `pub` :
  construire par littéral de struct, **ne pas éditer** le fichier — WP-10) ; `src/core/test_set.rs`
  (consomme `Profile.blacklist`).

## API à exposer / compléter (`src/core/config.rs`)

```rust
impl BanditConfig {
    /// `validate(path)` : erreurs `ConfigError` legacy + warning de dépréciation.
    pub fn validate(&self, path: &str) -> Result<(), ConfigError>;
    /// `convert_legacy_config()` : `profiles` convertis (`include`/`exclude` en ids, `blacklist`) —
    /// appelé par `new()` ; `profile(name)` renvoie le résultat converti.
    pub fn convert_legacy_config(&mut self);
}
```
`ConfigError` s'affiche `"{path} : {message}"` et expose `message` (comme `e.message` Python).
`get_option("a.b")` : `None` si un niveau manque **ou est faux** (déjà le cas).

## Tests (YAML ; la classe `TestTomlConfig` rejoue les 10 tests `TestConfigCompat` avec le TOML — suffixe `_toml`)

| Test | Mise en place | Attendu |
|---|---|---|
| `test_settings` | fichier `<clé aléatoire>: <valeur>` | `get_setting("plugin_name_pattern") == Some("*.py")` ; `raw == {clé: valeur}` ; `get_option(clé)` = valeur |
| `test_file_does_not_exist` | `<cwd>/notafile` | `Err(ConfigError)` dont l'affichage contient le chemin |
| `test_yaml_invalid` | `- [ something` | `Err` contenant le chemin |
| `test_levels` | `k:\n  sk: sv` ; `get_option("k.sk")` | `sv` |
| `test_levels_not_exist` | `get_option("<x>.<y>")` | `None` |
| `test_not_exist` | `get_setting("<aléatoire>")` | `None` |
| `test_converted_include` | échantillon `sample` (ci-dessous) ; `profile("test_1")` | `include == {"B101","B604"}`, `exclude == {}`, `blacklist == {}` (map vide, pas `None`) |
| `test_converted_exclude` | `profile("test_4")` | `exclude == {"B101"}` |
| `test_converted_blacklist_call_data` | `profile("test_2").blacklist["Call"]` | `[BlacklistEntry { qualnames: ["telnetlib"], level: "HIGH", message: "{name} is considered insecure.", name: "telnet", .. }]` |
| `test_converted_blacklist_import_data` | `profile("test_3").blacklist["Call"|"Import"|"ImportFrom"]` | chacun `== [BlacklistEntry { message: "{name} library appears to be in use.", name: "pickle", qualnames: ["pickle.loads"], .. }]` |
| `test_converted_blacklist_call_test` | `profile("test_2").include` | `{"B001"}` |
| `test_converted_blacklist_import_test` | `profile("test_3").include` | `{"B001"}` |
| `test_converted_exclude_blacklist` | `profile("test_5").exclude` | `{"B001"}` |
| `test_deprecation_message` | `raw = {"profiles": {}}` ; `validate("")` sous `log::with_buffer` | une entrée `Warning` : `Config file '' contains deprecated legacy config data. Please consider upgrading to the new config format. The tool 'bandit-config-generator' can help you with this. Support for legacy configs will be removed in a future bandit version.` |
| `test_blacklist_error` | pour `name` dans `["blacklist_call", "blacklist_imports", "blacklist_imports_func"]` : `raw = {"profiles": {"test": {"include": [name]}}}` ; `validate("")` | **vérifier empiriquement avec le venv** quels noms lèvent (le test Python n'asserte que dans `except`) ; pour ceux qui lèvent, `message == " : Config file has an include or exclude reference to legacy test '<name>' but no configuration data for it. Configuration data is required for this test. Please consider switching to the new config file format, the tool 'bandit-config-generator' can help you with this."` |
| `test_bad_yaml` | fichier `[]` | `Err` dont `message` contient `Error parsing file.` |

Échantillon YAML (`sample`, verbatim `test_config.py` lignes 121–166) et TOML (lignes 283–317) : recopier.
Pour `BlacklistEntry`, les champs non cités (`id`, `cwe`, `level` pour pickle) prennent ce que Python
produit : `id` absent → `"LEGACY"` côté `report_issue` ; ne pas inventer de valeurs, lire
`convert_legacy_blacklist_data` (`val["name"] = key`, `{func}`/`{module}` → `{name}`, `imports` → `qualnames`).

## Critères d'acceptation

- `cargo test --test unit_core_config` : 26 tests verts.
- `cargo test --test functional`, `--test runtime`, tests unitaires de `config.rs` inchangés.
- Différentiel avec un fichier legacy réel : écrire `sample` dans `/tmp/legacy.yaml`, puis
  `diff <(py bandit -c /tmp/legacy.yaml -p test_2 examples/telnetlib.py -f json) <(rust idem)` (normaliser
  `generated_at`) → zéro diff ; idem `-p test_3` sur `examples/pickle_deserialize.py`.
- Porte de qualité ; inventaire mis à jour ; `PLAN.md` n'est pas édité (signaler dans le rapport que le
  `todo` legacy est clos).

## Pièges

- TOML : la table utile est `[tool.bandit]` ; `[[tool.bandit.blacklist_calls.bad_name_sets]]` est une
  liste de tables à une clé (`pickle`) — même forme que la liste YAML de dicts à une clé.
- Un fichier chargé **remplace** les défauts (pas de fusion de `include`).
- `get_option("profiles")` après conversion doit renvoyer la structure convertie (les tests Python lisent
  `config.get_option("profiles")["test_1"]`) : en Rust, `profile(name)` suffit, mais la conversion doit
  avoir lieu **une fois** dans `new()` (pas à chaque appel).
