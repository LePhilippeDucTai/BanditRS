//! Port of `bandit/plugins/django_xss.py` — see docs/spec/plugins.md.
//!
//! `DeepAssignation` (upstream's search for the nearest assignment(s) of a
//! variable, walking into `FunctionDef`/`With`/`Try`/`ExceptHandler`/`If`/
//! `For`/`While` bodies and tuple-unpacking `Assign` targets) is ported in
//! full — see `is_assigned`/`is_assigned_in` below, which together are the
//! Rust equivalent of upstream's `DeepAssignation` class. `ignore_nodes` is
//! not reproduced: every call site in upstream passes `None` for it (it is
//! dead functionality — `if self.ignore_nodes:` never fires), so its
//! filtering behaviour has no observable effect to preserve.

use std::collections::VecDeque;

use ruff_python_ast::{ExceptHandler, Expr, ExprCall, Operator, Parameters, Stmt};
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
/// call is not nested in a function (`node._bandit_parent` walked up to the
/// nearest `Module`/`FunctionDef`).
fn enclosing_scope<'a>(ctx: &Context<'a, '_>) -> (&'a [Stmt], Option<&'a Parameters>) {
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

/// `name in parent.args.args` (positional-or-keyword parameters only — not
/// posonly/kwonly/vararg/kwarg, matching upstream's `parent.args.args` walk
/// bit for bit, including the resulting quirk that a keyword-only parameter
/// with the same name is *not* treated as a safe binding).
fn is_param(params: Option<&Parameters>, name: &str) -> bool {
    params.is_some_and(|p| p.args.iter().any(|a| a.name().as_str() == name))
}

/// `isinstance(e, ast.Constant) and isinstance(e.value, str)`.
fn is_str_constant(e: &Expr) -> bool {
    matches!(e, Expr::StringLiteral(_))
}

/// `isinstance(e, ast.Constant)` (any literal kind — used for `call.func.value`
/// in `evaluate_call`, which upstream does not narrow to strings).
fn is_constant_expr(e: &Expr) -> bool {
    matches!(
        e,
        Expr::StringLiteral(_)
            | Expr::BytesLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NoneLiteral(_)
            | Expr::EllipsisLiteral(_)
    )
}

/// One value `DeepAssignation.is_assigned`/`is_assigned_in` found for a
/// variable: `Expr` for the ordinary case (the right-hand side of an
/// `Assign`/`AugAssign`, possibly reached through nested control flow), or
/// `Opaque` for the one upstream quirk that returns a non-expression node
/// (`with ... as <var>:` sets `assigned = node`, the `With` statement
/// itself) — both are "found but not a Constant/Name/Call", so they only
/// ever matter for the truthiness check below, never for a recursive
/// evaluation.
#[derive(Clone, Copy)]
enum Leaf<'a> {
    Expr(&'a Expr),
    Opaque,
}

