//! Port of `bandit/plugins/injection_shell.py` — see docs/spec/plugins.md.

use ruff_python_ast::{Expr, ExprCall, Number};

use crate::ast::literal::PyErr;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::{ListOpt, PluginConfigs};
use crate::plugins::PluginResult;

fn qual_in(list: &ListOpt, key: &str, qual: &str) -> Result<bool, PyErr> {
    Ok(list.items(key)?.iter().any(|s| s == qual))
}

/// `has_shell(context)`.
fn has_shell(call: &ExprCall) -> bool {
    let Some(kw) = call
        .arguments
        .keywords
        .iter()
        .find(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == "shell"))
    else {
        return false;
    };
    match &kw.value {
        Expr::BooleanLiteral(b) => b.value,
        Expr::NumberLiteral(n) => match &n.value {
            Number::Int(i) => i.as_i64().map(|v| v != 0).unwrap_or(true),
            Number::Float(f) => *f != 0.0,
            Number::Complex { real, imag } => *real != 0.0 || *imag != 0.0,
        },
        Expr::List(l) => !l.elts.is_empty(),
        Expr::Dict(d) => !d.items.is_empty(),
        Expr::Name(n) if n.id.as_str() == "False" || n.id.as_str() == "None" => false,
        Expr::StringLiteral(s) => !s.value.to_str().is_empty(),
        Expr::BytesLiteral(b) => b.value.bytes().next().is_some(),
        Expr::NoneLiteral(_) => false,
        Expr::EllipsisLiteral(_) => true,
        _ => true,
    }
}

/// `_evaluate_shell_call(context)`.
fn evaluate_shell_call(call: &ExprCall) -> Rank {
    match call.arguments.args.first() {
        Some(Expr::StringLiteral(_)) => Rank::Low,
        _ => Rank::High,
    }
}

/// `full_path_match`: starts with a Windows drive letter (`X:`) or one of `\/.`.
fn full_path_match(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => chars.next() == Some(':'),
        Some(c) => c == '\\' || c == '/' || c == '.',
        None => false,
    }
}

/// `subprocess_popen_with_shell_equals_true` (B602).
pub fn subprocess_popen_with_shell_equals_true(
    ctx: &Context<'_, '_>,
    cfg: &PluginConfigs,
) -> PluginResult {
    let Some(call) = ctx.call else {
        return Ok(None);
    };
    let qual = ctx.call_function_name_qual().unwrap_or("");
    if !qual_in(&cfg.shell_injection.subprocess, "subprocess", qual)?
        || !has_shell(call)
        || call.arguments.args.is_empty()
    {
        return Ok(None);
    }
    let lineno = ctx.get_lineno_for_call_arg("shell");
    if evaluate_shell_call(call) == Rank::Low {
        return Ok(Some(
            IssueDraft::new(
                Rank::Low,
                Rank::High,
                Cwe::OS_COMMAND_INJECTION,
                "subprocess call with shell=True seems safe, but may be changed in the future, consider rewriting without shell",
            )
            .with_lineno(lineno),
        ));
    }
    Ok(Some(
        IssueDraft::new(
            Rank::High,
            Rank::High,
            Cwe::OS_COMMAND_INJECTION,
            "subprocess call with shell=True identified, security issue.",
        )
        .with_lineno(lineno),
    ))
}

/// `subprocess_without_shell_equals_true` (B603).
pub fn subprocess_without_shell_equals_true(
    ctx: &Context<'_, '_>,
    cfg: &PluginConfigs,
) -> PluginResult {
    let Some(call) = ctx.call else {
        return Ok(None);
    };
    let qual = ctx.call_function_name_qual().unwrap_or("");
    if qual_in(&cfg.shell_injection.subprocess, "subprocess", qual)? && !has_shell(call) {
        return Ok(Some(
            IssueDraft::new(
                Rank::Low,
                Rank::High,
                Cwe::OS_COMMAND_INJECTION,
                "subprocess call - check for execution of untrusted input.",
            )
            .with_lineno(ctx.get_lineno_for_call_arg("shell")),
        ));
    }
    Ok(None)
}

/// `any_other_function_with_shell_equals_true` (B604).
pub fn any_other_function_with_shell_equals_true(
    ctx: &Context<'_, '_>,
    cfg: &PluginConfigs,
) -> PluginResult {
    let Some(call) = ctx.call else {
        return Ok(None);
    };
    let qual = ctx.call_function_name_qual().unwrap_or("");
    if !qual_in(&cfg.shell_injection.subprocess, "subprocess", qual)? && has_shell(call) {
        return Ok(Some(
            IssueDraft::new(
                Rank::Medium,
                Rank::Low,
                Cwe::OS_COMMAND_INJECTION,
                "Function call with shell=True parameter identified, possible security issue.",
            )
            .with_lineno(ctx.get_lineno_for_call_arg("shell")),
        ));
    }
    Ok(None)
}

/// `start_process_with_a_shell` (B605).
pub fn start_process_with_a_shell(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let Some(call) = ctx.call else {
        return Ok(None);
    };
    let qual = ctx.call_function_name_qual().unwrap_or("");
    if qual_in(&cfg.shell_injection.shell, "shell", qual)? && !call.arguments.args.is_empty() {
        if evaluate_shell_call(call) == Rank::Low {
            return Ok(Some(IssueDraft::new(
                Rank::Low,
                Rank::High,
                Cwe::OS_COMMAND_INJECTION,
                "Starting a process with a shell: Seems safe, but may be changed in the future, consider rewriting without shell",
            )));
        }
        return Ok(Some(IssueDraft::new(
            Rank::High,
            Rank::High,
            Cwe::OS_COMMAND_INJECTION,
            "Starting a process with a shell, possible injection detected, security issue.",
        )));
    }
    Ok(None)
}

/// `start_process_with_no_shell` (B606).
pub fn start_process_with_no_shell(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let qual = ctx.call_function_name_qual().unwrap_or("");
    if qual_in(&cfg.shell_injection.no_shell, "no_shell", qual)? {
        return Ok(Some(IssueDraft::new(
            Rank::Low,
            Rank::Medium,
            Cwe::OS_COMMAND_INJECTION,
            "Starting a process without a shell.",
        )));
    }
    Ok(None)
}

/// `start_process_with_partial_path` (B607).
pub fn start_process_with_partial_path(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let Some(call) = ctx.call else {
        return Ok(None);
    };
    if call.arguments.args.is_empty() {
        return Ok(None);
    }
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let in_any = qual_in(&cfg.shell_injection.subprocess, "subprocess", qual)?
        || qual_in(&cfg.shell_injection.shell, "shell", qual)?
        || qual_in(&cfg.shell_injection.no_shell, "no_shell", qual)?;
    if !in_any {
        return Ok(None);
    }
    let mut node = &call.arguments.args[0];
    if let Expr::List(l) = node
        && let Some(first) = l.elts.first()
    {
        node = first;
    }
    if let Expr::StringLiteral(s) = node
        && !full_path_match(s.value.to_str())
    {
        return Ok(Some(IssueDraft::new(
            Rank::Low,
            Rank::High,
            Cwe::OS_COMMAND_INJECTION,
            "Starting a process with a partial executable path",
        )));
    }
    Ok(None)
}
