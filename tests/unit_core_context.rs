//! Port of `tests/unit/core/test_context.py` (`Context`, obtenu en parcourant de vrais extraits).
//!
//! Work package: `docs/plan/wp/WP-09-unit-core-context.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/unit/core/test_context.py`.
//!
//! The Python tests build `Context(context_object=dict|Mock)`: the Rust
//! `Context` is typed and only exists while the walker visits a node, so
//! every test below instead parses a small snippet and walks it with
//! `banditrs::ast::walker::with_contexts` (added by this work package),
//! picking the `Context` of the node kind the Python mock stood in for. A
//! few snippets differ cosmetically from the Python source (e.g. a bare
//! string literal statement is a *docstring* to the walker and never gets a
//! `Str` context — `x = 'spam'` is used instead) while producing the exact
//! same `Context` values the Python test asserted on.

use banditrs::ast::VNode;
use banditrs::ast::literal::{PyValue, get_literal_value};
use banditrs::ast::vnode::NodeKind;
use banditrs::ast::walker::with_contexts;
use banditrs::core::context::Context;
use ruff_python_parser::parse_expression;

/// Walk `src` (as file `t.py`) and apply `f` to the first `Context` of kind
/// `kind` visited, returning its (necessarily owned — the `Context` and
/// everything it borrows from is gone once the walk returns) result. Panics
/// if no node of that kind is visited.
fn first_context<T>(src: &str, kind: NodeKind, f: impl FnOnce(&Context<'_, '_>) -> T) -> T {
    let mut f = Some(f);
    let mut out = None;
    // "./t.py" (rather than "t.py") has a directory component, so
    // `get_module_qualname_from_path` resolves it without a warning.
    with_contexts("./t.py", src, kind, |ctx| {
        if let Some(f) = f.take() {
            out = Some(f(ctx));
        }
    });
    out.unwrap_or_else(|| panic!("first_context: no {kind:?} node visited in {src:?}"))
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test__get_literal_value`
/// (renamed: Python's double-underscore method name is not a valid Rust
/// identifier — `non_snake_case`). Exercises `ast::literal::get_literal_value`
/// directly on parsed literal expressions (no walker needed). The Python
/// `_get_literal_value(None)` case has no typed Rust equivalent — the
/// function takes `&Expr`, never an absent node — and is not ported.
#[test]
fn test_get_literal_value() {
    fn expr(src: &str) -> ruff_python_ast::Expr {
        parse_expression(src).unwrap().into_expr()
    }

    let e = expr("42");
    let v = get_literal_value(&e).unwrap();
    assert_eq!(v.as_int(), Some(42));

    let e = expr("'spam'");
    let v = get_literal_value(&e).unwrap();
    assert_eq!(v.as_str(), Some("spam"));

    let e = expr("['spam', 42]");
    let v = get_literal_value(&e).unwrap();
    let list = v.as_list().unwrap();
    assert_eq!(list[0].as_str(), Some("spam"));
    assert_eq!(list[1].as_int(), Some(42));

    let e = expr("('spam', 42)");
    let v = get_literal_value(&e).unwrap();
    match v {
        PyValue::Tuple(items) => {
            assert_eq!(items[0].as_str(), Some("spam"));
            assert_eq!(items[1].as_int(), Some(42));
        }
        other => panic!("expected a tuple, got {other:?}"),
    }

    let e = expr("{'spam', 42}");
    let v = get_literal_value(&e).unwrap();
    match v {
        PyValue::Set(items) => {
            assert!(items.iter().any(|x| x.as_str() == Some("spam")));
            assert!(items.iter().any(|x| x.as_int() == Some(42)));
        }
        other => panic!("expected a set, got {other:?}"),
    }

    // `_get_literal_value(ast.Dict)` does not evaluate its keys/values — it
    // just zips the raw nodes (`dict(zip(literal.keys, literal.values))`);
    // `PyValue::Dict` mirrors that by holding the raw `ExprDict`, so the
    // check below walks its items instead of comparing `PyValue`s (`PyValue`
    // has no `PartialEq` — see `check_call_arg_value`'s `py_eq`).
    let e = expr("{'spam': 42, 'eggs': 'foo'}");
    let v = get_literal_value(&e).unwrap();
    match v {
        PyValue::Dict(d) => {
            assert_eq!(d.items.len(), 2);
            let key0 = get_literal_value(d.items[0].key.as_ref().unwrap()).unwrap();
            let val0 = get_literal_value(&d.items[0].value).unwrap();
            let key1 = get_literal_value(d.items[1].key.as_ref().unwrap()).unwrap();
            let val1 = get_literal_value(&d.items[1].value).unwrap();
            assert_eq!(key0.as_str(), Some("spam"));
            assert_eq!(val0.as_int(), Some(42));
            assert_eq!(key1.as_str(), Some("eggs"));
            assert_eq!(val1.as_str(), Some("foo"));
        }
        other => panic!("expected a dict, got {other:?}"),
    }

    let e = expr("spam");
    let v = get_literal_value(&e).unwrap();
    assert_eq!(v.as_str(), Some("spam"));

    let e = expr("b'spam'");
    let v = get_literal_value(&e).unwrap();
    match v {
        PyValue::Bytes(b) => assert_eq!(&*b, b"spam"),
        other => panic!("expected bytes, got {other:?}"),
    }
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_call_args` (adapted, see WP).
#[test]
fn test_call_args() {
    let args: Vec<String> = first_context("f(x.spam, 'eggs')", NodeKind::Call, |ctx| {
        ctx.call_args()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    });
    assert_eq!(args, vec!["spam".to_string(), "eggs".to_string()]);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_call_args_count` (adapted, see WP).
#[test]
fn test_call_args_count() {
    let n = first_context("f('spam', 'eggs')", NodeKind::Call, |ctx| {
        ctx.call_args_count()
    });
    assert_eq!(n, Some(2));

    // `call_args_count` is `None` off a `Call` context; `x = 'x'` gives a
    // real `Str` context (a bare `'x'` statement would be a docstring — see
    // module doc comment).
    let n = first_context("x = 'x'", NodeKind::Str, |ctx| ctx.call_args_count());
    assert_eq!(n, None);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_call_function_name` (adapted, see WP).
#[test]
fn test_call_function_name() {
    let name = first_context("spam()", NodeKind::Call, |ctx| {
        ctx.call_function_name().map(str::to_string)
    });
    assert_eq!(name.as_deref(), Some("spam"));

    let name = first_context("x = 'x'", NodeKind::Str, |ctx| {
        ctx.call_function_name().map(str::to_string)
    });
    assert_eq!(name, None);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_call_function_name_qual` (adapted, see WP).
#[test]
fn test_call_function_name_qual() {
    let name = first_context("spam()", NodeKind::Call, |ctx| {
        ctx.call_function_name_qual().map(str::to_string)
    });
    assert_eq!(name.as_deref(), Some("spam"));

    let name = first_context("x = 'x'", NodeKind::Str, |ctx| {
        ctx.call_function_name_qual().map(str::to_string)
    });
    assert_eq!(name, None);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_call_keywords` (adapted, see WP).
#[test]
fn test_call_keywords() {
    let (arg1, arg2) = first_context("f(arg1=x.spam, arg2='eggs')", NodeKind::Call, |ctx| {
        let kw = ctx.call_keywords().unwrap().unwrap();
        (
            kw.get("arg1").unwrap().as_str().unwrap().to_string(),
            kw.get("arg2").unwrap().as_str().unwrap().to_string(),
        )
    });
    assert_eq!(arg1, "spam");
    assert_eq!(arg2, "eggs");

    let none = first_context("x = 'x'", NodeKind::Str, |ctx| {
        ctx.call_keywords().unwrap().is_none()
    });
    assert!(none);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_check_call_arg_value` (adapted, see WP).
#[test]
fn test_check_call_arg_value() {
    first_context("f(spam='eggs')", NodeKind::Call, |ctx| {
        assert_eq!(
            ctx.check_call_arg_value("spam", &[PyValue::str("eggs")])
                .unwrap(),
            Some(true)
        );
        assert_eq!(
            ctx.check_call_arg_value("spam", &[PyValue::str("spam"), PyValue::str("eggs")])
                .unwrap(),
            Some(true)
        );
        assert_eq!(
            ctx.check_call_arg_value("spam", &[PyValue::str("spam")])
                .unwrap(),
            Some(false)
        );
        // Python's `check_call_arg_value("spam")` defaults `argument_values`
        // to `None`, compared as `[None]`; the Rust port has no default
        // parameter, so `&[PyValue::None]` spells out the same comparison.
        assert_eq!(
            ctx.check_call_arg_value("spam", &[PyValue::None]).unwrap(),
            Some(false)
        );
        // Python's `check_call_arg_value("eggs")` returns `None` (the
        // argument is absent) and `assertFalse(None)` passes because `None`
        // is falsy; the Rust `Result<Option<bool>, _>` keeps "absent" and
        // "found but False" distinct, so this asserts the more precise
        // `None` rather than `Some(false)`.
        assert_eq!(
            ctx.check_call_arg_value("eggs", &[PyValue::None]).unwrap(),
            None
        );
    });

    let none = first_context("x = 'x'", NodeKind::Str, |ctx| {
        ctx.check_call_arg_value("spam", &[PyValue::None]).unwrap()
    });
    assert_eq!(none, None);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_filename` (adapted, see WP).
#[test]
fn test_filename() {
    let mut filename = None;
    with_contexts("spam.py", "x = 'x'", NodeKind::Str, |ctx| {
        filename = Some(ctx.filename().to_string());
    });
    assert_eq!(filename.as_deref(), Some("spam.py"));
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_function_def_defaults_qual` (adapted, see WP).
#[test]
fn test_function_def_defaults_qual() {
    let defaults = first_context(
        "import spam\ndef f(a=spam.eggs): pass",
        NodeKind::FunctionDef,
        |ctx| ctx.function_def_defaults_qual(),
    );
    assert_eq!(defaults, vec!["spam.eggs".to_string()]);

    let defaults = first_context("def f(): pass", NodeKind::FunctionDef, |ctx| {
        ctx.function_def_defaults_qual()
    });
    assert_eq!(defaults, Vec::<String>::new());

    let defaults = first_context("x = 'x'", NodeKind::Str, |ctx| {
        ctx.function_def_defaults_qual()
    });
    assert_eq!(defaults, Vec::<String>::new());
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_get_call_arg_at_position` (adapted, see WP).
#[test]
fn test_get_call_arg_at_position() {
    let arg = first_context("f('spam')", NodeKind::Call, |ctx| {
        ctx.get_call_arg_at_position(0)
            .unwrap()
            .and_then(|v| v.as_str().map(str::to_string))
    });
    assert_eq!(arg.as_deref(), Some("spam"));

    let none = first_context("f('spam')", NodeKind::Call, |ctx| {
        ctx.get_call_arg_at_position(1).unwrap().is_none()
    });
    assert!(none);

    let none = first_context("f()", NodeKind::Call, |ctx| {
        ctx.get_call_arg_at_position(0).unwrap().is_none()
    });
    assert!(none);

    let none = first_context("x = 'x'", NodeKind::Str, |ctx| {
        ctx.get_call_arg_at_position(0).unwrap().is_none()
    });
    assert!(none);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_get_lineno_for_call_arg` (adapted, see WP).
#[test]
fn test_get_lineno_for_call_arg() {
    let lineno = first_context("f(\n    spam=1)", NodeKind::Call, |ctx| {
        ctx.get_lineno_for_call_arg("spam")
    });
    assert_eq!(lineno, Some(2));

    let lineno = first_context("f(\n    spam=1)", NodeKind::Call, |ctx| {
        ctx.get_lineno_for_call_arg("eggs")
    });
    assert_eq!(lineno, None);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_is_module_being_imported` (adapted, see WP).
#[test]
fn test_is_module_being_imported() {
    let (spam, eggs) = first_context("import spam", NodeKind::Import, |ctx| {
        (
            ctx.is_module_being_imported("spam"),
            ctx.is_module_being_imported("eggs"),
        )
    });
    assert!(spam);
    assert!(!eggs);

    let no_module = first_context("x = 'x'", NodeKind::Str, |ctx| {
        ctx.is_module_being_imported("spam")
    });
    assert!(!no_module);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_is_module_imported_exact` (adapted, see WP).
#[test]
fn test_is_module_imported_exact() {
    let (spam, eggs) = first_context("import spam\nf()", NodeKind::Call, |ctx| {
        (
            ctx.is_module_imported_exact("spam"),
            ctx.is_module_imported_exact("eggs"),
        )
    });
    assert!(spam);
    assert!(!eggs);

    let no_import = first_context("f()", NodeKind::Call, |ctx| {
        ctx.is_module_imported_exact("spam")
    });
    assert!(!no_import);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_is_module_imported_like` (adapted, see WP).
#[test]
fn test_is_module_imported_like() {
    let (os, bacon) = first_context("import os.path\nf()", NodeKind::Call, |ctx| {
        (
            ctx.is_module_imported_like("os"),
            ctx.is_module_imported_like("bacon"),
        )
    });
    assert!(os);
    assert!(!bacon);

    let no_import = first_context("f()", NodeKind::Call, |ctx| {
        ctx.is_module_imported_like("spam")
    });
    assert!(!no_import);
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_node` (adapted, see WP).
#[test]
fn test_node() {
    let kind = first_context("spam()", NodeKind::Call, |ctx| ctx.node.map(VNode::kind));
    assert_eq!(kind, Some(NodeKind::Call));

    let kind = first_context("x = 'spam'", NodeKind::Str, |ctx| ctx.node.map(VNode::kind));
    assert_eq!(kind, Some(NodeKind::Str));
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_statement` (adapted, see WP).
///
/// Upstream `Context.statement` is `self._context.get("statement")`, always
/// `None` in real bandit (`node_visitor.py` never sets that key); this WP
/// gives `Context::statement` a real implementation — the nearest enclosing
/// `Stmt` ancestor — so the property is testable (DEVIATIONS.md #13).
#[test]
fn test_statement() {
    let class_name = first_context("x = spam()", NodeKind::Call, |ctx| {
        ctx.statement().map(VNode::class_name)
    });
    assert_eq!(class_name, Some("Assign"));
}

/// Port of `tests/unit/core/test_context.py::ContextTests::test_string_val` (adapted, see WP).
#[test]
fn test_string_val() {
    let s = first_context("x = 'spam'", NodeKind::Str, |ctx| {
        ctx.string_val().map(str::to_string)
    });
    assert_eq!(s.as_deref(), Some("spam"));

    let none = first_context("f()", NodeKind::Call, |ctx| ctx.string_val().is_none());
    assert!(none);
}
