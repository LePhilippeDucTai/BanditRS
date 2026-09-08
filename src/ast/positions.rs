//! CPython-compatible positions for the nodes of the virtual tree.

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::vnode::VNode;
use crate::source::SourceFile;

/// `lineno`, `col_offset`, `end_lineno`, `end_col_offset` (lines 1-based,
/// columns in UTF-8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub lineno: u32,
    pub col_offset: u32,
    pub end_lineno: u32,
    pub end_col_offset: u32,
}

/// Offset of the first significant character at or after `from`, skipping
/// whitespace, comments, line continuations and newlines.
fn keyword_start_after(text: &str, from: usize) -> usize {
    let b = text.as_bytes();
    let mut i = from;
    loop {
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r' | b'\x0c') {
            i += 1;
        }
        if i < b.len() && b[i] == b'#' {
            while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                i += 1;
            }
            continue;
        }
        if i < b.len() && b[i] == b'\\' {
            i += 1;
            continue;
        }
        return i;
    }
}

/// The CPython range of `node` (`None` for position-less nodes). `parent` is
/// needed for the generator-argument special case.
pub fn cpython_range(
    node: VNode<'_>,
    parent: Option<VNode<'_>>,
    source: &str,
) -> Option<TextRange> {
    if !node.has_position() {
        return None;
    }
    let raw = node.raw_range();
    match node {
        VNode::Stmt(Stmt::FunctionDef(f)) if !f.decorator_list.is_empty() => {
            let last = f.decorator_list.last().expect("non-empty");
            let start = keyword_start_after(source, last.end().to_usize());
            Some(TextRange::new(TextSize::from(start as u32), raw.end()))
        }
        VNode::Stmt(Stmt::ClassDef(c)) if !c.decorator_list.is_empty() => {
            let last = c.decorator_list.last().expect("non-empty");
            let start = keyword_start_after(source, last.end().to_usize());
            Some(TextRange::new(TextSize::from(start as u32), raw.end()))
        }
        VNode::Arg(_) => {
            // CPython's `arg` excludes the `*` / `**` of varargs.
            let b = source.as_bytes();
            let mut s = raw.start().to_usize();
            if b.get(s) == Some(&b'*') {
                while b.get(s) == Some(&b'*') {
                    s += 1;
                }
                while matches!(
                    b.get(s),
                    Some(b' ' | b'\t' | b'\x0c' | b'\\' | b'\n' | b'\r')
                ) {
                    s += 1;
                }
            }
            Some(TextRange::new(TextSize::from(s as u32), raw.end()))
        }
        VNode::Expr(Expr::Generator(g)) if !g.parenthesized => {
            // `f(x for x in y)`: CPython includes the call parentheses.
            if let Some(VNode::Expr(Expr::Call(call))) = parent
                && call.arguments.keywords.is_empty()
                && call.arguments.args.len() == 1
                && let Some(Expr::Generator(first)) = call.arguments.args.first()
                && std::ptr::eq(first, g)
            {
                return Some(call.arguments.range());
            }
            Some(raw)
        }
        _ => Some(raw),
    }
}

/// CPython position of `node`.
pub fn position(node: VNode<'_>, parent: Option<VNode<'_>>, file: &SourceFile) -> Option<Pos> {
    let range = cpython_range(node, parent, &file.text)?;
    let (lineno, col_offset) = file.line_col(range.start().to_u32());
    let (end_lineno, end_col_offset) = file.line_col(range.end().to_u32());
    Some(Pos {
        lineno,
        col_offset,
        end_lineno,
        end_col_offset,
    })
}

/// Start line of `node` (CPython `lineno`).
pub fn lineno(node: VNode<'_>, parent: Option<VNode<'_>>, file: &SourceFile) -> Option<u32> {
    cpython_range(node, parent, &file.text).map(|r| file.line_index(r.start().to_u32()))
}

/// Helper for tests: whether a statement is decorated.
pub fn is_decorated(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::FunctionDef(f) => !f.decorator_list.is_empty(),
        Stmt::ClassDef(c) => !c.decorator_list.is_empty(),
        _ => false,
    }
}

#[allow(dead_code)]
fn _assert_types(_: &ast::StmtFunctionDef) {}
