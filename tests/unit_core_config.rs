//! Port of `tests/unit/core/test_config.py` (`BanditConfig`, YAML + TOML, conversion legacy).
//!
//! Work package: `docs/plan/wp/WP-08-unit-core-config.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/unit/core/test_config.py`.
//!
//! `TestTomlConfig` in the Python suite subclasses `TestConfigCompat`, replaying its ten
//! tests against a TOML sample (`suffix = ".toml"`); the `_toml`-suffixed tests below are
//! that replay. Two of the ten (`test_bad_yaml`, `test_blacklist_error`) never actually
//! load `self.sample`: `test_bad_yaml` builds its own fixture with `TempFile("[]")`, whose
//! *default* suffix is `.yaml` (the `suffix` class attribute isn't threaded through), and
//! `test_blacklist_error` bypasses the file entirely by overwriting `_config` directly — so
//! both `_toml` variants below are byte-identical to their YAML counterparts, matching
//! upstream.

use std::io::Write as _;

use indexmap::{IndexMap, IndexSet};

use banditrs::core::blacklist::BlacklistEntry;
use banditrs::core::config::{BanditConfig, ConfigValue};
use banditrs::core::issue::Cwe;

/// `TestConfigCompat.sample` (`tests/unit/core/test_config.py` lines 121-166), verbatim.
const SAMPLE_YAML: &str = r#"
profiles:
    test_1:
        include:
            - any_other_function_with_shell_equals_true
            - assert_used
        exclude:

    test_2:
        include:
            - blacklist_calls

    test_3:
        include:
            - blacklist_imports

    test_4:
        exclude:
            - assert_used

    test_5:
        exclude:
            - blacklist_calls
            - blacklist_imports

    test_6:
        include:
            - blacklist_calls

        exclude:
            - blacklist_imports

blacklist_calls:
    bad_name_sets:
        - pickle:
            qualnames: [pickle.loads]
            message: "{func} library appears to be in use."

blacklist_imports:
    bad_import_sets:
        - telnet:
            imports: [telnetlib]
            level: HIGH
            message: "{module} is considered insecure."
"#;

/// `TestTomlConfig.sample` (`tests/unit/core/test_config.py` lines 283-317), verbatim.
const SAMPLE_TOML: &str = r#"
[tool.bandit.profiles.test_1]
include = [
    "any_other_function_with_shell_equals_true",
    "assert_used",
]

[tool.bandit.profiles.test_2]
include = ["blacklist_calls"]

[tool.bandit.profiles.test_3]
include = ["blacklist_imports"]

[tool.bandit.profiles.test_4]
exclude = ["assert_used"]

[tool.bandit.profiles.test_5]
exclude = ["blacklist_calls", "blacklist_imports"]

[tool.bandit.profiles.test_6]
include = ["blacklist_calls"]
exclude = ["blacklist_imports"]

[[tool.bandit.blacklist_calls.bad_name_sets]]
    [tool.bandit.blacklist_calls.bad_name_sets.pickle]
    qualnames = ["pickle.loads"]
    message = "{func} library appears to be in use."

[[tool.bandit.blacklist_imports.bad_import_sets]]
    [tool.bandit.blacklist_imports.bad_import_sets.telnet]
    imports = ["telnetlib"]
    level = "HIGH"
    message = "{module} is considered insecure."
"#;

/// `TempFile(contents, suffix)` + `config.BanditConfig(f.name)`.
fn build_config(
    contents: &str,
    suffix: &str,
) -> Result<BanditConfig, banditrs::core::config::ConfigError> {
    let mut f = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
    f.write_all(contents.as_bytes()).unwrap();
    let path = f.path().to_str().unwrap().to_string();
    BanditConfig::new(Some(&path))
}

/// `self.config` in `TestConfigCompat.setUp` / `TestTomlConfig.setUp`.
fn sample_config(suffix: &str) -> BanditConfig {
    let sample = if suffix == ".toml" {
        SAMPLE_TOML
    } else {
        SAMPLE_YAML
    };
    build_config(sample, suffix).expect("sample config must parse")
}

