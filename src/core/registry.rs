//! The plugin registry (port of `bandit/core/extension_loader.py` +
//! `setup.cfg` entry points). The table is complete; plugin bodies live in
//! `crate::plugins` (stubs until M3/M4).
//!
//! Order matters: it is the `setup.cfg` entry-point order, which fixes the
//! order of issues reported for a single node. The builtin `B001` blacklist
//! test is always appended last by `TestSet`.

use crate::ast::NodeKind;
use crate::core::blacklist;
use crate::plugins::PluginFn;

/// One plugin (`@test.test_id` + `@test.checks` + `@test.takes_config`).
#[derive(Clone, Copy)]
pub struct PluginDef {
    /// Test id, e.g. `B101`.
    pub id: &'static str,
    /// Entry-point name (`setup.cfg`), e.g. `hashlib_insecure_functions`.
    pub name: &'static str,
    /// Python function name (`Issue.test`, docs URL), e.g. `hashlib`.
    pub func_name: &'static str,
    /// Node kinds the plugin is registered for.
    pub checks: &'static [NodeKind],
    /// Config section consumed by the plugin (`@test.takes_config`).
    pub config_key: Option<&'static str>,
    pub func: PluginFn,
}

use NodeKind::{Assert, Call, ExceptHandler, File, FunctionDef, Str};

macro_rules! plugin {
    ($id:literal, $name:literal, $func:literal, [$($k:expr),*], $cfg:expr, $f:path) => {
        PluginDef { id: $id, name: $name, func_name: $func, checks: &[$($k),*], config_key: $cfg, func: $f }
    };
}

/// All plugins in `setup.cfg` order.
pub static PLUGINS: &[PluginDef] = &[
    plugin!("B201", "flask_debug_true", "flask_debug_true", [Call], None, crate::plugins::app_debug::flask_debug_true),
    plugin!("B101", "assert_used", "assert_used", [Assert], Some("assert_used"), crate::plugins::asserts::assert_used),
    plugin!("B501", "request_with_no_cert_validation", "request_with_no_cert_validation", [Call], None, crate::plugins::crypto_request_no_cert_validation::request_with_no_cert_validation),
    plugin!("B113", "request_without_timeout", "request_without_timeout", [Call], None, crate::plugins::request_without_timeout::request_without_timeout),
    plugin!("B102", "exec_used", "exec_used", [Call], None, crate::plugins::exec::exec_used),
    plugin!("B103", "set_bad_file_permissions", "set_bad_file_permissions", [Call], None, crate::plugins::general_bad_file_permissions::set_bad_file_permissions),
    plugin!("B104", "hardcoded_bind_all_interfaces", "hardcoded_bind_all_interfaces", [Str], None, crate::plugins::general_bind_all_interfaces::hardcoded_bind_all_interfaces),
    plugin!("B105", "hardcoded_password_string", "hardcoded_password_string", [Str], None, crate::plugins::general_hardcoded_password::hardcoded_password_string),
    plugin!("B106", "hardcoded_password_funcarg", "hardcoded_password_funcarg", [Call], None, crate::plugins::general_hardcoded_password::hardcoded_password_funcarg),
    plugin!("B107", "hardcoded_password_default", "hardcoded_password_default", [FunctionDef], None, crate::plugins::general_hardcoded_password::hardcoded_password_default),
    plugin!("B108", "hardcoded_tmp_directory", "hardcoded_tmp_directory", [Str], Some("hardcoded_tmp_directory"), crate::plugins::general_hardcoded_tmp::hardcoded_tmp_directory),
    plugin!("B601", "paramiko_calls", "paramiko_calls", [Call], None, crate::plugins::injection_paramiko::paramiko_calls),
    plugin!("B602", "subprocess_popen_with_shell_equals_true", "subprocess_popen_with_shell_equals_true", [Call], Some("shell_injection"), crate::plugins::injection_shell::subprocess_popen_with_shell_equals_true),
    plugin!("B603", "subprocess_without_shell_equals_true", "subprocess_without_shell_equals_true", [Call], Some("shell_injection"), crate::plugins::injection_shell::subprocess_without_shell_equals_true),
    plugin!("B604", "any_other_function_with_shell_equals_true", "any_other_function_with_shell_equals_true", [Call], Some("shell_injection"), crate::plugins::injection_shell::any_other_function_with_shell_equals_true),
    plugin!("B605", "start_process_with_a_shell", "start_process_with_a_shell", [Call], Some("shell_injection"), crate::plugins::injection_shell::start_process_with_a_shell),
    plugin!("B606", "start_process_with_no_shell", "start_process_with_no_shell", [Call], Some("shell_injection"), crate::plugins::injection_shell::start_process_with_no_shell),
    plugin!("B607", "start_process_with_partial_path", "start_process_with_partial_path", [Call], Some("shell_injection"), crate::plugins::injection_shell::start_process_with_partial_path),
    plugin!("B608", "hardcoded_sql_expressions", "hardcoded_sql_expressions", [Str], None, crate::plugins::injection_sql::hardcoded_sql_expressions),
    plugin!("B324", "hashlib_insecure_functions", "hashlib", [Call], None, crate::plugins::hashlib_insecure_functions::hashlib),
    plugin!("B609", "linux_commands_wildcard_injection", "linux_commands_wildcard_injection", [Call], Some("shell_injection"), crate::plugins::injection_wildcard::linux_commands_wildcard_injection),
    plugin!("B610", "django_extra_used", "django_extra_used", [Call], None, crate::plugins::django_sql_injection::django_extra_used),
    plugin!("B611", "django_rawsql_used", "django_rawsql_used", [Call], None, crate::plugins::django_sql_injection::django_rawsql_used),
    plugin!("B502", "ssl_with_bad_version", "ssl_with_bad_version", [Call], Some("ssl_with_bad_version"), crate::plugins::insecure_ssl_tls::ssl_with_bad_version),
    plugin!("B503", "ssl_with_bad_defaults", "ssl_with_bad_defaults", [FunctionDef], Some("ssl_with_bad_version"), crate::plugins::insecure_ssl_tls::ssl_with_bad_defaults),
    plugin!("B504", "ssl_with_no_version", "ssl_with_no_version", [Call], None, crate::plugins::insecure_ssl_tls::ssl_with_no_version),
    plugin!("B701", "jinja2_autoescape_false", "jinja2_autoescape_false", [Call], None, crate::plugins::jinja2_templates::jinja2_autoescape_false),
    plugin!("B702", "use_of_mako_templates", "use_of_mako_templates", [Call], None, crate::plugins::mako_templates::use_of_mako_templates),
    plugin!("B703", "django_mark_safe", "django_mark_safe", [Call], None, crate::plugins::django_xss::django_mark_safe),
    plugin!("B112", "try_except_continue", "try_except_continue", [ExceptHandler], Some("try_except_continue"), crate::plugins::try_except_continue::try_except_continue),
    plugin!("B110", "try_except_pass", "try_except_pass", [ExceptHandler], Some("try_except_pass"), crate::plugins::try_except_pass::try_except_pass),
    plugin!("B505", "weak_cryptographic_key", "weak_cryptographic_key", [Call], Some("weak_cryptographic_key"), crate::plugins::weak_cryptographic_key::weak_cryptographic_key),
    plugin!("B506", "yaml_load", "yaml_load", [Call], None, crate::plugins::yaml_load::yaml_load),
    plugin!("B507", "ssh_no_host_key_verification", "ssh_no_host_key_verification", [Call], None, crate::plugins::ssh_no_host_key_verification::ssh_no_host_key_verification),
    plugin!("B508", "snmp_insecure_version", "snmp_insecure_version_check", [Call], None, crate::plugins::snmp_security_check::snmp_insecure_version_check),
    plugin!("B509", "snmp_weak_cryptography", "snmp_crypto_check", [Call], None, crate::plugins::snmp_security_check::snmp_crypto_check),
    plugin!("B612", "logging_config_insecure_listen", "logging_config_insecure_listen", [Call], None, crate::plugins::logging_config_insecure_listen::logging_config_insecure_listen),
    plugin!("B202", "tarfile_unsafe_members", "tarfile_unsafe_members", [Call], None, crate::plugins::tarfile_unsafe_members::tarfile_unsafe_members),
    plugin!("B614", "pytorch_load", "pytorch_load", [Call], None, crate::plugins::pytorch_load::pytorch_load),
    plugin!("B613", "trojansource", "trojansource", [File], None, crate::plugins::trojansource::trojansource),
    plugin!("B704", "markupsafe_markup_xss", "markupsafe_markup_xss", [Call], Some("markupsafe_xss"), crate::plugins::markupsafe_markup_xss::markupsafe_markup_xss),
    plugin!("B615", "huggingface_unsafe_download", "huggingface_unsafe_download", [Call], None, crate::plugins::huggingface_unsafe_download::huggingface_unsafe_download),
];

