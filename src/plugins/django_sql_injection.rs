//! Port of `bandit/plugins/django_sql_injection.py` — see docs/spec/plugins.md.

use ruff_python_ast::Expr;

use crate::ast::literal::PyErr;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `django_extra_used` (B610).
pub fn django_extra_used(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if ctx.call_function_name() != Some("extra") {
        return Ok(None);
    }
    let Some(call) = ctx.call else { return Ok(None) };

    let mut select: Option<&Expr> = None;
    let mut where_: Option<&Expr> = None;
    let mut tables: Option<&Expr> = None;
    for kw in &call.arguments.keywords {
        match kw.arg.as_ref().map(|a| a.as_str()) {
            Some("select") => select = Some(&kw.value),
            Some("where") => where_ = Some(&kw.value),
            Some("tables") => tables = Some(&kw.value),
            _ => {}
        }
    }
    let args = &call.arguments.args;
    if !args.is_empty() {
        select = args.first();
        where_ = args.get(1).or(where_);
        tables = args.get(3).or(tables);
    }

    let mut insecure = false;
    'outer: for val in [where_, tables] {
        if let Some(v) = val {
            match v {
                Expr::List(l) => {
                    for elt in &l.elts {
                        if !matches!(elt, Expr::StringLiteral(_)) {
                            insecure = true;
                            break 'outer;
                        }
                    }
                }
                _ => {
                    insecure = true;
                    break 'outer;
                }
            }
        }
    }
    if !insecure {
        if let Some(v) = select {
            match v {
                Expr::Dict(d) => {
                    let all_str_keys = d.items.iter().all(|it| it.key.as_ref().is_some_and(|k| matches!(k, Expr::StringLiteral(_))));
                    let all_str_values = all_str_keys && d.items.iter().all(|it| matches!(&it.value, Expr::StringLiteral(_)));
                    insecure = !all_str_values;
                }
                _ => insecure = true,
            }
        }
    }

    if insecure {
        return Ok(Some(IssueDraft::new(Rank::Medium, Rank::Medium, Cwe::SQL_INJECTION, "Use of extra potential SQL attack vector.")));
    }
    Ok(None)
}

/// `django_rawsql_used` (B611).
pub fn django_rawsql_used(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !ctx.is_module_imported_like("django.db.models") || ctx.call_function_name() != Some("RawSQL") {
        return Ok(None);
    }
    let Some(call) = ctx.call else { return Ok(None) };
    let sql: &Expr = if let Some(first) = call.arguments.args.first() {
        first
    } else {
        match call.arguments.keywords.iter().find(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == "sql")) {
            Some(k) => &k.value,
            None => return Err(PyErr::key_error("sql")),
        }
    };
    if !matches!(sql, Expr::StringLiteral(_)) {
        return Ok(Some(IssueDraft::new(Rank::Medium, Rank::Medium, Cwe::SQL_INJECTION, "Use of RawSQL potential SQL attack vector.")));
    }
    Ok(None)
}
