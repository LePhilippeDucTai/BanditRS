//! Typed per-plugin configuration (`@test.takes_config` sections).
//!
//! Each section is read from the config file (`BanditConfig::get_option`)
//! at `TestSet` construction; an absent section falls back to the plugin's
//! `gen_config` defaults (listed below, verbatim from upstream). A present
//! but incomplete section reproduces the Python failure modes: a missing key
//! is a `KeyError`, a `null` value a `TypeError` when used as a list — the
//! plugin then returns `Err(PyErr)` and reports nothing.
//!
//! Status: defaults and `from_config` parsing are implemented.

use indexmap::IndexMap;

use crate::ast::literal::PyErr;
use crate::core::config::ConfigValue;

/// A list option with its Python failure states.
#[derive(Debug, Clone, PartialEq)]
pub enum ListOpt {
    /// Key absent (`KeyError` on access).
    Missing,
    /// Explicit `null` (`TypeError` when iterated / searched).
    Null,
    /// A list of string items (non-string items are kept as their `str()`).
    Items(Vec<String>),
}

impl ListOpt {
    /// `config.get(key, [])`: a missing key behaves like the empty-list
    /// default; an explicit `null` still raises (Python would try to iterate
    /// `None`).
    pub fn items_or_empty(&self) -> Result<&[String], PyErr> {
        match self {
            ListOpt::Missing => Ok(&[]),
            ListOpt::Null => Err(PyErr::type_error("'NoneType' object is not iterable")),
            ListOpt::Items(items) => Ok(items),
        }
    }

    /// `value in config[key]` with Python error semantics.
    pub fn contains_str(&self, key: &str, value: &str) -> Result<bool, PyErr> {
        match self {
            ListOpt::Missing => Err(PyErr::key_error(key)),
            ListOpt::Null => Err(PyErr::type_error("argument of type 'NoneType' is not iterable")),
            ListOpt::Items(items) => Ok(items.iter().any(|i| i == value)),
        }
    }

    pub fn items(&self, key: &str) -> Result<&[String], PyErr> {
        match self {
            ListOpt::Missing => Err(PyErr::key_error(key)),
            ListOpt::Null => Err(PyErr::type_error("'NoneType' object is not iterable")),
            ListOpt::Items(items) => Ok(items),
        }
    }

    pub fn of(items: &[&str]) -> ListOpt {
        ListOpt::Items(items.iter().map(|s| s.to_string()).collect())
    }
}

/// `shell_injection` (B602–B607, B609). `present` reproduces `if config:`
/// (an empty mapping is falsy).
#[derive(Debug, Clone, PartialEq)]
pub struct ShellInjectionConfig {
    pub present: bool,
    pub subprocess: ListOpt,
    pub shell: ListOpt,
    pub no_shell: ListOpt,
}

impl Default for ShellInjectionConfig {
    fn default() -> Self {
        ShellInjectionConfig {
            present: true,
            subprocess: ListOpt::of(&[
                "subprocess.Popen",
                "subprocess.call",
                "subprocess.check_call",
                "subprocess.check_output",
                "subprocess.run",
            ]),
            shell: ListOpt::of(&[
                "os.system",
                "os.popen",
                "os.popen2",
                "os.popen3",
                "os.popen4",
                "popen2.popen2",
                "popen2.popen3",
                "popen2.popen4",
                "popen2.Popen3",
                "popen2.Popen4",
                "commands.getoutput",
                "commands.getstatusoutput",
                "subprocess.getoutput",
                "subprocess.getstatusoutput",
            ]),
            no_shell: ListOpt::of(&[
                "os.execl",
                "os.execle",
                "os.execlp",
                "os.execlpe",
                "os.execv",
                "os.execve",
                "os.execvp",
                "os.execvpe",
                "os.spawnl",
                "os.spawnle",
                "os.spawnlp",
                "os.spawnlpe",
                "os.spawnv",
                "os.spawnve",
                "os.spawnvp",
                "os.spawnvpe",
                "os.startfile",
            ]),
        }
    }
}

/// `assert_used` (B101): `{"skips": []}`.
#[derive(Debug, Clone, PartialEq)]
pub struct AssertUsedConfig {
    pub skips: ListOpt,
}

impl Default for AssertUsedConfig {
    fn default() -> Self {
        AssertUsedConfig { skips: ListOpt::Items(vec![]) }
    }
}

