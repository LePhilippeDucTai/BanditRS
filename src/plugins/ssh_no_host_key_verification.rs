//! Port of `bandit/plugins/ssh_no_host_key_verification.py` — see docs/spec/plugins.md.

use ruff_python_ast::Expr;

use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `ssh_no_host_key_verification` (B507).
pub fn ssh_no_host_key_verification(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !ctx.is_module_imported_like("paramiko") || ctx.call_function_name() != Some("set_missing_host_key_policy") {
        return Ok(None);
    }
    let Some(call) = ctx.call else { return Ok(None) };
    let Some(first) = call.arguments.args.first() else { return Ok(None) };
    let val: Option<&str> = match first {
        Expr::Attribute(a) => Some(a.attr.as_str()),
        Expr::Name(n) => Some(n.id.as_str()),
        Expr::Call(c) => match &*c.func {
            Expr::Attribute(a) => Some(a.attr.as_str()),
            Expr::Name(n) => Some(n.id.as_str()),
            _ => None,
        },
        _ => None,
    };
    if matches!(val, Some("AutoAddPolicy") | Some("WarningPolicy")) {
        return Ok(Some(
            IssueDraft::new(
                Rank::High,
                Rank::Medium,
                Cwe::IMPROPER_CERT_VALIDATION,
                "Paramiko call with policy set to automatically trust the unknown host key.",
            )
            .with_lineno(ctx.get_lineno_for_call_arg("set_missing_host_key_policy")),
        ));
    }
    Ok(None)
}
