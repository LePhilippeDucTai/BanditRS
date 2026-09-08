//! Configuration files (port of `bandit/core/config.py`). See PLAN.md (M3/M7)
//! and docs/spec/core.md §6 for the exact semantics.
//!
//! Status: data model and public API defined; loading, validation and the
//! legacy conversion are stubs.

use std::fmt;

use indexmap::{IndexMap, IndexSet};

use crate::core::blacklist::BlacklistEntry;
use crate::core::registry;

/// A loaded configuration value (YAML 1.1 / TOML), insertion ordered.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<ConfigValue>),
    Map(IndexMap<String, ConfigValue>),
}

impl ConfigValue {
    pub fn as_map(&self) -> Option<&IndexMap<String, ConfigValue>> {
        match self {
            ConfigValue::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[ConfigValue]> {
        match self {
            ConfigValue::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Python truthiness (`get_option` returns `None` for falsy levels).
    pub fn truthy(&self) -> bool {
        match self {
            ConfigValue::Null => false,
            ConfigValue::Bool(b) => *b,
            ConfigValue::Int(i) => *i != 0,
            ConfigValue::Float(f) => *f != 0.0,
            ConfigValue::Str(s) => !s.is_empty(),
            ConfigValue::List(l) => !l.is_empty(),
            ConfigValue::Map(m) => !m.is_empty(),
        }
    }

    /// Python `str(value)` of a scalar (used for list items in profiles).
    pub fn py_str(&self) -> String {
        match self {
            ConfigValue::Null => "None".into(),
            ConfigValue::Bool(b) => {
                if *b {
                    "True".into()
                } else {
                    "False".into()
                }
            }
            ConfigValue::Int(i) => i.to_string(),
            ConfigValue::Float(f) => crate::ast::literal::repr_float(*f),
            ConfigValue::Str(s) => s.clone(),
            ConfigValue::List(_) | ConfigValue::Map(_) => format!("{self:?}"),
        }
    }
}

/// `utils.ConfigError`: `message = f"{config_file} : {message}"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub config_file: String,
    pub message: String,
}

impl ConfigError {
    pub fn new(message: &str, config_file: &str) -> ConfigError {
        ConfigError {
            config_file: config_file.to_string(),
            message: format!("{config_file} : {message}"),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

/// `utils.ProfileNotFound`: `"Unable to find profile ({profile}) in config file: {config_file}"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileNotFound {
    pub config_file: String,
    pub profile: String,
}

impl fmt::Display for ProfileNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unable to find profile ({}) in config file: {}",
            self.profile, self.config_file
        )
    }
}

/// A test profile (`include`/`exclude` sets plus optional legacy blacklist
/// data produced by the legacy config conversion).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Profile {
    pub include: IndexSet<String>,
    pub exclude: IndexSet<String>,
    /// Legacy `blacklist` data: node type (`Call`, `Import`, `ImportFrom`) → entries.
    pub blacklist: Option<IndexMap<String, Vec<BlacklistEntry>>>,
}

/// The loaded configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct BanditConfig {
    /// Path of the config file (`None` when defaults are used).
    pub path: Option<String>,
    /// Raw configuration tree (`_config`).
    pub raw: ConfigValue,
    /// `plugin_name_pattern` setting.
    pub plugin_name_pattern: String,
}

impl Default for BanditConfig {
    /// `BanditConfig()` without a file: `{"plugin_name_pattern": "*.py", "include": ["*.py", "*.pyw"]}`.
    fn default() -> BanditConfig {
        let mut m = IndexMap::new();
        m.insert(
            "plugin_name_pattern".to_string(),
            ConfigValue::Str("*.py".into()),
        );
        m.insert(
            "include".to_string(),
            ConfigValue::List(vec![
                ConfigValue::Str("*.py".into()),
                ConfigValue::Str("*.pyw".into()),
            ]),
        );
        BanditConfig {
            path: None,
            raw: ConfigValue::Map(m),
            plugin_name_pattern: "*.py".into(),
        }
    }
}

