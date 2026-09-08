//! Enumerate the children of a node in the order CPython's `ast.iter_fields`
//! yields them, together with the "next sibling in the same list" that
//! bandit records as `_bandit_sibling`.

use ruff_python_ast::{self as ast, Expr, Stmt};

use super::PyCompat;
use super::joined_str::{JoinedPart, JoinedStrView, ViewArena};
use super::vnode::VNode;

/// A child node and its next sibling within the same list field (if any).
#[derive(Clone, Copy)]
pub struct Child<'a> {
    pub node: VNode<'a>,
    pub sibling: Option<VNode<'a>>,
}

/// Shared state needed to enumerate children (f-string views are built on
/// demand and stored in the arena).
pub struct WalkCtx<'a, 'b> {
    pub arena: &'a ViewArena<'a>,
    pub source: &'b str,
    pub compat: PyCompat,
}

impl<'a, 'b> WalkCtx<'a, 'b> {
    pub fn new(arena: &'a ViewArena<'a>, source: &'b str, compat: PyCompat) -> Self {
        WalkCtx {
            arena,
            source,
            compat,
        }
    }
}

/// Push a list field: each element's sibling is the next slot (which may be
/// `None`, e.g. a `**` entry in a dict or a keyword-only parameter without a
/// default).
fn push_slots<'a>(out: &mut Vec<Child<'a>>, slots: &[Option<VNode<'a>>]) {
    for (i, slot) in slots.iter().enumerate() {
        if let Some(node) = slot {
            out.push(Child {
                node: *node,
                sibling: slots.get(i + 1).copied().flatten(),
            });
        }
    }
}

fn push_list<'a>(out: &mut Vec<Child<'a>>, nodes: impl Iterator<Item = VNode<'a>>) {
    let start = out.len();
    for node in nodes {
        out.push(Child {
            node,
            sibling: None,
        });
    }
    let end = out.len();
    for i in start..end {
        if i + 1 < end {
            out[i].sibling = Some(out[i + 1].node);
        }
    }
}

fn push_single<'a>(out: &mut Vec<Child<'a>>, node: VNode<'a>) {
    out.push(Child {
        node,
        sibling: None,
    });
}

fn push_opt<'a>(out: &mut Vec<Child<'a>>, node: Option<VNode<'a>>) {
    if let Some(n) = node {
        push_single(out, n);
    }
}

fn exprs<'a>(out: &mut Vec<Child<'a>>, it: impl Iterator<Item = &'a Expr>) {
    push_list(out, it.map(VNode::Expr));
}

fn stmts<'a>(out: &mut Vec<Child<'a>>, it: impl Iterator<Item = &'a Stmt>) {
    push_list(out, it.map(VNode::Stmt));
}

fn expr<'a>(out: &mut Vec<Child<'a>>, e: &'a Expr) {
    push_single(out, VNode::Expr(e));
}

fn opt_expr<'a>(out: &mut Vec<Child<'a>>, e: Option<&'a Expr>) {
    push_opt(out, e.map(VNode::Expr));
}

fn type_params<'a>(out: &mut Vec<Child<'a>>, tp: Option<&'a ast::TypeParams>) {
    if let Some(tp) = tp {
        push_list(out, tp.type_params.iter().map(VNode::TypeParam));
    }
}

fn joined_values<'a>(out: &mut Vec<Child<'a>>, view: &'a JoinedStrView<'a>) {
    push_list(
        out,
        view.values.iter().enumerate().map(|(i, v)| match v {
            JoinedPart::Constant { .. } => VNode::JoinedConst(view, i as u32),
            JoinedPart::Formatted { .. } => VNode::FormattedValue(view, i as u32),
        }),
    );
}

/// `orelse` of an `if` statement / `elif` clause: the next clause is either
/// a nested `If` (elif) or the statements of the final `else`.
fn if_orelse<'a>(out: &mut Vec<Child<'a>>, stmt: &'a ast::StmtIf, next: usize) {
    match stmt.elif_else_clauses.get(next) {
        None => {}
        Some(clause) if clause.test.is_some() => push_single(out, VNode::ElifClause(clause, stmt)),
        Some(clause) => stmts(out, clause.body.iter()),
    }
}

