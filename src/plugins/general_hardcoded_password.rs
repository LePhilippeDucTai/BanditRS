//! Port of `bandit/plugins/general_hardcoded_password.py` — see docs/spec/plugins.md.

use std::sync::LazyLock;

use regex::Regex;
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;

use crate::ast::VNode;
use crate::ast::literal::get_literal_value;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

static RE_CANDIDATES: LazyLock<Regex> = LazyLock::new(|| {
    const WORDS: &str = r"(pas+wo?r?d|pass(phrase)?|pwd|token|secrete?)";
    Regex::new(&format!(r"(?i)(^{WORDS}$|_{WORDS}_|^{WORDS}_|_{WORDS}$)")).unwrap()
});

fn report(value: &str, lineno: Option<u32>) -> IssueDraft {
    IssueDraft::new(
        Rank::Low,
        Rank::Medium,
        Cwe::HARD_CODED_PASSWORD,
        format!("Possible hardcoded password: '{value}'"),
    )
    .with_lineno(lineno)
}

/// `hardcoded_password_string` (B105).
pub fn hardcoded_password_string(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    let Some(node) = ctx.node else {
        return Ok(None);
    };
    let Some(parent) = ctx.parent() else {
        return Ok(None);
    };
    match parent {
        VNode::Stmt(Stmt::Assign(a)) => {
            for targ in &a.targets {
                let matched = match targ {
                    Expr::Name(n) => RE_CANDIDATES.is_match(n.id.as_str()),
                    Expr::Attribute(at) => RE_CANDIDATES.is_match(at.attr.as_str()),
                    _ => false,
                };
                if matched && let Some(s) = ctx.string_val() {
                    return Ok(Some(report(s, None)));
                }
            }
        }
        VNode::Expr(Expr::Dict(d)) => {
            if let Some(node_expr) = node.as_expr()
                && let Some(pos) = d
                    .items
                    .iter()
                    .position(|it| it.key.as_ref().is_some_and(|k| std::ptr::eq(k, node_expr)))
                && ctx.string_val().is_some_and(|s| RE_CANDIDATES.is_match(s))
                && let Expr::StringLiteral(_)
                | Expr::NumberLiteral(_)
                | Expr::BooleanLiteral(_)
                | Expr::NoneLiteral(_)
                | Expr::BytesLiteral(_) = &d.items[pos].value
            {
                let value = get_literal_value(&d.items[pos].value)?;
                return Ok(Some(report(&value.py_str(), None)));
            }
        }
        VNode::Expr(Expr::Subscript(_)) => {
            if ctx.string_val().is_some_and(|s| RE_CANDIDATES.is_match(s))
                && let Some(VNode::Stmt(Stmt::Assign(a))) = ctx.ancestor(2)
                && let Expr::StringLiteral(v) = &*a.value
            {
                return Ok(Some(report(v.value.to_str(), None)));
            }
        }
        VNode::Expr(Expr::Compare(c)) => {
            let candidate = match &*c.left {
                Expr::Name(n) => Some(n.id.as_str()),
                Expr::Attribute(at) => Some(at.attr.as_str()),
                _ => None,
            };
            if candidate.is_some_and(|n| RE_CANDIDATES.is_match(n))
                && let Some(Expr::StringLiteral(v)) = c.comparators.first()
            {
                return Ok(Some(report(v.value.to_str(), None)));
            }
        }
        _ => {}
    }
    Ok(None)
}

/// `hardcoded_password_funcarg` (B106).
pub fn hardcoded_password_funcarg(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    let Some(call) = ctx.call else {
        return Ok(None);
    };
    for kw in &call.arguments.keywords {
        let Some(arg) = &kw.arg else { continue };
        if let Expr::StringLiteral(s) = &kw.value
            && RE_CANDIDATES.is_match(arg.as_str())
        {
            let lineno = ctx.file.line_index(kw.value.start().to_u32());
            return Ok(Some(report(s.value.to_str(), Some(lineno))));
        }
    }
    Ok(None)
}

/// `hardcoded_password_default` (B107).
pub fn hardcoded_password_default(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    let Some(f) = ctx.function else {
        return Ok(None);
    };
    for p in f.parameters.args.iter() {
        let Some(default) = p.default() else { continue };
        if matches!(default, Expr::NoneLiteral(_)) {
            continue;
        }
        if let Expr::StringLiteral(s) = default
            && RE_CANDIDATES.is_match(p.name().as_str())
        {
            return Ok(Some(report(s.value.to_str(), None)));
        }
    }
    Ok(None)
}
