//! Virtual AST nodes: a `Copy` reference to any node bandit may visit,
//! including the synthetic nodes CPython has but ruff does not (`elif`
//! clauses as nested `If`, merged f-string constants, format-spec
//! `JoinedStr`s).

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use super::joined_str::JoinedStrView;

/// The node kinds bandit dispatches tests on (CPython class names).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NodeKind {
    Call = 0,
    Str = 1,
    Bytes = 2,
    FunctionDef = 3,
    AsyncFunctionDef = 4,
    ClassDef = 5,
    Assert = 6,
    ExceptHandler = 7,
    Import = 8,
    ImportFrom = 9,
    File = 10,
    Other = 11,
}

impl NodeKind {
    pub const COUNT: usize = 12;

    /// Kinds a plugin can register for (`@test.checks(...)`).
    pub fn parse(name: &str) -> Option<NodeKind> {
        Some(match name {
            "Call" => NodeKind::Call,
            "Str" => NodeKind::Str,
            "Bytes" => NodeKind::Bytes,
            "FunctionDef" => NodeKind::FunctionDef,
            "AsyncFunctionDef" => NodeKind::AsyncFunctionDef,
            "ClassDef" => NodeKind::ClassDef,
            "Assert" => NodeKind::Assert,
            "ExceptHandler" => NodeKind::ExceptHandler,
            "Import" => NodeKind::Import,
            "ImportFrom" => NodeKind::ImportFrom,
            "File" => NodeKind::File,
            _ => return None,
        })
    }

    pub const fn name(self) -> &'static str {
        match self {
            NodeKind::Call => "Call",
            NodeKind::Str => "Str",
            NodeKind::Bytes => "Bytes",
            NodeKind::FunctionDef => "FunctionDef",
            NodeKind::AsyncFunctionDef => "AsyncFunctionDef",
            NodeKind::ClassDef => "ClassDef",
            NodeKind::Assert => "Assert",
            NodeKind::ExceptHandler => "ExceptHandler",
            NodeKind::Import => "Import",
            NodeKind::ImportFrom => "ImportFrom",
            NodeKind::File => "File",
            NodeKind::Other => "Other",
        }
    }
}

