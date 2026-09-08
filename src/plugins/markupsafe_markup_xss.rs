//! Port of `bandit/plugins/markupsafe_markup_xss.py` — see docs/spec/plugins.md.

use ruff_python_ast::Expr;

use crate::ast::qualname::call_name;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

fn is_constant(e: &Expr) -> bool {
    matches!(
        e,
        Expr::StringLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NoneLiteral(_)
            | Expr::BytesLiteral(_)
            | Expr::EllipsisLiteral(_)
    )
}

/// `markupsafe_markup_xss` (B704).
pub fn markupsafe_markup_xss(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let qual = ctx.call_function_name_qual().unwrap_or("");
    if qual != "markupsafe.Markup" && qual != "flask.Markup" {
        let extend = cfg.markupsafe_xss.extend_markup_names.items_or_empty()?;
        if !extend.iter().any(|n| n == qual) {
            return Ok(None);
        }
    }
    let Some(call) = ctx.call else {
        return Ok(None);
    };
    let args = &call.arguments.args;
    if args.is_empty() || is_constant(&args[0]) {
        return Ok(None);
    }
    let allowed = cfg.markupsafe_xss.allowed_calls.items_or_empty()?;
    if !allowed.is_empty()
        && let Expr::Call(inner) = &args[0]
    {
        let name = call_name(inner, ctx.import_aliases);
        if allowed.iter().any(|a| a == &name) {
            return Ok(None);
        }
    }
    Ok(Some(IssueDraft::new(
        Rank::Medium,
        Rank::High,
        Cwe::XSS,
        format!(
            "Potential XSS with ``{qual}`` detected. Do not use ``{}`` on untrusted data.",
            ctx.call_function_name().unwrap_or("")
        ),
    )))
}
