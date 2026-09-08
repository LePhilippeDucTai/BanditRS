//! The node visitor (port of `bandit/core/node_visitor.py`).
//!
//! The traversal is iterative (explicit frame stack) and mirrors bandit's
//! hand-rolled `generic_visit`: pre-order, CPython field order, with the
//! parent chain and next-sibling information available to tests.

use std::borrow::Cow;

use ruff_python_ast::{Expr, ModModule, Stmt};
use rustc_hash::FxHashSet;

use super::children::{Child, WalkCtx, push_children};
use super::joined_str::ViewArena;
use super::linerange::linerange;
use super::positions::position;
use super::qualname::{Aliases, call_name_into};
use super::vnode::{NodeKind, VNode};
use super::{Pos, PyCompat};
use crate::core::context::Context;
use crate::core::issue::LineRange;
use crate::core::metrics::Scores;
use crate::core::utils::get_module_qualname_from_path;
use crate::source::SourceFile;

/// Receiver of the contexts produced by the walk (the tester).
pub trait TestRunner<'a> {
    /// Whether any test is registered for `kind` (contexts are only built
    /// when needed).
    fn wants(&self, kind: NodeKind) -> bool;

    /// Run the tests registered for `kind`.
    fn run_tests(&mut self, ctx: &Context<'a, '_>, kind: NodeKind) -> Scores;

    /// Whether `pre_visit` should be called for every node (tracing).
    fn trace_all(&self) -> bool {
        false
    }

    /// Called for every node when tracing is enabled.
    fn pre_visit(&mut self, _ctx: &Context<'a, '_>) {}
}

struct Frame<'a> {
    children_start: usize,
    children_count: usize,
    cursor: usize,
    /// Namespace length to restore when leaving the node.
    ns_len: usize,
    pops_namespace: bool,
    /// Next sibling of the node (needed by `visit_Str` for the parent).
    sibling: Option<VNode<'a>>,
}

/// Mutable traversal state, kept apart from the runner so that contexts
/// (which borrow the state) can be handed to the runner.
struct State<'a, 'w> {
    file: &'w SourceFile,
    ctx: WalkCtx<'a, 'w>,
    namespace: String,
    imports: FxHashSet<String>,
    import_aliases: Aliases,
    frames: Vec<Frame<'a>>,
    ancestors: Vec<VNode<'a>>,
    children: Vec<Child<'a>>,
    scores: Scores,
}

impl<'a, 'w> State<'a, 'w> {
    /// Build the base context of `node` (`pre_visit`).
    fn base_context<'c>(&'c self, node: VNode<'a>, sibling: Option<VNode<'a>>) -> Context<'a, 'c> {
        let parent = self.ancestors.last().copied();
        Context {
            file: self.file,
            node: Some(node),
            ancestors: &self.ancestors,
            sibling,
            pos: position(node, parent, self.file),
            linerange: linerange(node, sibling, parent, self.file, &self.ctx),
            call: None,
            function: None,
            qualname: None,
            name: None,
            module: None,
            str_val: None,
            bytes_val: None,
            imports: &self.imports,
            import_aliases: &self.import_aliases,
        }
    }

    /// `n`-th ancestor of the node being visited (1 = parent).
    fn ancestor(&self, n: usize) -> Option<VNode<'a>> {
        let len = self.ancestors.len();
        if n == 0 || n > len {
            None
        } else {
            Some(self.ancestors[len - n])
        }
    }

    /// `visit_Import` bookkeeping; returns the last module name.
    fn record_import(&mut self, names: &'a [ruff_python_ast::Alias]) -> Option<&'a str> {
        let mut module = None;
        for alias in names {
            let name = alias.name.as_str();
            if let Some(asname) = &alias.asname {
                self.import_aliases
                    .insert(asname.as_str().to_string(), name.to_string());
            }
            self.imports.insert(name.to_string());
            module = Some(name);
        }
        module
    }
}

/// Walks one module and feeds contexts to a [`TestRunner`].
pub struct Walker<'a, 'w, R> {
    state: State<'a, 'w>,
    runner: &'w mut R,
    qual_buf: String,
}

