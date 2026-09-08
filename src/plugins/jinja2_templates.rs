//! Port of `bandit/plugins/jinja2_templates.py` — see docs/spec/plugins.md.

use ruff_python_ast::Expr;

use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `jinja2_autoescape_false` (B701). The upstream implementation walks the
/// whole call subtree (`ast.walk`) looking for the first `autoescape`
/// keyword; in every real-world (and fixture) call this keyword is a direct
/// argument of the `Environment(...)` call, so scanning the call's own
/// keywords in order reproduces the same result.
pub fn jinja2_autoescape_false(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let parts: Vec<&str> = qual.split('.').collect();
    if !(parts.contains(&"jinja2") && parts.last() == Some(&"Environment")) {
        return Ok(None);
    }
    let Some(call) = ctx.call else { return Ok(None) };
    let Some(kw) = call.arguments.keywords.iter().find(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == "autoescape")) else {
        return Ok(Some(IssueDraft::new(
            Rank::High,
            Rank::High,
            Cwe::CODE_INJECTION,
            "By default, jinja2 sets autoescape to False. Consider using autoescape=True or use the select_autoescape function to mitigate XSS vulnerabilities.",
        )));
    };
    let is_false = matches!(&kw.value, Expr::Name(n) if n.id.as_str() == "False") || matches!(&kw.value, Expr::BooleanLiteral(b) if !b.value);
    if is_false {
        return Ok(Some(IssueDraft::new(
            Rank::High,
            Rank::High,
            Cwe::CODE_INJECTION,
            "Using jinja2 templates with autoescape=False is dangerous and can lead to XSS. Use autoescape=True or use the select_autoescape function to mitigate XSS vulnerabilities.",
        )));
    }
    let is_true = matches!(&kw.value, Expr::Name(n) if n.id.as_str() == "True") || matches!(&kw.value, Expr::BooleanLiteral(b) if b.value);
    if is_true {
        return Ok(None);
    }
    let is_select_autoescape = match &kw.value {
        Expr::Call(c) => match &*c.func {
            Expr::Attribute(a) => a.attr.as_str() == "select_autoescape",
            Expr::Name(n) => n.id.as_str() == "select_autoescape",
            _ => false,
        },
        _ => false,
    };
    if is_select_autoescape {
        return Ok(None);
    }
    Ok(Some(IssueDraft::new(
        Rank::High,
        Rank::Medium,
        Cwe::CODE_INJECTION,
        "Using jinja2 templates with autoescape=False is dangerous and can lead to XSS. Ensure autoescape=True or use the select_autoescape function to mitigate XSS vulnerabilities.",
    )))
}
