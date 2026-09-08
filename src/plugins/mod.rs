//! Plugins (port of `bandit/plugins/*.py`), one module per upstream file.
//!
//! Every plugin is a plain function `fn(&Context, &PluginConfigs) -> PluginResult`
//! returning `Ok(None)` (no finding), `Ok(Some(draft))` (a finding whose
//! location defaults are filled in by the tester) or `Err(PyErr)` where the
//! Python implementation would raise (the tester logs
//! `Bandit internal error running: ...` and drops the finding).
//!
//! The exact detection logic, messages, severities, CWEs and config defaults
//! of each plugin are specified verbatim in docs/spec/plugins.md. Status: all
//! 42 plugins are implemented (PLAN.md milestones M3/M4); `django_xss`'s
//! `DeepAssignation` only covers straight-line assignments (see its module
//! doc and DEVIATIONS.md).

use crate::ast::literal::PyErr;
use crate::core::context::Context;
use crate::core::issue::IssueDraft;
use crate::core::plugin_config::PluginConfigs;

/// Result of a plugin invocation.
pub type PluginResult = Result<Option<IssueDraft>, PyErr>;

/// Signature shared by all plugins.
pub type PluginFn = for<'a, 'w> fn(&Context<'a, 'w>, &PluginConfigs) -> PluginResult;

pub mod app_debug;
pub mod asserts;
pub mod crypto_request_no_cert_validation;
pub mod request_without_timeout;
pub mod exec;
pub mod general_bad_file_permissions;
pub mod general_bind_all_interfaces;
pub mod general_hardcoded_password;
pub mod general_hardcoded_tmp;
pub mod injection_paramiko;
pub mod injection_shell;
pub mod injection_sql;
pub mod hashlib_insecure_functions;
pub mod injection_wildcard;
pub mod django_sql_injection;
pub mod insecure_ssl_tls;
pub mod jinja2_templates;
pub mod mako_templates;
pub mod django_xss;
pub mod try_except_continue;
pub mod try_except_pass;
pub mod weak_cryptographic_key;
pub mod yaml_load;
pub mod ssh_no_host_key_verification;
pub mod snmp_security_check;
pub mod logging_config_insecure_listen;
pub mod tarfile_unsafe_members;
pub mod pytorch_load;
pub mod trojansource;
pub mod markupsafe_markup_xss;
pub mod huggingface_unsafe_download;