/// `DeepAssignation.is_assigned`'s return value: `False`, a single node, or
/// a list (Python is dynamically typed; this enum makes the three cases
/// explicit). An empty `Many` is falsy, matching Python's `if to:` on `[]`.
enum Assigned<'a> {
    No,
    One(Leaf<'a>),
    Many(Vec<Leaf<'a>>),
}

/// `DeepAssignation(var_name).is_assigned(node)`.
fn is_assigned<'a>(stmt: &'a Stmt, name: &str) -> Assigned<'a> {
    match stmt {
        // `ast.Expr`: upstream recurses into `node.value`, but none of the
        // `is_assigned` branches ever match a bare expression node, so the
        // recursion is always `False` — short-circuited here.
        Stmt::Expr(_) => Assigned::No,
        Stmt::FunctionDef(f) => {
            if is_param(Some(&f.parameters), name) {
                Assigned::No
            } else {
                Assigned::Many(is_assigned_in(&f.body, name))
            }
        }
        Stmt::With(w) => {
            // The loop over `items` overwrites `assigned` each iteration —
            // only the last item's outcome survives.
            let mut result = Assigned::No;
            for item in &w.items {
                let matches_var = matches!(
                    item.optional_vars.as_deref(),
                    Some(Expr::Name(n)) if n.id.as_str() == name
                );
                result = if matches_var {
                    Assigned::One(Leaf::Opaque)
                } else {
                    Assigned::Many(is_assigned_in(&w.body, name))
                };
            }
            result
        }
        Stmt::Try(t) => {
            let mut v = is_assigned_in(&t.body, name);
            for h in &t.handlers {
                let ExceptHandler::ExceptHandler(eh) = h;
                v.extend(is_assigned_in(&eh.body, name));
            }
            v.extend(is_assigned_in(&t.orelse, name));
            v.extend(is_assigned_in(&t.finalbody, name));
            Assigned::Many(v)
        }
        Stmt::If(i) => {
            let mut v = is_assigned_in(&i.body, name);
            for clause in &i.elif_else_clauses {
                v.extend(is_assigned_in(&clause.body, name));
            }
            Assigned::Many(v)
        }
        Stmt::For(f) => {
            let mut v = is_assigned_in(&f.body, name);
            v.extend(is_assigned_in(&f.orelse, name));
            Assigned::Many(v)
        }
        Stmt::While(w) => {
            let mut v = is_assigned_in(&w.body, name);
            v.extend(is_assigned_in(&w.orelse, name));
            Assigned::Many(v)
        }
        Stmt::AugAssign(a) => {
            if matches!(&*a.target, Expr::Name(n) if n.id.as_str() == name) {
                Assigned::One(Leaf::Expr(&a.value))
            } else {
                Assigned::No
            }
        }
        Stmt::Assign(a) => match a.targets.first() {
            Some(Expr::Name(n)) if n.id.as_str() == name => Assigned::One(Leaf::Expr(&a.value)),
            Some(Expr::Tuple(t)) => {
                if let Expr::Tuple(vt) = &*a.value {
                    for (pos, elt) in t.elts.iter().enumerate() {
                        // A non-`Name` unpacking target (nested tuple/list)
                        // would `AttributeError` in upstream; not exercised
                        // by any fixture, skip it defensively instead.
                        if let Expr::Name(n) = elt
                            && n.id.as_str() == name
                        {
                            return match vt.elts.get(pos) {
                                Some(v) => Assigned::One(Leaf::Expr(v)),
                                None => Assigned::No,
                            };
                        }
                    }
                    Assigned::No
                } else {
                    Assigned::No
                }
            }
            _ => Assigned::No,
        },
        _ => Assigned::No,
    }
}

/// `DeepAssignation.is_assigned_in(items)`: flatten `is_assigned` over a
/// statement list.
fn is_assigned_in<'a>(stmts: &'a [Stmt], name: &str) -> Vec<Leaf<'a>> {
    let mut out = Vec::new();
    for s in stmts {
        match is_assigned(s, name) {
            Assigned::No => {}
            Assigned::One(l) => out.push(l),
            Assigned::Many(v) => out.extend(v),
        }
    }
    out
}

