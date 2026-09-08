//! Port of `bandit/plugins/injection_wildcard.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::{ListOpt, PluginConfigs};
use crate::plugins::PluginResult;

const VULNERABLE_FUNCS: &[&str] = &["chown", "chmod", "tar", "rsync"];

/// `linux_commands_wildcard_injection` (B609, shares the `shell_injection` config).
pub fn linux_commands_wildcard_injection(
    ctx: &Context<'_, '_>,
    cfg: &PluginConfigs,
) -> PluginResult {
    let shell_cfg = &cfg.shell_injection;
    if matches!(shell_cfg.shell, ListOpt::Missing)
        || matches!(shell_cfg.subprocess, ListOpt::Missing)
    {
        return Ok(None);
    }
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let in_shell = shell_cfg.shell.items("shell")?.iter().any(|s| s == qual);
    let in_subprocess_with_shell = shell_cfg
        .subprocess
        .items("subprocess")?
        .iter()
        .any(|s| s == qual)
        && ctx.check_call_arg_is("shell", &PyValue::str("True"))? == Some(true);
    if !(in_shell || in_subprocess_with_shell) {
        return Ok(None);
    }
    if ctx.call_args_count().unwrap_or(0) < 1 {
        return Ok(None);
    }
    let argument_string = match ctx.get_call_arg_at_position(0)? {
        Some(PyValue::List(items)) => items
            .iter()
            .map(|i| format!(" {}", i.py_str()))
            .collect::<String>(),
        Some(PyValue::Str(s)) => s.to_string(),
        _ => String::new(),
    };
    if !argument_string.is_empty() {
        for f in VULNERABLE_FUNCS {
            if argument_string.contains(f) && argument_string.contains('*') {
                return Ok(Some(
                    IssueDraft::new(
                        Rank::High,
                        Rank::Medium,
                        Cwe::IMPROPER_WILDCARD_NEUTRALIZATION,
                        format!("Possible wildcard injection in call: {qual}"),
                    )
                    .with_lineno(ctx.get_lineno_for_call_arg("shell")),
                ));
            }
        }
    }
    Ok(None)
}