impl<'a, 'w, R: TestRunner<'a>> Walker<'a, 'w, R> {
    pub fn new(
        file: &'w SourceFile,
        arena: &'a ViewArena<'a>,
        compat: PyCompat,
        runner: &'w mut R,
    ) -> Self
    where
        'w: 'a,
    {
        let namespace = match get_module_qualname_from_path(&file.name) {
            Ok(ns) => ns,
            Err(_) => {
                crate::log_warning!(
                    "node_visitor",
                    "Unable to find qualified name for module: {}",
                    file.name
                );
                String::new()
            }
        };
        crate::log_debug!("node_visitor", "Module qualified name: {}", namespace);
        Walker {
            state: State {
                file,
                ctx: WalkCtx::new(arena, &file.text, compat),
                namespace,
                imports: FxHashSet::default(),
                import_aliases: Aliases::default(),
                frames: Vec::with_capacity(64),
                ancestors: Vec::with_capacity(64),
                children: Vec::with_capacity(256),
                scores: Scores::default(),
            },
            runner,
            qual_buf: String::with_capacity(64),
        }
    }

    /// Walk the module, then run the file-level tests.
    pub fn process(&mut self, module: &'a ModModule) -> Scores {
        let st = &mut self.state;
        let root = VNode::Module(module);
        st.ancestors.push(root);
        let start = st.children.len();
        push_children(root, &st.ctx, &mut st.children);
        let count = st.children.len() - start;
        let ns_len = st.namespace.len();
        st.frames.push(Frame {
            children_start: start,
            children_count: count,
            cursor: 0,
            ns_len,
            pops_namespace: false,
            sibling: None,
        });
        while let Some(frame) = self.state.frames.last_mut() {
            if frame.cursor < frame.children_count {
                let child = self.state.children[frame.children_start + frame.cursor];
                frame.cursor += 1;
                let (ns_len, pops_namespace) = self.visit(child.node, child.sibling);
                let st = &mut self.state;
                st.ancestors.push(child.node);
                let start = st.children.len();
                push_children(child.node, &st.ctx, &mut st.children);
                let count = st.children.len() - start;
                st.frames.push(Frame {
                    children_start: start,
                    children_count: count,
                    cursor: 0,
                    ns_len,
                    pops_namespace,
                    sibling: child.sibling,
                });
            } else {
                let st = &mut self.state;
                let frame = st.frames.pop().expect("frame");
                st.children.truncate(frame.children_start);
                st.ancestors.pop();
                if frame.pops_namespace {
                    st.namespace.truncate(frame.ns_len);
                }
            }
        }
        // File-level tests: a synthetic context without a node.
        if self.runner.wants(NodeKind::File) {
            let st = &self.state;
            let ctx = Context {
                file: st.file,
                node: None,
                ancestors: &[],
                sibling: None,
                pos: Some(Pos {
                    lineno: 0,
                    col_offset: 0,
                    end_lineno: 0,
                    end_col_offset: 0,
                }),
                linerange: LineRange::new(0, 1),
                call: None,
                function: None,
                qualname: None,
                name: None,
                module: None,
                str_val: None,
                bytes_val: None,
                imports: &st.imports,
                import_aliases: &st.import_aliases,
            };
            let s = self.runner.run_tests(&ctx, NodeKind::File);
            self.state.scores.add(&s);
        }
        self.state.scores
    }

    /// `visit(node)`: returns the namespace length before the visit and
    /// whether `post_visit` must restore it.
    fn visit(&mut self, node: VNode<'a>, sibling: Option<VNode<'a>>) -> (usize, bool) {
        let ns_len = self.state.namespace.len();
        let kind = node.kind();
        if self.runner.trace_all() {
            let ctx = self.state.base_context(node, sibling);
            self.runner.pre_visit(&ctx);
        }
        match kind {
            NodeKind::ClassDef => {
                if let VNode::Stmt(Stmt::ClassDef(c)) = node {
                    self.state.namespace.push('.');
                    self.state.namespace.push_str(c.name.as_str());
                }
                (ns_len, true)
            }
            NodeKind::FunctionDef => {
                let VNode::Stmt(Stmt::FunctionDef(f)) = node else {
                    unreachable!()
                };
                let qualname = format!("{}.{}", self.state.namespace, f.name.as_str());
                let name = qualname.rsplit('.').next().unwrap_or("").to_string();
                self.state.namespace.push('.');
                self.state.namespace.push_str(&name);
                if self.runner.wants(kind) {
                    let mut ctx = self.state.base_context(node, sibling);
                    ctx.function = Some(f);
                    ctx.qualname = Some(&qualname);
                    ctx.name = Some(&name);
                    let s = self.runner.run_tests(&ctx, kind);
                    drop(ctx);
                    self.state.scores.add(&s);
                }
                (ns_len, true)
            }
            NodeKind::AsyncFunctionDef => {
                if self.runner.wants(kind) {
                    let VNode::Stmt(Stmt::FunctionDef(f)) = node else {
                        unreachable!()
                    };
                    let mut ctx = self.state.base_context(node, sibling);
                    ctx.function = Some(f);
                    let s = self.runner.run_tests(&ctx, kind);
                    drop(ctx);
                    self.state.scores.add(&s);
                }
                (ns_len, false)
            }
            NodeKind::Call => {
                if self.runner.wants(kind) {
                    let VNode::Expr(Expr::Call(call)) = node else {
                        unreachable!()
                    };
                    let mut buf = std::mem::take(&mut self.qual_buf);
                    call_name_into(call, &self.state.import_aliases, &mut buf);
                    let s = {
                        let name = buf.rsplit('.').next().unwrap_or("");
                        let mut ctx = self.state.base_context(node, sibling);
                        ctx.call = Some(call);
                        ctx.qualname = Some(&buf);
                        ctx.name = Some(name);
                        self.runner.run_tests(&ctx, kind)
                    };
                    self.state.scores.add(&s);
                    self.qual_buf = buf;
                }
                (ns_len, false)
            }
            NodeKind::Import => {
                let VNode::Stmt(Stmt::Import(imp)) = node else {
                    unreachable!()
                };
                let module = self.state.record_import(&imp.names);
                self.run_import(node, sibling, module);
                (ns_len, false)
            }
            NodeKind::ImportFrom => {
                let VNode::Stmt(Stmt::ImportFrom(imp)) = node else {
                    unreachable!()
                };
                match &imp.module {
                    None => {
                        // `from . import x` is handled like a plain import.
                        let module = self.state.record_import(&imp.names);
                        self.run_import(node, sibling, module);
                    }
                    Some(module) => {
                        let module = module.as_str();
                        let mut last_name: Option<&'a str> = None;
                        for alias in &imp.names {
                            let qual = format!("{module}.{}", alias.name.as_str());
                            let key = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                            self.state
                                .import_aliases
                                .insert(key.to_string(), qual.clone());
                            self.state.imports.insert(qual);
                            last_name = Some(alias.name.as_str());
                        }
                        if self.runner.wants(NodeKind::ImportFrom) {
                            let mut ctx = self.state.base_context(node, sibling);
                            ctx.module = Some(module);
                            ctx.name = last_name;
                            let s = self.runner.run_tests(&ctx, NodeKind::ImportFrom);
                            drop(ctx);
                            self.state.scores.add(&s);
                        }
                    }
                }
                (ns_len, false)
            }
            NodeKind::Str | NodeKind::Bytes => {
                let parent = self.state.ancestors.last().copied();
                let is_docstring = parent.is_some_and(|p| p.is_expr_stmt());
                if !is_docstring && self.runner.wants(kind) {
                    let st = &self.state;
                    let mut ctx = st.base_context(node, sibling);
                    if kind == NodeKind::Str {
                        ctx.str_val = node.str_value();
                    } else if let VNode::Expr(Expr::BytesLiteral(b)) = node {
                        ctx.bytes_val = Some(if b.value.is_implicit_concatenated() {
                            Cow::Owned(b.value.bytes().collect())
                        } else {
                            Cow::Borrowed(
                                b.value.iter().next().map(|p| p.as_slice()).unwrap_or(&[]),
                            )
                        });
                    }
                    if let Some(p) = parent {
                        let grandparent = st.ancestor(2);
                        let parent_sibling = st.frames.last().and_then(|f| f.sibling);
                        ctx.linerange = linerange(p, parent_sibling, grandparent, st.file, &st.ctx);
                    }
                    let s = self.runner.run_tests(&ctx, kind);
                    drop(ctx);
                    self.state.scores.add(&s);
                }
                (ns_len, false)
            }
            NodeKind::Assert | NodeKind::ExceptHandler | NodeKind::Other | NodeKind::File => {
                if kind != NodeKind::Other && self.runner.wants(kind) {
                    let ctx = self.state.base_context(node, sibling);
                    let s = self.runner.run_tests(&ctx, kind);
                    drop(ctx);
                    self.state.scores.add(&s);
                }
                (ns_len, false)
            }
        }
    }

    fn run_import(&mut self, node: VNode<'a>, sibling: Option<VNode<'a>>, module: Option<&'a str>) {
        if self.runner.wants(NodeKind::Import) {
            let mut ctx = self.state.base_context(node, sibling);
            ctx.module = module;
            let s = self.runner.run_tests(&ctx, NodeKind::Import);
            drop(ctx);
            self.state.scores.add(&s);
        }
    }
}