/// `hardcoded_tmp_directory` (B108): `{"tmp_dirs": ["/tmp", "/var/tmp", "/dev/shm"]}`.
/// The plugin itself falls back to the defaults when the key is missing.
#[derive(Debug, Clone, PartialEq)]
pub struct TmpDirConfig {
    pub tmp_dirs: ListOpt,
}

impl Default for TmpDirConfig {
    fn default() -> Self {
        TmpDirConfig { tmp_dirs: ListOpt::of(&["/tmp", "/var/tmp", "/dev/shm"]) }
    }
}

/// `try_except_pass` / `try_except_continue` (B110/B112):
/// `{"check_typed_exception": False}`.
#[derive(Debug, Clone, PartialEq)]
pub struct TryExceptConfig {
    pub check_typed_exception: Result<bool, PyErr>,
}

impl Default for TryExceptConfig {
    fn default() -> Self {
        TryExceptConfig { check_typed_exception: Ok(false) }
    }
}

/// `ssl_with_bad_version` (B502/B503): `{"bad_protocol_versions": [...]}`.
#[derive(Debug, Clone, PartialEq)]
pub struct SslConfig {
    pub bad_protocol_versions: ListOpt,
}

impl Default for SslConfig {
    fn default() -> Self {
        SslConfig {
            bad_protocol_versions: ListOpt::of(&[
                "PROTOCOL_SSLv2",
                "SSLv2_METHOD",
                "SSLv23_METHOD",
                "PROTOCOL_SSLv3",
                "PROTOCOL_TLSv1",
                "SSLv3_METHOD",
                "TLSv1_METHOD",
                "PROTOCOL_TLSv1_1",
                "TLSv1_1_METHOD",
            ]),
        }
    }
}

/// `weak_cryptographic_key` (B505).
#[derive(Debug, Clone, PartialEq)]
pub struct WeakKeyConfig {
    pub weak_key_size_dsa_high: Result<i64, PyErr>,
    pub weak_key_size_dsa_medium: Result<i64, PyErr>,
    pub weak_key_size_rsa_high: Result<i64, PyErr>,
    pub weak_key_size_rsa_medium: Result<i64, PyErr>,
    pub weak_key_size_ec_high: Result<i64, PyErr>,
    pub weak_key_size_ec_medium: Result<i64, PyErr>,
}

impl Default for WeakKeyConfig {
    fn default() -> Self {
        WeakKeyConfig {
            weak_key_size_dsa_high: Ok(1024),
            weak_key_size_dsa_medium: Ok(2048),
            weak_key_size_rsa_high: Ok(1024),
            weak_key_size_rsa_medium: Ok(2048),
            weak_key_size_ec_high: Ok(160),
            weak_key_size_ec_medium: Ok(224),
        }
    }
}

/// `markupsafe_xss` (B704): `{"extend_markup_names": [], "allowed_calls": []}`.
/// The plugin uses `config.get(...)`, so missing keys behave like `[]`.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkupSafeConfig {
    pub extend_markup_names: ListOpt,
    pub allowed_calls: ListOpt,
}

impl Default for MarkupSafeConfig {
    fn default() -> Self {
        MarkupSafeConfig { extend_markup_names: ListOpt::Items(vec![]), allowed_calls: ListOpt::Items(vec![]) }
    }
}

/// All plugin configurations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PluginConfigs {
    pub shell_injection: ShellInjectionConfig,
    pub assert_used: AssertUsedConfig,
    pub hardcoded_tmp_directory: TmpDirConfig,
    pub try_except_pass: TryExceptConfig,
    pub try_except_continue: TryExceptConfig,
    pub ssl_with_bad_version: SslConfig,
    pub weak_cryptographic_key: WeakKeyConfig,
    pub markupsafe_xss: MarkupSafeConfig,
}

/// `config[key]` where `config` is the section map (raises `KeyError` when the
/// key is absent, `TypeError` when the value is `null` and iterated/searched).
fn list_opt(map: &IndexMap<String, ConfigValue>, key: &str) -> ListOpt {
    match map.get(key) {
        None => ListOpt::Missing,
        Some(ConfigValue::Null) => ListOpt::Null,
        Some(v) => ListOpt::Items(v.as_list().map(|l| l.iter().map(ConfigValue::py_str).collect()).unwrap_or_else(|| vec![v.py_str()])),
    }
}