/// Append the children of `node` to `out` in CPython field order.
pub fn push_children<'a>(node: VNode<'a>, ctx: &WalkCtx<'a, '_>, out: &mut Vec<Child<'a>>) {
    match node {
        VNode::Module(m) => stmts(out, m.body.iter()),
        VNode::Stmt(s) => match s {
            Stmt::FunctionDef(f) => {
                push_single(out, VNode::Arguments(&f.parameters));
                stmts(out, f.body.iter());
                exprs(out, f.decorator_list.iter().map(|d| &d.expression));
                opt_expr(out, f.returns.as_deref());
                type_params(out, f.type_params.as_deref());
            }
            Stmt::ClassDef(c) => {
                if let Some(args) = &c.arguments {
                    exprs(out, args.args.iter());
                    push_list(out, args.keywords.iter().map(VNode::Keyword));
                }
                stmts(out, c.body.iter());
                exprs(out, c.decorator_list.iter().map(|d| &d.expression));
                type_params(out, c.type_params.as_deref());
            }
            Stmt::Return(r) => opt_expr(out, r.value.as_deref()),
            Stmt::Delete(d) => exprs(out, d.targets.iter()),
            Stmt::TypeAlias(t) => {
                expr(out, &t.name);
                type_params(out, t.type_params.as_deref());
                expr(out, &t.value);
            }
            Stmt::Assign(a) => {
                exprs(out, a.targets.iter());
                expr(out, &a.value);
            }
            Stmt::AugAssign(a) => {
                expr(out, &a.target);
                expr(out, &a.value);
            }
            Stmt::AnnAssign(a) => {
                expr(out, &a.target);
                expr(out, &a.annotation);
                opt_expr(out, a.value.as_deref());
            }
            Stmt::For(f) => {
                expr(out, &f.target);
                expr(out, &f.iter);
                stmts(out, f.body.iter());
                stmts(out, f.orelse.iter());
            }
            Stmt::While(w) => {
                expr(out, &w.test);
                stmts(out, w.body.iter());
                stmts(out, w.orelse.iter());
            }
            Stmt::If(i) => {
                expr(out, &i.test);
                stmts(out, i.body.iter());
                if_orelse(out, i, 0);
            }
            Stmt::With(w) => {
                push_list(out, w.items.iter().map(VNode::WithItem));
                stmts(out, w.body.iter());
            }
            Stmt::Match(m) => {
                expr(out, &m.subject);
                push_list(out, m.cases.iter().map(VNode::MatchCase));
            }
            Stmt::Raise(r) => {
                opt_expr(out, r.exc.as_deref());
                opt_expr(out, r.cause.as_deref());
            }
            Stmt::Try(t) => {
                stmts(out, t.body.iter());
                push_list(
                    out,
                    t.handlers.iter().map(|h| match h {
                        ast::ExceptHandler::ExceptHandler(h) => VNode::ExceptHandler(h),
                    }),
                );
                stmts(out, t.orelse.iter());
                stmts(out, t.finalbody.iter());
            }
            Stmt::Assert(a) => {
                expr(out, &a.test);
                opt_expr(out, a.msg.as_deref());
            }
            Stmt::Import(i) => push_list(out, i.names.iter().map(VNode::Alias)),
            Stmt::ImportFrom(i) => push_list(out, i.names.iter().map(VNode::Alias)),
            Stmt::Expr(e) => expr(out, &e.value),
            Stmt::Global(_)
            | Stmt::Nonlocal(_)
            | Stmt::Pass(_)
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::IpyEscapeCommand(_) => {}
        },
        VNode::ElifClause(clause, stmt) => {
            opt_expr(out, clause.test.as_ref());
            stmts(out, clause.body.iter());
            let idx = stmt
                .elif_else_clauses
                .iter()
                .position(|c| std::ptr::eq(c, clause))
                .unwrap_or(stmt.elif_else_clauses.len());
            if_orelse(out, stmt, idx + 1);
        }
        VNode::Expr(e) => match e {
            Expr::BoolOp(b) => exprs(out, b.values.iter()),
            Expr::Named(n) => {
                expr(out, &n.target);
                expr(out, &n.value);
            }
            Expr::BinOp(b) => {
                expr(out, &b.left);
                expr(out, &b.right);
            }
            Expr::UnaryOp(u) => expr(out, &u.operand),
            Expr::Lambda(l) => {
                match &l.parameters {
                    Some(p) => push_single(out, VNode::Arguments(p)),
                    None => push_single(out, VNode::EmptyArguments(l)),
                }
                expr(out, &l.body);
            }
            Expr::If(i) => {
                expr(out, &i.test);
                expr(out, &i.body);
                expr(out, &i.orelse);
            }
            Expr::Dict(d) => {
                let keys: Vec<Option<VNode<'a>>> = d
                    .items
                    .iter()
                    .map(|it| it.key.as_ref().map(VNode::Expr))
                    .collect();
                push_slots(out, &keys);
                exprs(out, d.items.iter().map(|it| &it.value));
            }
            Expr::Set(s) => exprs(out, s.elts.iter()),
            Expr::ListComp(c) => {
                expr(out, &c.elt);
                push_list(out, c.generators.iter().map(VNode::Comprehension));
            }
            Expr::SetComp(c) => {
                expr(out, &c.elt);
                push_list(out, c.generators.iter().map(VNode::Comprehension));
            }
            Expr::DictComp(c) => {
                opt_expr(out, c.key.as_deref());
                expr(out, &c.value);
                push_list(out, c.generators.iter().map(VNode::Comprehension));
            }
            Expr::Generator(g) => {
                expr(out, &g.elt);
                push_list(out, g.generators.iter().map(VNode::Comprehension));
            }
            Expr::Await(a) => expr(out, &a.value),
            Expr::Yield(y) => opt_expr(out, y.value.as_deref()),
            Expr::YieldFrom(y) => expr(out, &y.value),
            Expr::Compare(c) => {
                expr(out, &c.left);
                exprs(out, c.comparators.iter());
            }
            Expr::Call(c) => {
                expr(out, &c.func);
                exprs(out, c.arguments.args.iter());
                push_list(out, c.arguments.keywords.iter().map(VNode::Keyword));
            }
            Expr::FString(_) | Expr::TString(_) => {
                if let Some(view) = JoinedStrView::build(e, ctx.arena, ctx.source, ctx.compat) {
                    joined_values(out, view);
                }
            }
            Expr::Attribute(a) => expr(out, &a.value),
            Expr::Subscript(s) => {
                expr(out, &s.value);
                expr(out, &s.slice);
            }
            Expr::Starred(s) => expr(out, &s.value),
            Expr::List(l) => exprs(out, l.elts.iter()),
            Expr::Tuple(t) => exprs(out, t.elts.iter()),
            Expr::Slice(s) => {
                opt_expr(out, s.lower.as_deref());
                opt_expr(out, s.upper.as_deref());
                opt_expr(out, s.step.as_deref());
            }
            Expr::StringLiteral(_)
            | Expr::BytesLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NoneLiteral(_)
            | Expr::EllipsisLiteral(_)
            | Expr::Name(_)
            | Expr::IpyEscapeCommand(_) => {}
        },
        VNode::ExceptHandler(h) => {
            opt_expr(out, h.type_.as_deref());
            stmts(out, h.body.iter());
        }
        VNode::Arguments(p) => {
            push_list(out, p.posonlyargs.iter().map(|a| VNode::Arg(&a.parameter)));
            push_list(out, p.args.iter().map(|a| VNode::Arg(&a.parameter)));
            push_opt(out, p.vararg.as_deref().map(VNode::Arg));
            push_list(out, p.kwonlyargs.iter().map(|a| VNode::Arg(&a.parameter)));
            let kw_defaults: Vec<Option<VNode<'a>>> = p
                .kwonlyargs
                .iter()
                .map(|a| a.default.as_deref().map(VNode::Expr))
                .collect();
            push_slots(out, &kw_defaults);
            push_opt(out, p.kwarg.as_deref().map(VNode::Arg));
            exprs(
                out,
                p.posonlyargs
                    .iter()
                    .chain(p.args.iter())
                    .filter_map(|a| a.default.as_deref()),
            );
        }
        VNode::EmptyArguments(_) => {}
        VNode::Arg(a) => opt_expr(out, a.annotation.as_deref()),
        VNode::Keyword(k) => expr(out, &k.value),
        VNode::Alias(_) => {}
        VNode::WithItem(w) => {
            expr(out, &w.context_expr);
            opt_expr(out, w.optional_vars.as_deref());
        }
        VNode::Comprehension(c) => {
            expr(out, &c.target);
            expr(out, &c.iter);
            exprs(out, c.ifs.iter());
        }
        VNode::MatchCase(m) => {
            push_single(out, VNode::Pattern(&m.pattern));
            opt_expr(out, m.guard.as_deref());
            stmts(out, m.body.iter());
        }
        VNode::Pattern(p) => match p {
            ast::Pattern::MatchValue(v) => expr(out, &v.value),
            ast::Pattern::MatchSingleton(_) | ast::Pattern::MatchStar(_) => {}
            ast::Pattern::MatchSequence(s) => push_list(out, s.patterns.iter().map(VNode::Pattern)),
            ast::Pattern::MatchMapping(m) => {
                exprs(out, m.keys.iter());
                push_list(out, m.patterns.iter().map(VNode::Pattern));
            }
            ast::Pattern::MatchClass(c) => {
                expr(out, &c.cls);
                push_list(out, c.arguments.patterns.iter().map(VNode::Pattern));
                push_list(
                    out,
                    c.arguments
                        .keywords
                        .iter()
                        .map(|k| VNode::Pattern(&k.pattern)),
                );
            }
            ast::Pattern::MatchAs(a) => push_opt(out, a.pattern.as_deref().map(VNode::Pattern)),
            ast::Pattern::MatchOr(o) => push_list(out, o.patterns.iter().map(VNode::Pattern)),
        },
        VNode::TypeParam(t) => match t {
            ast::TypeParam::TypeVar(v) => {
                opt_expr(out, v.bound.as_deref());
                opt_expr(out, v.default.as_deref());
            }
            ast::TypeParam::ParamSpec(v) => opt_expr(out, v.default.as_deref()),
            ast::TypeParam::TypeVarTuple(v) => opt_expr(out, v.default.as_deref()),
        },
        VNode::JoinedConst(..) => {}
        VNode::FormattedValue(view, i) => {
            if let JoinedPart::Formatted { element, spec, .. } = &view.values[i as usize] {
                expr(out, &element.expression);
                if let Some(spec) = spec {
                    push_single(out, VNode::FormatSpec(spec));
                }
            }
        }
        VNode::FormatSpec(view) => joined_values(out, view),
    }
}

/// Convenience: collect the children of a node.
pub fn children<'a>(node: VNode<'a>, ctx: &WalkCtx<'a, '_>) -> Vec<Child<'a>> {
    let mut out = Vec::new();
    push_children(node, ctx, &mut out);
    out
}
