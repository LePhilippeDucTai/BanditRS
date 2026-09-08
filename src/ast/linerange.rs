//! `utils.linerange` / `utils.calc_linerange`.

use ruff_python_ast::{Expr, Stmt};

use super::children::{WalkCtx, push_children};
use super::positions;
use super::vnode::VNode;
use crate::core::issue::LineRange;
use crate::source::SourceFile;

/// Fields stripped when computing the range of a position-less node.
fn stripped_children<'a>(node: VNode<'a>, ctx: &WalkCtx<'a, '_>) -> Vec<VNode<'a>> {
    let mut out = Vec::new();
    match node {
        // `Module.body` is stripped: nothing left.
        VNode::Module(_) => {}
        // `match_case.body` is stripped: pattern and guard only.
        VNode::MatchCase(m) => {
            out.push(VNode::Pattern(&m.pattern));
            if let Some(g) = &m.guard {
                out.push(VNode::Expr(g));
            }
        }
        _ => {
            let mut children = Vec::new();
            push_children(node, ctx, &mut children);
            out.extend(children.into_iter().map(|c| c.node));
        }
    }
    out
}

/// `calc_linerange(node)`: min/max of the start lines of the node and all
/// its descendants (`(9999999999, -1)` sentinels when none has a position).
pub fn calc_linerange<'a>(
    node: VNode<'a>,
    parent: Option<VNode<'a>>,
    file: &SourceFile,
    ctx: &WalkCtx<'a, '_>,
) -> (i64, i64) {
    let mut lines_min: i64 = 9_999_999_999;
    let mut lines_max: i64 = -1;
    let mut stack: Vec<(VNode<'a>, Option<VNode<'a>>)> = vec![(node, parent)];
    let mut buf = Vec::new();
    while let Some((n, p)) = stack.pop() {
        if let Some(l) = positions::lineno(n, p, file) {
            let l = l as i64;
            lines_min = lines_min.min(l);
            lines_max = lines_max.max(l);
        }
        buf.clear();
        push_children(n, ctx, &mut buf);
        for c in buf.iter().rev() {
            stack.push((c.node, Some(n)));
        }
    }
    (lines_min, lines_max)
}

/// `linerange(node)`.
pub fn linerange<'a>(
    node: VNode<'a>,
    sibling: Option<VNode<'a>>,
    parent: Option<VNode<'a>>,
    file: &SourceFile,
    ctx: &WalkCtx<'a, '_>,
) -> LineRange {
    if let Some(pos) = positions::position(node, parent, file) {
        return LineRange::new(pos.lineno, pos.end_lineno);
    }
    let mut lines_min: i64 = 9_999_999_999;
    let mut lines_max: i64 = -1;
    for child in stripped_children(node, ctx) {
        let (lo, hi) = calc_linerange(child, Some(node), file, ctx);
        lines_min = lines_min.min(lo);
        lines_max = lines_max.max(hi);
    }
    if lines_max == -1 {
        lines_min = 0;
        lines_max = 1;
    }
    // Work around CPython issue #16806 (multi-line strings): clamp to the
    // line before the next sibling.
    if let Some(sib) = sibling
        && let Some(sib_line) = positions::lineno(sib, parent, file)
    {
        let start = lines_min;
        let delta = sib_line as i64 - start;
        if delta > 1 {
            return LineRange::new(start as u32, sib_line - 1);
        }
    }
    LineRange::new(lines_min as u32, lines_max as u32)
}

#[allow(dead_code)]
fn _types(_: &Stmt, _: &Expr) {}