/// The builtin test ids (`Manager.builtin`).
pub const BUILTIN: &[&str] = &["B001"];

/// `plugins_by_id`.
pub fn plugin_by_id(id: &str) -> Option<&'static PluginDef> {
    PLUGINS.iter().find(|p| p.id == id)
}

/// `plugins_by_name` (entry-point name).
pub fn plugin_by_name(name: &str) -> Option<&'static PluginDef> {
    PLUGINS.iter().find(|p| p.name == name)
}

/// `Manager.get_test_id(name)`: plugin name → id, else blacklist name → id.
pub fn get_test_id(name: &str) -> Option<&'static str> {
    if let Some(p) = plugin_by_name(name) {
        return Some(p.id);
    }
    blacklist::entry_by_name(name).map(|e| e.id)
}

/// `Manager.check_id(id)`: a plugin id, a blacklist id or a builtin id.
pub fn check_id(id: &str) -> bool {
    plugin_by_id(id).is_some() || blacklist::entry_by_id(id).is_some() || BUILTIN.contains(&id)
}

/// Resolve a `# nosec` token: a known id, or a known name mapped to its id.
pub fn resolve_test_token(token: &str) -> Option<String> {
    if check_id(token) {
        return Some(token.to_string());
    }
    get_test_id(token).map(str::to_string)
}

/// `(id, name)` pairs of every plugin and blacklist datum, for the CLI
/// epilog (`sorted(set(...))` is applied by the caller).
pub fn all_ids_and_names() -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<(&str, &str)> = PLUGINS.iter().map(|p| (p.id, p.name)).collect();
    v.extend(blacklist::all_entries().map(|e| (e.id, e.name)));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_consistent() {
        assert_eq!(PLUGINS.len(), 42);
        let mut ids: Vec<&str> = PLUGINS.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 42);
        assert_eq!(plugin_by_id("B324").unwrap().func_name, "hashlib");
        assert_eq!(get_test_id("md5"), Some("B303"));
        assert_eq!(get_test_id("subprocess_popen_with_shell_equals_true"), Some("B602"));
        assert!(check_id("B001") && check_id("B101") && check_id("B401") && !check_id("B999"));
        assert_eq!(resolve_test_token("import_subprocess").as_deref(), Some("B404"));
        assert_eq!(PLUGINS.iter().filter(|p| p.checks.contains(&NodeKind::Call)).count(), 32);
        assert_eq!(PLUGINS.iter().filter(|p| p.checks.contains(&NodeKind::Str)).count(), 4);
    }
}