/// `config[key]` truthiness (used by `not config["check_typed_exception"]`).
fn bool_opt(map: &IndexMap<String, ConfigValue>, key: &str) -> Result<bool, PyErr> {
    match map.get(key) {
        None => Err(PyErr::key_error(key)),
        Some(v) => Ok(v.truthy()),
    }
}

/// `config[key]` as an integer (`KeyError` when absent).
fn int_opt(map: &IndexMap<String, ConfigValue>, key: &str) -> Result<i64, PyErr> {
    match map.get(key) {
        None => Err(PyErr::key_error(key)),
        Some(ConfigValue::Int(i)) => Ok(*i),
        Some(ConfigValue::Float(f)) => Ok(*f as i64),
        Some(v) => Err(PyErr::type_error(format!("'{}' object cannot be interpreted as an integer", v.py_str()))),
    }
}

impl PluginConfigs {
    /// Build from a loaded configuration: each section overrides the
    /// defaults when present (`BanditTestSet._load_tests`). A present section
    /// entirely replaces the plugin's `gen_config` defaults (no merging), so a
    /// field missing from a present section reproduces the Python `KeyError`.
    pub fn from_config(config: &crate::core::config::BanditConfig) -> PluginConfigs {
        let section = |key: &str| config.get_option(key).and_then(ConfigValue::as_map);

        let shell_injection = match section("shell_injection") {
            None => ShellInjectionConfig::default(),
            Some(m) => ShellInjectionConfig {
                present: true,
                subprocess: list_opt(m, "subprocess"),
                shell: list_opt(m, "shell"),
                no_shell: list_opt(m, "no_shell"),
            },
        };
        let assert_used = match section("assert_used") {
            None => AssertUsedConfig::default(),
            Some(m) => AssertUsedConfig { skips: list_opt(m, "skips") },
        };
        let hardcoded_tmp_directory = match section("hardcoded_tmp_directory") {
            None => TmpDirConfig::default(),
            Some(m) => TmpDirConfig { tmp_dirs: list_opt(m, "tmp_dirs") },
        };
        let try_except_pass = match section("try_except_pass") {
            None => TryExceptConfig::default(),
            Some(m) => TryExceptConfig { check_typed_exception: bool_opt(m, "check_typed_exception") },
        };
        let try_except_continue = match section("try_except_continue") {
            None => TryExceptConfig::default(),
            Some(m) => TryExceptConfig { check_typed_exception: bool_opt(m, "check_typed_exception") },
        };
        let ssl_with_bad_version = match section("ssl_with_bad_version") {
            None => SslConfig::default(),
            Some(m) => SslConfig { bad_protocol_versions: list_opt(m, "bad_protocol_versions") },
        };
        let weak_cryptographic_key = match section("weak_cryptographic_key") {
            None => WeakKeyConfig::default(),
            Some(m) => WeakKeyConfig {
                weak_key_size_dsa_high: int_opt(m, "weak_key_size_dsa_high"),
                weak_key_size_dsa_medium: int_opt(m, "weak_key_size_dsa_medium"),
                weak_key_size_rsa_high: int_opt(m, "weak_key_size_rsa_high"),
                weak_key_size_rsa_medium: int_opt(m, "weak_key_size_rsa_medium"),
                weak_key_size_ec_high: int_opt(m, "weak_key_size_ec_high"),
                weak_key_size_ec_medium: int_opt(m, "weak_key_size_ec_medium"),
            },
        };
        let markupsafe_xss = match section("markupsafe_xss") {
            None => MarkupSafeConfig::default(),
            Some(m) => MarkupSafeConfig {
                extend_markup_names: match m.get("extend_markup_names") {
                    None => ListOpt::Items(vec![]),
                    Some(_) => list_opt(m, "extend_markup_names"),
                },
                allowed_calls: match m.get("allowed_calls") {
                    None => ListOpt::Items(vec![]),
                    Some(_) => list_opt(m, "allowed_calls"),
                },
            },
        };
        PluginConfigs {
            shell_injection,
            assert_used,
            hardcoded_tmp_directory,
            try_except_pass,
            try_except_continue,
            ssl_with_bad_version,
            weak_cryptographic_key,
            markupsafe_xss,
        }
    }

    /// `gen_config(name)` defaults rendered as JSON-like values, in the
    /// order `bandit-config-generator` prints them (sorted keys).
    /// TODO(M8): used by `bandit-config-generator --show-defaults`.
    pub fn defaults_yaml() -> String {
        todo!("M8: render gen_config defaults with the PyYAML-compatible emitter")
    }
}