/// `evaluate_var(xss_var, parent, until, ignore_nodes)`.
fn evaluate_var(
    file: &SourceFile,
    body: &[Stmt],
    params: Option<&Parameters>,
    name: &str,
    until: u32,
) -> bool {
    if is_param(params, name) {
        return false; // Params are not secure.
    }
    let mut secure = false;
    for stmt in body {
        let node_line = file.line_index(stmt.start().to_u32());
        if node_line >= until {
            break;
        }
        match is_assigned(stmt, name) {
            Assigned::No => {}
            Assigned::One(leaf) => match leaf {
                Leaf::Expr(e) if is_str_constant(e) => secure = true,
                Leaf::Expr(Expr::Name(n)) => {
                    let to_line = file.line_index(n.range().start().to_u32());
                    secure = evaluate_var(file, body, params, n.id.as_str(), to_line);
                }
                Leaf::Expr(Expr::Call(c)) => {
                    secure = evaluate_call(file, body, params, c);
                }
                _ => {
                    secure = false;
                    break;
                }
            },
            Assigned::Many(leaves) => {
                if leaves.is_empty() {
                    continue; // `if to:` — an empty list is falsy.
                }
                let mut num_secure = 0usize;
                for leaf in &leaves {
                    match leaf {
                        Leaf::Expr(e) if is_str_constant(e) => num_secure += 1,
                        Leaf::Expr(Expr::Name(n)) => {
                            if evaluate_var(file, body, params, n.id.as_str(), node_line) {
                                num_secure += 1;
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
                if num_secure == leaves.len() {
                    secure = true;
                } else {
                    secure = false;
                    break;
                }
            }
        }
    }
    secure
}

/// `evaluate_call(call, parent, ignore_nodes)`: only `<constant>.format(...)`
/// calls without keyword arguments.
fn evaluate_call(
    file: &SourceFile,
    body: &[Stmt],
    params: Option<&Parameters>,
    call: &ExprCall,
) -> bool {
    let Expr::Attribute(a) = &*call.func else {
        return false;
    };
    if a.attr.as_str() != "format" || !is_constant_expr(&a.value) {
        return false;
    }
    if !call.arguments.keywords.is_empty() {
        return false; // TODO(??) get support for this — upstream's own note.
    }
    let until = file.line_index(call.start().to_u32());
    evaluate_args(file, body, params, call.arguments.args.iter(), until)
}

/// The `args`-securing loop shared by `evaluate_call` and the `%`-formatting
/// case (`transform2call` + `evaluate_call`): a `Constant` string, a `Name`
/// resolved securely, a nested secure `Call`, or a `Starred` list/tuple whose
/// elements are all secure (mutating the pending queue exactly as upstream's
/// `args.extend(...)` mutates the list it iterates).
fn evaluate_args<'a>(
    file: &SourceFile,
    body: &[Stmt],
    params: Option<&Parameters>,
    args: impl IntoIterator<Item = &'a Expr>,
    until: u32,
) -> bool {
    let mut queue: VecDeque<&Expr> = args.into_iter().collect();
    let mut total = 0usize;
    let mut num_secure = 0usize;
    while let Some(arg) = queue.pop_front() {
        total += 1;
        match arg {
            e if is_str_constant(e) => num_secure += 1,
            Expr::Name(n) => {
                if evaluate_var(file, body, params, n.id.as_str(), until) {
                    num_secure += 1;
                } else {
                    break;
                }
            }
            Expr::Call(c) => {
                if evaluate_call(file, body, params, c) {
                    num_secure += 1;
                } else {
                    break;
                }
            }
            Expr::Starred(s) if matches!(&*s.value, Expr::List(_) | Expr::Tuple(_)) => {
                let elts: &[Expr] = match &*s.value {
                    Expr::List(l) => &l.elts,
                    Expr::Tuple(t) => &t.elts,
                    _ => unreachable!(),
                };
                queue.extend(elts.iter());
                num_secure += 1;
            }
            _ => break,
        }
    }
    num_secure == total
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
                evaluate_var(file, body, params, n.id.as_str(), call_lineno)
            }
        }
        Expr::Call(c) => evaluate_call(file, body, params, c),
        Expr::BinOp(b) if b.op == Operator::Mod && is_str_constant(&b.left) => {
            // `transform2call`: `"fmt" % x` behaves like `"fmt".format(x)` /
            // `.format(*x)` — a tuple right-hand side supplies several
            // values, anything else supplies one. `new_call.lineno` is the
            // `BinOp`'s own line, not the outer `mark_safe(...)` call's.
            let until = file.line_index(b.start().to_u32());
            let args: Vec<&Expr> = match &*b.right {
                Expr::Tuple(t) => t.elts.iter().collect(),
                other => vec![other],
            };
            evaluate_args(file, body, params, args, until)
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
