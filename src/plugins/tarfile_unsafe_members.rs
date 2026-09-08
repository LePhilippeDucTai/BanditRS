//! Port of `bandit/plugins/tarfile_unsafe_members.py` — see docs/spec/plugins.md.

use ruff_python_ast::Expr;

use crate::ast::literal::{PyErr, node_repr};
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

fn is_filter_data(expr: &Expr) -> bool {
    matches!(expr, Expr::StringLiteral(s) if s.value.to_str() == "data")
}

fn get_members_issue(arg: &Expr) -> PluginResult {
    let (kind, repr): (&str, String) = match arg {
        Expr::Call(call) => match &*call.func {
            Expr::Name(n) => ("Function", format!("'{}'", n.id.as_str())),
            _ => return Err(PyErr::attribute_error("'Attribute' object has no attribute 'id'")),
        },
        Expr::Name(n) => ("Other", format!("'{}'", n.id.as_str())),
        other => ("Other", node_repr(other)),
    };
    let members_repr = format!("{{'{kind}': {repr}}}");
    if kind == "Function" {
        Ok(Some(IssueDraft::new(
            Rank::Low,
            Rank::Low,
            Cwe::PATH_TRAVERSAL,
            format!(
                "Usage of tarfile.extractall(members=function(tarfile)). Make sure your function properly discards dangerous members {members_repr})."
            ),
        )))
    } else {
        Ok(Some(IssueDraft::new(
            Rank::Medium,
            Rank::Medium,
            Cwe::PATH_TRAVERSAL,
            format!("Found tarfile.extractall(members=?) but couldn't identify the type of members. Check if the members were properly validated {members_repr})."),
        )))
    }
}

/// `tarfile_unsafe_members` (B202).
pub fn tarfile_unsafe_members(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !ctx.is_module_imported_exact("tarfile") || !ctx.call_function_name().unwrap_or("").contains("extractall") {
        return Ok(None);
    }
    let call = ctx.call.expect("Call context always carries a call");
    if let Some(kw) = call.arguments.keywords.iter().find(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == "filter")) {
        if is_filter_data(&kw.value) {
            return Ok(None);
        }
    }
    if let Some(kw) = call.arguments.keywords.iter().find(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == "members")) {
        return get_members_issue(&kw.value);
    }
    Ok(Some(IssueDraft::new(
        Rank::High,
        Rank::High,
        Cwe::PATH_TRAVERSAL,
        "tarfile.extractall used without any validation. Please check and discard dangerous members.",
    )))
}
