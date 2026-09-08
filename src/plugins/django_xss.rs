//! Port of `bandit/plugins/django_xss.py` — see docs/spec/plugins.md.
//!
//! `DeepAssignation` is reproduced for the common straight-line cases (an
//! `Assign` to a plain `Name` earlier in the same function/module body).
//! Python's version also walks into `try`/`with`/`for`/`while` bodies and
//! tuple-unpacking targets; those are not reproduced here (M4 follow-up) —
//! the config-gated `mark_safe_secure.py`/`mark_safe_insecure.py` fixtures
//! that exercise them are not part of the base functional suite.

use ruff_python_ast::{Expr, Operator, Stmt};
use ruff_text_size::Ranged;

use crate::ast::VNode;
use crate::ast::literal::PyErr;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;
use crate::source::SourceFile;

const MARK_SAFE_NAMES: &[&str] = &[
    "mark_safe",
    "SafeText",
    "SafeUnicode",
    "SafeString",
    "SafeBytes",
];

/// The enclosing function's parameters and body, or the module body when the
/// call is not nested in a function.
fn enclosing_scope<'a>(
    ctx: &Context<'a, '_>,
) -> (&'a [Stmt], Option<&'a ruff_python_ast::Parameters>) {
    for anc in ctx.ancestors.iter().rev() {
        if let VNode::Stmt(Stmt::FunctionDef(f)) = anc {
            return (&f.body, Some(&f.parameters));
        }
    }
    let VNode::Module(m) = ctx.ancestors[0] else {
        unreachable!("ancestors[0] is always Module")
    };
    (&m.body, None)
}

fn is_param(params: Option<&ruff_python_ast::Parameters>, name: &str) -> bool {
    let Some(p) = params else { return false };
    p.posonlyargs
        .iter()
        .chain(p.args.iter())
        .chain(p.kwonlyargs.iter())
        .any(|a| a.name().as_str() == name)
        || p.vararg.as_ref().is_some_and(|a| a.name.as_str() == name)
        || p.kwarg.as_ref().is_some_and(|a| a.name.as_str() == name)
}

/// `evaluate_var(name, body, until)`: the nearest preceding straight-line
/// assignment to `name`, evaluated recursively. `until` is a 1-based line
/// number (matching Python's `node.lineno` granularity): a statement on the
/// same line as `until` (e.g. the very assignment whose right-hand side is
/// being evaluated) is not considered "preceding".
fn evaluate_var(file: &SourceFile, body: &[Stmt], name: &str, until: u32) -> bool {
    let mut secure = false;
    let mut found = false;
    for stmt in body {
        if file.line_index(stmt.start().to_u32()) >= until {
            break;
        }
        if let Stmt::Assign(a) = stmt
            && a.targets
                .iter()
                .any(|t| matches!(t, Expr::Name(n) if n.id.as_str() == name))
        {
            secure = is_secure_value(file, &a.value, body, file.line_index(stmt.start().to_u32()));
            found = true;
        }
    }
    found && secure
}

fn is_secure_value(file: &SourceFile, expr: &Expr, body: &[Stmt], until: u32) -> bool {
    match expr {
        Expr::StringLiteral(_) => true,
        Expr::Name(n) => evaluate_var(file, body, n.id.as_str(), until),
        Expr::Call(c) => evaluate_call(file, c, body, until),
        Expr::List(l) => l.elts.iter().all(|e| is_secure_value(file, e, body, until)),
        Expr::Tuple(t) => t.elts.iter().all(|e| is_secure_value(file, e, body, until)),
        _ => false,
    }
}

/// `evaluate_call`: only `"...".format(...)` calls without keyword arguments.
fn evaluate_call(
    file: &SourceFile,
    call: &ruff_python_ast::ExprCall,
    body: &[Stmt],
    until: u32,
) -> bool {
    let Expr::Attribute(a) = &*call.func else {
        return false;
    };
    if a.attr.as_str() != "format"
        || !matches!(&*a.value, Expr::StringLiteral(_))
        || !call.arguments.keywords.is_empty()
    {
        return false;
    }
    let mut total = 0usize;
    let mut secure = 0usize;
    for arg in &call.arguments.args {
        match arg {
            Expr::Starred(s) => {
                let elts: &[Expr] = match &*s.value {
                    Expr::List(l) => &l.elts,
                    Expr::Tuple(t) => &t.elts,
                    _ => {
                        total += 1;
                        continue;
                    }
                };
                for e in elts {
                    total += 1;
                    if is_secure_value(file, e, body, until) {
                        secure += 1;
                    }
                }
            }
            other => {
                total += 1;
                if is_secure_value(file, other, body, until) {
                    secure += 1;
                }
            }
        }
    }
    secure == total
}

/// `check_risk(node)`.
fn check_risk(ctx: &Context<'_, '_>, xss: &Expr, call_lineno: u32) -> bool {
    let (body, params) = enclosing_scope(ctx);
    let file = ctx.file;
    match xss {
        Expr::Name(n) => {
            if is_param(params, n.id.as_str()) {
                false
            } else {
                evaluate_var(file, body, n.id.as_str(), call_lineno)
            }
        }
        Expr::Call(c) => evaluate_call(file, c, body, call_lineno),
        Expr::BinOp(b) if b.op == Operator::Mod && matches!(&*b.left, Expr::StringLiteral(_)) => {
            // `"fmt" % x` behaves like `"fmt".format(x)` / `.format(*x)` for
            // our purposes: a tuple right-hand side supplies several values,
            // anything else supplies one.
            match &*b.right {
                Expr::Tuple(t) => t
                    .elts
                    .iter()
                    .all(|e| is_secure_value(file, e, body, call_lineno)),
                other => is_secure_value(file, other, body, call_lineno),
            }
        }
        _ => false,
    }
}

/// `django_mark_safe` (B703).
pub fn django_mark_safe(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !ctx.is_module_imported_like("django.utils.safestring") {
        return Ok(None);
    }
    if !ctx
        .call_function_name()
        .is_some_and(|n| MARK_SAFE_NAMES.contains(&n))
    {
        return Ok(None);
    }
    let call = ctx.call.expect("Call context always carries a call");
    let Some(xss) = call.arguments.args.first() else {
        return Err(PyErr::index_error("list index out of range"));
    };
    if matches!(xss, Expr::StringLiteral(_)) {
        return Ok(None);
    }
    let call_lineno = ctx.file.line_index(call.start().to_u32());
    let secure = check_risk(ctx, xss, call_lineno);
    if !secure {
        return Ok(Some(IssueDraft::new(
            Rank::Medium,
            Rank::High,
            Cwe::BASIC_XSS,
            "Potential XSS on mark_safe function.",
        )));
    }
    Ok(None)
}