/// A `BanditConfig` built directly from a raw tree, bypassing file I/O — used for
/// `test_deprecation_message`/`test_blacklist_error`, which overwrite `self._config`
/// in place rather than loading a file.
fn config_with_raw(raw: ConfigValue) -> BanditConfig {
    BanditConfig {
        path: None,
        raw,
        plugin_name_pattern: "*.py".into(),
        profiles: IndexMap::new(),
    }
}

fn ids(names: &[&str]) -> IndexSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_bad_yaml`.
#[test]
fn test_bad_yaml() {
    check_bad_yaml();
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_blacklist_error`.
#[test]
fn test_blacklist_error() {
    check_blacklist_error();
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_converted_blacklist_call_data`.
#[test]
fn test_converted_blacklist_call_data() {
    check_converted_blacklist_call_data(".yaml");
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_converted_blacklist_call_test`.
#[test]
fn test_converted_blacklist_call_test() {
    check_converted_blacklist_call_test(".yaml");
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_converted_blacklist_import_data`.
#[test]
fn test_converted_blacklist_import_data() {
    check_converted_blacklist_import_data(".yaml");
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_converted_blacklist_import_test`.
#[test]
fn test_converted_blacklist_import_test() {
    check_converted_blacklist_import_test(".yaml");
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_converted_exclude`.
#[test]
fn test_converted_exclude() {
    check_converted_exclude(".yaml");
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_converted_exclude_blacklist`.
#[test]
fn test_converted_exclude_blacklist() {
    check_converted_exclude_blacklist(".yaml");
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_converted_include`.
#[test]
fn test_converted_include() {
    check_converted_include(".yaml");
}

/// Port of `tests/unit/core/test_config.py::TestConfigCompat::test_deprecation_message`.
#[test]
fn test_deprecation_message() {
    check_deprecation_message();
}

/// Port of `tests/unit/core/test_config.py::TestGetOption::test_levels`.
#[test]
fn test_levels() {
    let key = "wp08_levels_key";
    let subkey = "wp08_levels_subkey";
    let subvalue = "wp08-levels-subvalue";
    let contents = format!("\n{key}:\n    {subkey}: {subvalue}\n");
    let c = build_config(&contents, ".yaml").unwrap();

    assert_eq!(
        c.get_option(&format!("{key}.{subkey}")),
        Some(&ConfigValue::Str(subvalue.to_string()))
    );
}

/// Port of `tests/unit/core/test_config.py::TestGetOption::test_levels_not_exist`.
#[test]
fn test_levels_not_exist() {
    let key = "wp08_levels_key";
    let subkey = "wp08_levels_subkey";
    let subvalue = "wp08-levels-subvalue";
    let contents = format!("\n{key}:\n    {subkey}: {subvalue}\n");
    let c = build_config(&contents, ".yaml").unwrap();

    assert!(
        c.get_option("wp08_missing_key.wp08_missing_subkey")
            .is_none()
    );
}

/// Port of `tests/unit/core/test_config.py::TestGetSetting::test_not_exist`.
#[test]
fn test_not_exist() {
    let c = build_config("key: value", ".yaml").unwrap();

    assert!(c.get_setting("wp08_unknown_setting_name").is_none());
}

/// Port of `tests/unit/core/test_config.py::TestInit::test_file_does_not_exist`.
#[test]
fn test_file_does_not_exist() {
    let cfg_file = std::env::current_dir().unwrap().join("notafile");
    let path = cfg_file.to_str().unwrap();

    let e = BanditConfig::new(Some(path)).unwrap_err();
    assert!(e.message.contains(path));
}

/// Port of `tests/unit/core/test_config.py::TestInit::test_settings`.
#[test]
fn test_settings() {
    // `example_key = uuid.uuid4().hex`, `example_value = self.getUniqueString()`: any
    // arbitrary key/value pair exercises the same round-trip.
    let example_key = "wp08_settings_example_key";
    let example_value = "wp08-settings-example-value";
    let contents = format!("{example_key}: {example_value}");
    let c = build_config(&contents, ".yaml").unwrap();

    assert_eq!(c.get_setting("plugin_name_pattern"), Some("*.py"));
    let expected_raw = ConfigValue::Map(IndexMap::from_iter([(
        example_key.to_string(),
        ConfigValue::Str(example_value.to_string()),
    )]));
    assert_eq!(c.raw, expected_raw);
    assert_eq!(
        c.get_option(example_key),
        Some(&ConfigValue::Str(example_value.to_string()))
    );
}

/// Port of `tests/unit/core/test_config.py::TestInit::test_yaml_invalid`.
#[test]
fn test_yaml_invalid() {
    // Invalid because it starts a sequence and doesn't end it.
    let invalid_yaml = "- [ something";
    let mut f = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
    f.write_all(invalid_yaml.as_bytes()).unwrap();
    let path = f.path().to_str().unwrap().to_string();

    let e = BanditConfig::new(Some(&path)).unwrap_err();
    assert!(e.message.contains(&path));
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_bad_yaml`.
///
/// See the module doc: upstream's `test_bad_yaml` never honours `self.suffix`, so this is
/// the same YAML-parsing scenario as `test_bad_yaml`, not a TOML one.
#[test]
fn test_bad_yaml_toml() {
    check_bad_yaml();
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_blacklist_error`.
///
/// See the module doc: upstream's `test_blacklist_error` never loads a file, so this is
/// byte-identical to `test_blacklist_error`.
#[test]
fn test_blacklist_error_toml() {
    check_blacklist_error();
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_converted_blacklist_call_data`.
#[test]
fn test_converted_blacklist_call_data_toml() {
    check_converted_blacklist_call_data(".toml");
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_converted_blacklist_call_test`.
#[test]
fn test_converted_blacklist_call_test_toml() {
    check_converted_blacklist_call_test(".toml");
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_converted_blacklist_import_data`.
#[test]
fn test_converted_blacklist_import_data_toml() {
    check_converted_blacklist_import_data(".toml");
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_converted_blacklist_import_test`.
#[test]
fn test_converted_blacklist_import_test_toml() {
    check_converted_blacklist_import_test(".toml");
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_converted_exclude`.
#[test]
fn test_converted_exclude_toml() {
    check_converted_exclude(".toml");
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_converted_exclude_blacklist`.
#[test]
fn test_converted_exclude_blacklist_toml() {
    check_converted_exclude_blacklist(".toml");
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_converted_include`.
#[test]
fn test_converted_include_toml() {
    check_converted_include(".toml");
}

/// Port of `tests/unit/core/test_config.py::TestTomlConfig::test_deprecation_message`.
#[test]
fn test_deprecation_message_toml() {
    check_deprecation_message();
}

// --- shared bodies (see the `_toml` twins above) -----------------------------------------

fn check_bad_yaml() {
    let mut f = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
    f.write_all(b"[]").unwrap();
    let path = f.path().to_str().unwrap().to_string();

    let e = BanditConfig::new(Some(&path)).unwrap_err();
    assert!(e.message.contains("Error parsing file."));
}

fn check_blacklist_error() {
    let expected = " : Config file has an include or exclude reference to legacy test \
        'blacklist_imports' but no configuration data for it. Configuration data is \
        required for this test. Please consider switching to the new config file \
        format, the tool 'bandit-config-generator' can help you with this.";

    // Empirically (against the reference interpreter), only "blacklist_imports" matches
    // one of `validate`'s three checked keys ("blacklist_imports", "blacklist_import_func",
    // "blacklist_calls"); "blacklist_call" and "blacklist_imports_func" match none of them.
    for name in [
        "blacklist_call",
        "blacklist_imports",
        "blacklist_imports_func",
    ] {
        let raw = ConfigValue::Map(IndexMap::from_iter([(
            "profiles".to_string(),
            ConfigValue::Map(IndexMap::from_iter([(
                "test".to_string(),
                ConfigValue::Map(IndexMap::from_iter([(
                    "include".to_string(),
                    ConfigValue::List(vec![ConfigValue::Str(name.to_string())]),
                )])),
            )])),
        )]));
        let c = config_with_raw(raw);
        let result = c.validate("");

        if name == "blacklist_imports" {
            let e = result.unwrap_err();
            assert_eq!(e.message, expected);
        } else {
            assert!(result.is_ok());
        }
    }
}

fn check_converted_include(suffix: &str) {
    let c = sample_config(suffix);
    let p = c.profile("test_1").unwrap();

    assert_eq!(p.include, ids(&["B101", "B604"]));
    assert_eq!(p.exclude, IndexSet::new());
    // Upstream's own `profiles["test_1"]["blacklist"]` is `{}` (Python always sets it,
    // possibly empty). `Profile::blacklist` instead uses `None` for "no legacy override"
    // (an empty map here would look, to `TestSet::new`'s `Some`/`None` match, like an
    // override with nothing rather than no override — see the comment in
    // `convert_legacy_blacklist_tests` for the differential regression this avoids); the
    // *content* asserted here (no legacy data for this profile) is the same fact upstream
    // checks via `{"blacklist": {}, ...} == test`.
    assert_eq!(p.blacklist, None);
}

fn check_converted_exclude(suffix: &str) {
    let c = sample_config(suffix);
    let p = c.profile("test_4").unwrap();

    assert_eq!(p.exclude, ids(&["B101"]));
}

fn telnet_entry() -> BlacklistEntry {
    BlacklistEntry {
        name: "telnet".into(),
        id: "LEGACY".into(),
        cwe: Cwe::NOTSET,
        qualnames: vec!["telnetlib".into()],
        message: "{name} is considered insecure.".into(),
        level: "HIGH".into(),
    }
}

fn pickle_entry() -> BlacklistEntry {
    BlacklistEntry {
        name: "pickle".into(),
        id: "LEGACY".into(),
        cwe: Cwe::NOTSET,
        qualnames: vec!["pickle.loads".into()],
        message: "{name} library appears to be in use.".into(),
        level: "MEDIUM".into(),
    }
}

fn check_converted_blacklist_call_data(suffix: &str) {
    let c = sample_config(suffix);
    let p = c.profile("test_2").unwrap();
    let blacklist = p.blacklist.unwrap();

    // upstream: convert_legacy_blacklist_tests's argument swap — the "blacklist_calls"
    // profile is populated with the *blacklist_imports* config data (telnet).
    assert_eq!(blacklist.get("Call"), Some(&vec![telnet_entry()]));
}

fn check_converted_blacklist_import_data(suffix: &str) {
    let c = sample_config(suffix);
    let p = c.profile("test_3").unwrap();
    let blacklist = p.blacklist.unwrap();

    // upstream: convert_legacy_blacklist_tests's argument swap — the "blacklist_imports"
    // profile is populated with the *blacklist_calls* config data (pickle).
    let expected = vec![pickle_entry()];
    assert_eq!(blacklist.get("Call"), Some(&expected));
    assert_eq!(blacklist.get("Import"), Some(&expected));
    assert_eq!(blacklist.get("ImportFrom"), Some(&expected));
}

fn check_converted_blacklist_call_test(suffix: &str) {
    let c = sample_config(suffix);
    let p = c.profile("test_2").unwrap();

    assert_eq!(p.include, ids(&["B001"]));
}

fn check_converted_blacklist_import_test(suffix: &str) {
    let c = sample_config(suffix);
    let p = c.profile("test_3").unwrap();

    assert_eq!(p.include, ids(&["B001"]));
}

fn check_converted_exclude_blacklist(suffix: &str) {
    let c = sample_config(suffix);
    let p = c.profile("test_5").unwrap();

    assert_eq!(p.exclude, ids(&["B001"]));
}

fn check_deprecation_message() {
    let raw = ConfigValue::Map(IndexMap::from_iter([(
        "profiles".to_string(),
        ConfigValue::Map(IndexMap::new()),
    )]));
    let c = config_with_raw(raw);

    let (result, entries) = banditrs::log::with_buffer(|| c.validate(""));
    assert!(result.is_ok());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, banditrs::log::Level::Warning);
    assert_eq!(
        entries[0].message,
        "Config file '' contains deprecated legacy config data. Please consider \
         upgrading to the new config format. The tool 'bandit-config-generator' can \
         help you with this. Support for legacy configs will be removed in a future \
         bandit version."
    );
}