impl BanditConfig {
    /// `BanditConfig(config_file)`.
    ///
    /// TODO(M3/M7): read the file (`OSError` → `ConfigError("Could not read config file.")`),
    /// parse YAML (`pycompat::yaml_load`, `safe_load` semantics) or TOML
    /// (`.toml` suffix, `[tool.bandit]` table; parse errors →
    /// `ConfigError("Error parsing file.")`), run `validate()` (legacy
    /// `blacklist_*` references without data → the long "Config file has an
    /// include or exclude reference to legacy test '%s' but no configuration
    /// data for it. ..." message; `profiles` present → deprecation warning
    /// `"Config file '%s' contains deprecated legacy config data. Please
    /// consider upgrading to the new config format. The tool
    /// 'bandit-config-generator' can help you with this. Support for legacy
    /// configs will be removed in a future bandit version."` logged with
    /// the config path as argument), reject non-mapping roots
    /// (`"Error parsing file."`), then `convert_legacy_config()` and
    /// `_init_settings()`. Note: a loaded file replaces the defaults entirely
    /// (no `include` default merged in).
    pub fn new(config_file: Option<&str>) -> Result<BanditConfig, ConfigError> {
        let Some(path) = config_file else {
            return Ok(BanditConfig::default());
        };
        let text = std::fs::read_to_string(path)
            .map_err(|_| ConfigError::new("Could not read config file.", path))?;

        let raw = if path.ends_with(".toml") {
            let table: toml::Table = text.parse().map_err(|e| {
                crate::log_error!("config", "{}", e);
                ConfigError::new("Error parsing file.", path)
            })?;
            let bandit = table
                .get("tool")
                .and_then(|t| t.get("bandit"))
                .cloned()
                .unwrap_or(toml::Value::Table(toml::Table::new()));
            toml_to_config_value(&bandit)
        } else {
            crate::pycompat::yaml_load::safe_load(&text).map_err(|e| {
                crate::log_error!("config", "{}", e);
                ConfigError::new("Error parsing file.", path)
            })?
        };

        if !matches!(raw, ConfigValue::Map(_)) {
            return Err(ConfigError::new("Error parsing file.", path));
        }
        let mut config = BanditConfig {
            path: Some(path.to_string()),
            raw,
            plugin_name_pattern: String::new(),
        };

        if config.get_option("profiles").is_some() {
            crate::log_warning!(
                "config",
                "Config file '{}' contains deprecated legacy config data. Please consider upgrading to the new config format. The tool 'bandit-config-generator' can help you with this. Support for legacy configs will be removed in a future bandit version.",
                path
            );
        }

        config.plugin_name_pattern = config
            .get_option("plugin_name_pattern")
            .and_then(ConfigValue::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| "*.py".to_string());

        Ok(config)
    }

    /// `get_option("a.b.c")`: walk the tree; `None` when a level is missing
    /// or falsy.
    pub fn get_option(&self, option: &str) -> Option<&ConfigValue> {
        let mut cur = &self.raw;
        for part in option.split('.') {
            let m = cur.as_map()?;
            cur = m.get(part)?;
            if !cur.truthy() {
                return None;
            }
        }
        Some(cur)
    }

    /// `get_setting(name)` (only `plugin_name_pattern` exists).
    pub fn get_setting(&self, name: &str) -> Option<&str> {
        match name {
            "plugin_name_pattern" => Some(&self.plugin_name_pattern),
            _ => None,
        }
    }

    /// `config["profiles"][name]` as converted by `convert_names_to_ids`
    /// (test names in `include`/`exclude` resolved to ids, unknown names
    /// left unchanged). Legacy `blacklist_calls`/`blacklist_imports` data
    /// (`convert_legacy_blacklist_*`) is not yet converted — TODO(M7+):
    /// keep the upstream argument swap between `bad_calls` and
    /// `bad_imports`, `tests/unit/core/test_config.py` depends on it.
    pub fn profile(&self, name: &str) -> Option<Profile> {
        let profiles = self.get_option("profiles")?;
        let prof = profiles.as_map()?.get(name)?;
        let to_ids = |key: &str| -> IndexSet<String> {
            prof.as_map()
                .and_then(|m| m.get(key))
                .and_then(ConfigValue::as_list)
                .map(|l| {
                    l.iter()
                        .map(ConfigValue::py_str)
                        .map(|n| registry::get_test_id(&n).map(str::to_string).unwrap_or(n))
                        .collect()
                })
                .unwrap_or_default()
        };
        Some(Profile {
            include: to_ids("include"),
            exclude: to_ids("exclude"),
            blacklist: None,
        })
    }