/// A reference to a node of the CPython-shaped tree.
#[derive(Clone, Copy)]
pub enum VNode<'a> {
    Module(&'a ast::ModModule),
    Stmt(&'a Stmt),
    Expr(&'a Expr),
    /// CPython represents `elif` as a nested `If` inside `orelse`; the second
    /// field is the enclosing `if` statement (its end is the clause's end).
    ElifClause(&'a ast::ElifElseClause, &'a ast::StmtIf),
    ExceptHandler(&'a ast::ExceptHandlerExceptHandler),
    /// CPython `arguments` (no position).
    Arguments(&'a ast::Parameters),
    /// CPython `arguments` of a `lambda` without parameters (ruff has no
    /// node for it; it has no position and no children).
    EmptyArguments(&'a ast::ExprLambda),
    /// CPython `arg`.
    Arg(&'a ast::Parameter),
    Keyword(&'a ast::Keyword),
    Alias(&'a ast::Alias),
    /// CPython `withitem` (no position).
    WithItem(&'a ast::WithItem),
    /// CPython `comprehension` (no position).
    Comprehension(&'a ast::Comprehension),
    /// CPython `match_case` (no position).
    MatchCase(&'a ast::MatchCase),
    Pattern(&'a ast::Pattern),
    TypeParam(&'a ast::TypeParam),
    /// A `Constant(str)` value of a `JoinedStr`/`TemplateStr` (merged literal
    /// segments); the index points into the view's `values`.
    JoinedConst(&'a JoinedStrView<'a>, u32),
    /// A `FormattedValue` / `Interpolation` of a joined string.
    FormattedValue(&'a JoinedStrView<'a>, u32),
    /// The `JoinedStr` synthesised for a format spec.
    FormatSpec(&'a JoinedStrView<'a>),
}

impl<'a> VNode<'a> {
    /// Kind used for test dispatch.
    pub fn kind(self) -> NodeKind {
        match self {
            VNode::Stmt(s) => match s {
                Stmt::FunctionDef(f) if f.is_async => NodeKind::AsyncFunctionDef,
                Stmt::FunctionDef(_) => NodeKind::FunctionDef,
                Stmt::ClassDef(_) => NodeKind::ClassDef,
                Stmt::Assert(_) => NodeKind::Assert,
                Stmt::Import(_) => NodeKind::Import,
                Stmt::ImportFrom(_) => NodeKind::ImportFrom,
                _ => NodeKind::Other,
            },
            VNode::Expr(e) => match e {
                Expr::Call(_) => NodeKind::Call,
                Expr::StringLiteral(_) => NodeKind::Str,
                Expr::BytesLiteral(_) => NodeKind::Bytes,
                _ => NodeKind::Other,
            },
            VNode::ExceptHandler(_) => NodeKind::ExceptHandler,
            VNode::JoinedConst(..) => NodeKind::Str,
            _ => NodeKind::Other,
        }
    }

    /// CPython class name of the node (`node.__class__.__name__`).
    pub fn class_name(self) -> &'static str {
        match self {
            VNode::Module(_) => "Module",
            VNode::Stmt(s) => match s {
                Stmt::FunctionDef(f) => {
                    if f.is_async {
                        "AsyncFunctionDef"
                    } else {
                        "FunctionDef"
                    }
                }
                Stmt::ClassDef(_) => "ClassDef",
                Stmt::Return(_) => "Return",
                Stmt::Delete(_) => "Delete",
                Stmt::TypeAlias(_) => "TypeAlias",
                Stmt::Assign(_) => "Assign",
                Stmt::AugAssign(_) => "AugAssign",
                Stmt::AnnAssign(_) => "AnnAssign",
                Stmt::For(f) => {
                    if f.is_async {
                        "AsyncFor"
                    } else {
                        "For"
                    }
                }
                Stmt::While(_) => "While",
                Stmt::If(_) => "If",
                Stmt::With(w) => {
                    if w.is_async {
                        "AsyncWith"
                    } else {
                        "With"
                    }
                }
                Stmt::Match(_) => "Match",
                Stmt::Raise(_) => "Raise",
                Stmt::Try(t) => {
                    if t.is_star {
                        "TryStar"
                    } else {
                        "Try"
                    }
                }
                Stmt::Assert(_) => "Assert",
                Stmt::Import(_) => "Import",
                Stmt::ImportFrom(_) => "ImportFrom",
                Stmt::Global(_) => "Global",
                Stmt::Nonlocal(_) => "Nonlocal",
                Stmt::Expr(_) => "Expr",
                Stmt::Pass(_) => "Pass",
                Stmt::Break(_) => "Break",
                Stmt::Continue(_) => "Continue",
                Stmt::IpyEscapeCommand(_) => "IpyEscapeCommand",
            },
            VNode::Expr(e) => match e {
                Expr::BoolOp(_) => "BoolOp",
                Expr::Named(_) => "NamedExpr",
                Expr::BinOp(_) => "BinOp",
                Expr::UnaryOp(_) => "UnaryOp",
                Expr::Lambda(_) => "Lambda",
                Expr::If(_) => "IfExp",
                Expr::Dict(_) => "Dict",
                Expr::Set(_) => "Set",
                Expr::ListComp(_) => "ListComp",
                Expr::SetComp(_) => "SetComp",
                Expr::DictComp(_) => "DictComp",
                Expr::Generator(_) => "GeneratorExp",
                Expr::Await(_) => "Await",
                Expr::Yield(_) => "Yield",
                Expr::YieldFrom(_) => "YieldFrom",
                Expr::Compare(_) => "Compare",
                Expr::Call(_) => "Call",
                Expr::FString(_) => "JoinedStr",
                Expr::TString(_) => "TemplateStr",
                Expr::StringLiteral(_)
                | Expr::BytesLiteral(_)
                | Expr::NumberLiteral(_)
                | Expr::BooleanLiteral(_)
                | Expr::NoneLiteral(_)
                | Expr::EllipsisLiteral(_) => "Constant",
                Expr::Attribute(_) => "Attribute",
                Expr::Subscript(_) => "Subscript",
                Expr::Starred(_) => "Starred",
                Expr::Name(_) => "Name",
                Expr::List(_) => "List",
                Expr::Tuple(_) => "Tuple",
                Expr::Slice(_) => "Slice",
                Expr::IpyEscapeCommand(_) => "IpyEscapeCommand",
            },
            VNode::ElifClause(..) => "If",
            VNode::ExceptHandler(_) => "ExceptHandler",
            VNode::Arguments(_) | VNode::EmptyArguments(_) => "arguments",
            VNode::Arg(_) => "arg",
            VNode::Keyword(_) => "keyword",
            VNode::Alias(_) => "alias",
            VNode::WithItem(_) => "withitem",
            VNode::Comprehension(_) => "comprehension",
            VNode::MatchCase(_) => "match_case",
            VNode::Pattern(p) => match p {
                ast::Pattern::MatchValue(_) => "MatchValue",
                ast::Pattern::MatchSingleton(_) => "MatchSingleton",
                ast::Pattern::MatchSequence(_) => "MatchSequence",
                ast::Pattern::MatchMapping(_) => "MatchMapping",
                ast::Pattern::MatchClass(_) => "MatchClass",
                ast::Pattern::MatchStar(_) => "MatchStar",
                ast::Pattern::MatchAs(_) => "MatchAs",
                ast::Pattern::MatchOr(_) => "MatchOr",
            },
            VNode::TypeParam(t) => match t {
                ast::TypeParam::TypeVar(_) => "TypeVar",
                ast::TypeParam::ParamSpec(_) => "ParamSpec",
                ast::TypeParam::TypeVarTuple(_) => "TypeVarTuple",
            },
            VNode::JoinedConst(..) => "Constant",
            VNode::FormattedValue(view, _) => {
                if view.is_template() {
                    "Interpolation"
                } else {
                    "FormattedValue"
                }
            }
            VNode::FormatSpec(_) => "JoinedStr",
        }
    }

    /// Whether the CPython node carries `lineno`/`col_offset` attributes.
    pub fn has_position(self) -> bool {
        !matches!(
            self,
            VNode::Module(_)
                | VNode::Arguments(_)
                | VNode::EmptyArguments(_)
                | VNode::WithItem(_)
                | VNode::Comprehension(_)
                | VNode::MatchCase(_)
        )
    }

    /// Raw ruff range of the node (see [`super::positions`] for the CPython
    /// adjusted range).
    pub fn raw_range(self) -> TextRange {
        match self {
            VNode::Module(m) => m.range(),
            VNode::Stmt(s) => s.range(),
            VNode::Expr(e) => e.range(),
            VNode::ElifClause(c, i) => TextRange::new(c.start(), i.end()),
            VNode::ExceptHandler(h) => h.range(),
            VNode::Arguments(p) => p.range(),
            VNode::EmptyArguments(l) => TextRange::empty(l.start()),
            VNode::Arg(a) => a.range(),
            VNode::Keyword(k) => k.range(),
            VNode::Alias(a) => a.range(),
            VNode::WithItem(w) => w.range(),
            VNode::Comprehension(c) => c.range(),
            VNode::MatchCase(m) => m.range(),
            VNode::Pattern(p) => p.range(),
            VNode::TypeParam(t) => t.range(),
            VNode::JoinedConst(view, i) | VNode::FormattedValue(view, i) => {
                view.values[i as usize].range()
            }
            VNode::FormatSpec(view) => view.range,
        }
    }

    /// Identity of the underlying node (`id(node)` in Python).
    pub fn ptr_id(self) -> usize {
        match self {
            VNode::Module(m) => m as *const _ as usize,
            VNode::Stmt(s) => s as *const _ as usize,
            VNode::Expr(e) => e as *const _ as usize,
            VNode::ElifClause(c, _) => c as *const _ as usize,
            VNode::ExceptHandler(h) => h as *const _ as usize,
            VNode::Arguments(p) => p as *const _ as usize,
            VNode::EmptyArguments(l) => (l as *const _ as usize) ^ 4,
            VNode::Arg(a) => a as *const _ as usize,
            VNode::Keyword(k) => k as *const _ as usize,
            VNode::Alias(a) => a as *const _ as usize,
            VNode::WithItem(w) => w as *const _ as usize,
            VNode::Comprehension(c) => c as *const _ as usize,
            VNode::MatchCase(m) => m as *const _ as usize,
            VNode::Pattern(p) => p as *const _ as usize,
            VNode::TypeParam(t) => t as *const _ as usize,
            VNode::JoinedConst(view, i) => (view as *const _ as usize) ^ ((i as usize + 1) << 3),
            VNode::FormattedValue(view, i) => {
                (view as *const _ as usize) ^ ((i as usize + 1) << 3) ^ 1
            }
            VNode::FormatSpec(view) => (view as *const _ as usize) ^ 2,
        }
    }

    /// Identity comparison (`node is other`).
    pub fn same(self, other: VNode<'a>) -> bool {
        self.ptr_id() == other.ptr_id()
    }

    pub fn as_expr(self) -> Option<&'a Expr> {
        match self {
            VNode::Expr(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_stmt(self) -> Option<&'a Stmt> {
        match self {
            VNode::Stmt(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_call(self) -> Option<&'a ast::ExprCall> {
        match self {
            VNode::Expr(Expr::Call(c)) => Some(c),
            _ => None,
        }
    }

    pub fn as_function_def(self) -> Option<&'a ast::StmtFunctionDef> {
        match self {
            VNode::Stmt(Stmt::FunctionDef(f)) => Some(f),
            _ => None,
        }
    }

    /// Whether this is a CPython `Expr` statement (used for the docstring check).
    pub fn is_expr_stmt(self) -> bool {
        matches!(self, VNode::Stmt(Stmt::Expr(_)))
    }

    /// String value when the node is a `Constant(str)`.
    pub fn str_value(self) -> Option<&'a str> {
        match self {
            VNode::Expr(Expr::StringLiteral(s)) => Some(s.value.to_str()),
            VNode::JoinedConst(view, i) => view.values[i as usize].constant_text(),
            _ => None,
        }
    }

    /// Whether the node is a `Constant` whose value is a `str`.
    pub fn is_str_constant(self) -> bool {
        matches!(
            self,
            VNode::Expr(Expr::StringLiteral(_)) | VNode::JoinedConst(..)
        )
    }
}

impl std::fmt::Debug for VNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{:?}", self.class_name(), self.raw_range())
    }
}
