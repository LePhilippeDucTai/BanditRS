//! Port of `bandit/plugins/injection_sql.py` — see docs/spec/plugins.md.
//!
//! `utils.concat_string` is reproduced here directly against `Context`'s
//! ancestor chain rather than as a standalone `_bandit_parent`-walking
//! helper: the ancestor slice already gives index-based access to every
//! level, which is simpler than reproducing Python's parent pointers.

use std::sync::LazyLock;

use regex::Regex;
use ruff_python_ast::Expr;

use crate::ast::VNode;
use crate::ast::qualname::called_name;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

static SIMPLE_SQL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?is)(select\s.*from\s|delete\s+from\s|insert\s+into\s.*values[\s(]|update\s.*set\s)",
    )
    .unwrap()
});

/// Collect the string-constant leaves of a `BinOp` chain (`_get`), skipping
/// the subtree rooted at `stop` (identity comparison).
fn collect_binop_strings(node: &Expr, stop: *const Expr, bits: &mut Vec<String>) {
    let Expr::BinOp(b) = node else { return };
    if std::ptr::eq(node as *const Expr, stop) {
        return;
    }
    push_leaf(&b.left, stop, bits);
    push_leaf(&b.right, stop, bits);
}

fn push_leaf(e: &Expr, stop: *const Expr, bits: &mut Vec<String>) {
    if matches!(e, Expr::BinOp(_)) {
        collect_binop_strings(e, stop, bits);
    } else if let Expr::StringLiteral(s) = e {
        bits.push(s.value.to_str().to_string());
    }
}

/// `_evaluate_ast(node)`: `(execute_call, statement, str_replace)`.
fn evaluate_ast(ctx: &Context<'_, '_>) -> (bool, String, bool) {
    let mut wrapper: Option<VNode<'_>> = None;
    let mut statement = String::new();
    let mut str_replace = false;

    match ctx.node {
        Some(VNode::JoinedConst(view, i)) => {
            let const_indices: Vec<u32> = view
                .values
                .iter()
                .enumerate()
                .filter(|(_, p)| p.is_constant())
                .map(|(idx, _)| idx as u32)
                .collect();
            if const_indices.first() == Some(&i) {
                statement = const_indices
                    .iter()
                    .filter_map(|&idx| view.values[idx as usize].constant_text())
                    .collect::<Vec<_>>()
                    .join("");
                wrapper = ctx.ancestor(2);
            }
        }
        _ => match ctx.parent() {
            Some(VNode::Expr(Expr::BinOp(_))) => {
                let ancestors = ctx.ancestors;
                let len = ancestors.len();
                let mut idx = len - 1;
                while idx > 0 && matches!(ancestors[idx - 1], VNode::Expr(Expr::BinOp(_))) {
                    idx -= 1;
                }
                wrapper = if idx > 0 {
                    Some(ancestors[idx - 1])
                } else {
                    None
                };
                let VNode::Expr(stop_expr) = ancestors[len - 1] else {
                    unreachable!()
                };
                let VNode::Expr(root_expr) = ancestors[idx] else {
                    unreachable!()
                };
                let mut bits: Vec<String> = Vec::new();
                if let Some(s) = ctx.string_val() {
                    bits.push(s.to_string());
                }
                collect_binop_strings(root_expr, stop_expr as *const Expr, &mut bits);
                statement = bits.join(" ");
            }
            Some(VNode::Expr(Expr::Attribute(a)))
                if a.attr.as_str() == "format" || a.attr.as_str() == "replace" =>
            {
                statement = ctx.string_val().unwrap_or("").to_string();
                wrapper = ctx.ancestor(3);
                str_replace = a.attr.as_str() == "replace";
            }
            _ => {}
        },
    }

    let execute_call = match wrapper {
        Some(VNode::Expr(Expr::Call(call))) => {
            matches!(called_name(call), "execute" | "executemany")
        }
        _ => false,
    };
    (execute_call, statement, str_replace)
}

/// `hardcoded_sql_expressions` (B608).
pub fn hardcoded_sql_expressions(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    let (execute_call, statement, str_replace) = evaluate_ast(ctx);
    if SIMPLE_SQL_RE.is_match(&statement) {
        let confidence = if execute_call && !str_replace {
            Rank::Medium
        } else {
            Rank::Low
        };
        return Ok(Some(IssueDraft::new(
            Rank::Medium,
            confidence,
            Cwe::SQL_INJECTION,
            "Possible SQL injection vector through string-based query construction.",
        )));
    }
    Ok(None)
}