    /// Profile built from the `tests` / `skips` options (`_get_profile`
    /// without `-p`).
    pub fn default_profile(&self) -> Profile {
        let to_set = |v: Option<&ConfigValue>| -> IndexSet<String> {
            v.and_then(ConfigValue::as_list)
                .map(|l| l.iter().map(ConfigValue::py_str).collect())
                .unwrap_or_default()
        };
        Profile {
            include: to_set(self.get_option("tests")),
            exclude: to_set(self.get_option("skips")),
            blacklist: None,
        }
    }
}

/// `tomllib.load(f).get("tool", {}).get("bandit", {})`, converted to a
/// `ConfigValue` tree (map key order preserved).
fn toml_to_config_value(v: &toml::Value) -> ConfigValue {
    match v {
        toml::Value::String(s) => ConfigValue::Str(s.clone()),
        toml::Value::Integer(i) => ConfigValue::Int(*i),
        toml::Value::Float(f) => ConfigValue::Float(*f),
        toml::Value::Boolean(b) => ConfigValue::Bool(*b),
        toml::Value::Datetime(d) => ConfigValue::Str(d.to_string()),
        toml::Value::Array(items) => {
            ConfigValue::List(items.iter().map(toml_to_config_value).collect())
        }
        toml::Value::Table(map) => ConfigValue::Map(
            map.iter()
                .map(|(k, v)| (k.clone(), toml_to_config_value(v)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_get_option() {
        let c = BanditConfig::default();
        assert_eq!(c.get_setting("plugin_name_pattern"), Some("*.py"));
        assert_eq!(c.get_setting("other"), None);
        let inc = c.get_option("include").unwrap().as_list().unwrap();
        assert_eq!(inc.len(), 2);
        assert!(c.get_option("a.b").is_none());
        let mut m = IndexMap::new();
        let mut inner = IndexMap::new();
        inner.insert("b".to_string(), ConfigValue::Int(3));
        m.insert("a".to_string(), ConfigValue::Map(inner));
        let c = BanditConfig {
            path: None,
            raw: ConfigValue::Map(m),
            plugin_name_pattern: "*.py".into(),
        };
        assert_eq!(c.get_option("a.b"), Some(&ConfigValue::Int(3)));
        assert!(c.get_option("a.c").is_none());
    }

    #[test]
    fn missing_file_errors() {
        let e = BanditConfig::new(Some("/nonexistent/nonexistent.yml")).unwrap_err();
        assert_eq!(
            e.message,
            "/nonexistent/nonexistent.yml : Could not read config file."
        );
    }

    #[test]
    fn loads_yaml_and_toml_and_profile() {
        let dir = tempfile::tempdir().unwrap();
        let yml = dir.path().join("bandit.yaml");
        std::fs::write(&yml, "assert_used:\n  skips: ['*_test.py']\nprofiles:\n  test:\n    include: [start_process_with_a_shell]\n    exclude: []\n").unwrap();
        let c = BanditConfig::new(Some(yml.to_str().unwrap())).unwrap();
        assert_eq!(
            c.get_option("assert_used.skips")
                .unwrap()
                .as_list()
                .unwrap()
                .len(),
            1
        );
        let p = c.profile("test").unwrap();
        assert!(p.include.contains("B605"));
        assert!(c.profile("missing").is_none());

        let toml_path = dir.path().join("bandit.toml");
        std::fs::write(&toml_path, "[tool.bandit]\nexclude_dirs = [\"tests\"]\n").unwrap();
        let c = BanditConfig::new(Some(toml_path.to_str().unwrap())).unwrap();
        let list = c.get_option("exclude_dirs").unwrap().as_list().unwrap();
        assert_eq!(list[0].as_str(), Some("tests"));
    }

    #[test]
    fn invalid_yaml_errors() {
        let dir = tempfile::tempdir().unwrap();
        let yml = dir.path().join("bad.yaml");
        std::fs::write(&yml, "a: [\n").unwrap();
        let e = BanditConfig::new(Some(yml.to_str().unwrap())).unwrap_err();
        assert!(e.message.ends_with("Error parsing file."));
    }
}