/// Test support (`docs/plan/wp/WP-09-unit-core-context.md`): parse `source`
/// (as file `filename`) and walk it exactly like `BanditNodeVisitor` would,
/// calling `f` with the [`Context`] of every node of kind `kind`, in visit
/// order. Not used by production code — tests build a real `Context` this
/// way instead of `bandit.core.context.Context(context_object=Mock)`, which
/// has no typed equivalent.
pub fn with_contexts(
    filename: &str,
    source: &str,
    kind: NodeKind,
    f: impl FnMut(&Context<'_, '_>),
) {
    struct Collector<F> {
        kind: NodeKind,
        f: F,
    }

    impl<'a, F> TestRunner<'a> for Collector<F>
    where
        F: for<'c> FnMut(&Context<'a, 'c>),
    {
        fn wants(&self, kind: NodeKind) -> bool {
            kind == self.kind
        }

        fn run_tests(&mut self, ctx: &Context<'a, '_>, _kind: NodeKind) -> Scores {
            (self.f)(ctx);
            Scores::default()
        }
    }

    let file = SourceFile::new(filename, source);
    let compat = PyCompat::Py312;
    let parsed = crate::source::parse::parse_module(&file.text, compat)
        .unwrap_or_else(|e| panic!("with_contexts: {filename}: {e}"));
    let arena = ViewArena::new();
    let mut runner = Collector { kind, f };
    let mut walker = Walker::new(&file, &arena, compat, &mut runner);
    walker.process(parsed.syntax());
}
