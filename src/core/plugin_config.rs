//! Typed per-plugin configuration (`@test.takes_config` sections).
//!
//! Each section is read from the config file (`BanditConfig::get_option`)
//! at `TestSet` construction; an absent section falls back to the plugin's
//! `gen_config` defaults (listed below, verbatim from upstream). A present
//! but incomplete section reproduces the Python failure modes: a missing key
//! is a `KeyError`, a `null` value a `TypeError` when used as a list — the
//! plugin then returns `Err(PyErr)` and reports nothing.
//!
//! Status: defaults complete; parsing from `ConfigValue` is a stub (M3).

use crate::ast::literal::PyErr;

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

impl PluginConfigs {
    /// Build from a loaded configuration: each section overrides the
    /// defaults when present (`BanditTestSet._load_tests`).
    /// TODO(M3): read `config.get_option(key)` for every section and convert
    /// `ConfigValue` lists/bools/ints into the typed fields, keeping
    /// `Missing`/`Null` states.
    pub fn from_config(_config: &crate::core::config::BanditConfig) -> PluginConfigs {
        todo!("M3: PluginConfigs::from_config")
    }

    /// `gen_config(name)` defaults rendered as JSON-like values, in the
    /// order `bandit-config-generator` prints them (sorted keys).
    /// TODO(M8): used by `bandit-config-generator --show-defaults`.
    pub fn defaults_yaml() -> String {
        todo!("M8: render gen_config defaults with the PyYAML-compatible emitter")
    }
}
