//! Port of `tests/unit/core/test_util.py` (`bandit.core.utils`).
//!
//! Work package: `docs/plan/wp/WP-07-unit-core-util.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/unit/core/test_util.py`.

use std::fs;

use banditrs::ast::PyCompat;
use banditrs::ast::children::WalkCtx;
use banditrs::ast::joined_str::ViewArena;
use banditrs::ast::linerange::linerange;
use banditrs::ast::qualname::{Aliases, attr_qual_name, call_name};
use banditrs::ast::vnode::{NodeKind, VNode};
use banditrs::core::utils::{
    escaped_bytes_representation, get_module_qualname_from_path, namespace_path_join,
    namespace_path_split,
};
use banditrs::pycompat::configparser::parse_ini_file;
use banditrs::source::SourceFile;
use ruff_python_ast::Expr;

/// Touch (create) an empty file at `path` (`_touch` in the Python test).
fn touch(path: &std::path::Path) {
    fs::write(path, "").unwrap();
}

/// Port of `UtilTests._setup_get_module_qualname_from_path`: build the
/// `good`/`missingmid`/`missingend`/`syms` fixture tree under a fresh
/// `TempDir`, returning `(tempdir, reltempdir)` — `reltempdir` mirrors
/// Python's `os.path.relpath(self.tempdir)` (relative to the process cwd,
/// which the tests never change).
fn setup_get_module_qualname_from_path() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let reltempdir = banditrs::pycompat::path::relpath(root.to_str().unwrap(), None);

    // good/a/b/c/test_typical.py
    fs::create_dir_all(root.join("good/a/b/c")).unwrap();
    touch(&root.join("good/__init__.py"));
    touch(&root.join("good/a/__init__.py"));
    touch(&root.join("good/a/b/__init__.py"));
    touch(&root.join("good/a/b/c/__init__.py"));
    touch(&root.join("good/a/b/c/test_typical.py"));

    // missingmid/a/b/c/test_missingmid.py (no missingmid/a/__init__.py)
    fs::create_dir_all(root.join("missingmid/a/b/c")).unwrap();
    touch(&root.join("missingmid/__init__.py"));
    touch(&root.join("missingmid/a/b/__init__.py"));
    touch(&root.join("missingmid/a/b/c/__init__.py"));
    touch(&root.join("missingmid/a/b/c/test_missingmid.py"));

    // missingend/a/b/c/test_missingend.py (no missingend/a/b/c/__init__.py)
    fs::create_dir_all(root.join("missingend/a/b/c")).unwrap();
    touch(&root.join("missingend/__init__.py"));
    touch(&root.join("missingend/a/b/__init__.py"));
    touch(&root.join("missingend/a/b/c/test_missingend.py"));

    // syms/a/bsym/c/test_typical.py -> good/a/b (symlink)
    fs::create_dir_all(root.join("syms/a")).unwrap();
    touch(&root.join("syms/__init__.py"));
    touch(&root.join("syms/a/__init__.py"));
    std::os::unix::fs::symlink(root.join("good/a/b"), root.join("syms/a/bsym")).unwrap();

    (dir, reltempdir)
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_abs_typical`.
#[test]
fn test_get_module_qualname_from_path_abs_typical() {
    let (dir, _rel) = setup_get_module_qualname_from_path();
    let path = dir.path().join("good/a/b/c/test_typical.py");
    let name = get_module_qualname_from_path(path.to_str().unwrap()).unwrap();
    assert_eq!("good.a.b.c.test_typical", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_with_dot`.
#[test]
fn test_get_module_qualname_from_path_with_dot() {
    let name = get_module_qualname_from_path("./__init__.py").unwrap();
    assert_eq!("__init__", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_abs_missingmid`.
#[test]
fn test_get_module_qualname_from_path_abs_missingmid() {
    let (dir, _rel) = setup_get_module_qualname_from_path();
    let path = dir.path().join("missingmid/a/b/c/test_missingmid.py");
    let name = get_module_qualname_from_path(path.to_str().unwrap()).unwrap();
    assert_eq!("b.c.test_missingmid", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_abs_missingend`.
#[test]
fn test_get_module_qualname_from_path_abs_missingend() {
    let (dir, _rel) = setup_get_module_qualname_from_path();
    let path = dir.path().join("missingend/a/b/c/test_missingend.py");
    let name = get_module_qualname_from_path(path.to_str().unwrap()).unwrap();
    assert_eq!("test_missingend", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_abs_syms`.
#[test]
fn test_get_module_qualname_from_path_abs_syms() {
    let (dir, _rel) = setup_get_module_qualname_from_path();
    let path = dir.path().join("syms/a/bsym/c/test_typical.py");
    let name = get_module_qualname_from_path(path.to_str().unwrap()).unwrap();
    assert_eq!("syms.a.bsym.c.test_typical", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_rel_typical`.
#[test]
fn test_get_module_qualname_from_path_rel_typical() {
    let (_dir, rel) = setup_get_module_qualname_from_path();
    let path = format!("{rel}/good/a/b/c/test_typical.py");
    let name = get_module_qualname_from_path(&path).unwrap();
    assert_eq!("good.a.b.c.test_typical", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_rel_missingmid`.
#[test]
fn test_get_module_qualname_from_path_rel_missingmid() {
    let (_dir, rel) = setup_get_module_qualname_from_path();
    let path = format!("{rel}/missingmid/a/b/c/test_missingmid.py");
    let name = get_module_qualname_from_path(&path).unwrap();
    assert_eq!("b.c.test_missingmid", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_rel_missingend`.
#[test]
fn test_get_module_qualname_from_path_rel_missingend() {
    let (_dir, rel) = setup_get_module_qualname_from_path();
    let path = format!("{rel}/missingend/a/b/c/test_missingend.py");
    let name = get_module_qualname_from_path(&path).unwrap();
    assert_eq!("test_missingend", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_rel_syms`.
#[test]
fn test_get_module_qualname_from_path_rel_syms() {
    let (_dir, rel) = setup_get_module_qualname_from_path();
    let path = format!("{rel}/syms/a/bsym/c/test_typical.py");
    let name = get_module_qualname_from_path(&path).unwrap();
    assert_eq!("syms.a.bsym.c.test_typical", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_sys` (adapted:
/// `os.__file__` is resolved via `python3 -c "import os; print(os.__file__)"` rather than the running
/// Python's `os` module, since the Rust process has no Python `sys.modules`).
#[test]
fn test_get_module_qualname_from_path_sys() {
    let output = std::process::Command::new("python3")
        .args(["-c", "import os; print(os.__file__)"])
        .output()
        .expect("python3 should be available");
    assert!(output.status.success());
    let os_file = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let name = get_module_qualname_from_path(&os_file).unwrap();
    assert_eq!("os", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_invalid_path`.
#[test]
fn test_get_module_qualname_from_path_invalid_path() {
    let name = get_module_qualname_from_path("/a/b/c/d/e.py").unwrap();
    assert_eq!("e", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_module_qualname_from_path_dir`.
#[test]
fn test_get_module_qualname_from_path_dir() {
    assert!(get_module_qualname_from_path("/tmp/").is_err());
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_namespace_path_join`.
#[test]
fn test_namespace_path_join() {
    let p = namespace_path_join("base1.base2", "name");
    assert_eq!("base1.base2.name", p);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_namespace_path_split`.
#[test]
fn test_namespace_path_split() {
    let (head, tail) = namespace_path_split("base1.base2.name");
    assert_eq!("base1.base2", head);
    assert_eq!(Some("name".to_string()), tail);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_call_name1`.
#[test]
fn test_get_call_name1() {
    let tree = ruff_python_parser::parse_expression("a.b.c.d(x,y)")
        .unwrap()
        .into_expr();
    let Expr::Call(call) = &tree else {
        panic!("expected a call expression")
    };
    let name = call_name(call, &Aliases::default());
    assert_eq!("a.b.c.d", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_call_name2`.
#[test]
fn test_get_call_name2() {
    let tree = ruff_python_parser::parse_expression("a.b.c.d(x,y)")
        .unwrap()
        .into_expr();
    let Expr::Call(call) = &tree else {
        panic!("expected a call expression")
    };

    let aliases: Aliases = [("a".to_string(), "alias.x.y".to_string())]
        .into_iter()
        .collect();
    let name = call_name(call, &aliases);
    assert_eq!("alias.x.y.b.c.d", name);

    let aliases: Aliases = [("a.b".to_string(), "alias.x.y".to_string())]
        .into_iter()
        .collect();
    let name = call_name(call, &aliases);
    assert_eq!("alias.x.y.c.d", name);

    let aliases: Aliases = [("a.b.c.d".to_string(), "alias.x.y".to_string())]
        .into_iter()
        .collect();
    let name = call_name(call, &aliases);
    assert_eq!("alias.x.y", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_get_call_name3` (adapted: `_get_attr_qual_name`
/// is private in Python and exposed here as `banditrs::ast::qualname::attr_qual_name`).
#[test]
fn test_get_call_name3() {
    let tree = ruff_python_parser::parse_expression("a.list[0](x,y)")
        .unwrap()
        .into_expr();
    let Expr::Call(call) = &tree else {
        panic!("expected a call expression")
    };
    let name = attr_qual_name(&call.func, &Aliases::default());
    assert_eq!("", name);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_linerange`.
#[test]
fn test_linerange() {
    let text = fs::read_to_string("./examples/jinja2_templating.py").unwrap();
    let parsed =
        banditrs::source::parse::parse_module(&text, PyCompat::Py312).expect("parses cleanly");
    let module = parsed.syntax();

    let arena = ViewArena::new();
    let ctx = WalkCtx::new(&arena, &text, PyCompat::Py312);
    let file = SourceFile::new("./examples/jinja2_templating.py", text.clone());

    // `tree.body[8]` (0-indexed): the ninth top-level statement.
    let stmt = &module.body[8];
    let sibling = module.body.get(9).map(VNode::Stmt);
    let lrange = linerange(VNode::Stmt(stmt), sibling, None, &file, &ctx);

    // line 9 should be three lines long.
    assert_eq!(3, lrange.len());
    // the range should be the correct line numbers.
    assert_eq!(vec![11u32, 12, 13], lrange.to_vec());
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_escaped_representation_simple`.
#[test]
fn test_escaped_representation_simple() {
    let res = escaped_bytes_representation(b"ascii").unwrap();
    assert_eq!(res, b"ascii");
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_escaped_representation_valid_not_printable`.
#[test]
fn test_escaped_representation_valid_not_printable() {
    let res = escaped_bytes_representation(b"\\u0000").unwrap();
    assert_eq!(res, b"\\x00");
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_escaped_representation_invalid`.
#[test]
fn test_escaped_representation_invalid() {
    let res = escaped_bytes_representation(b"\\uffff").unwrap();
    assert_eq!(res, b"\\uffff");
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_escaped_representation_mixed`.
#[test]
fn test_escaped_representation_mixed() {
    let res = escaped_bytes_representation(b"ascii\\u0000\\uffff").unwrap();
    assert_eq!(res, b"ascii\\x00\\uffff");
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_parse_ini_file`.
#[test]
fn test_parse_ini_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bandit.ini");

    fs::write(&path, "[bandit]\nexclude=/abc,/def").unwrap();
    let expected: indexmap::IndexMap<String, String> =
        [("exclude".to_string(), "/abc,/def".to_string())]
            .into_iter()
            .collect();
    let got = parse_ini_file(path.to_str().unwrap());
    assert_eq!(got, Some(expected));

    fs::write(&path, "[Blabla]\nsomething=something").unwrap();
    assert_eq!(parse_ini_file(path.to_str().unwrap()), None);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_check_ast_node_good` (adapted: Python's
/// `check_ast_node` is a runtime `getattr(ast, name)` check; the Rust port pre-registers the node
/// kinds a plugin can dispatch on as `NodeKind`, so `check_ast_node("X")` becomes `NodeKind::parse("X")`).
#[test]
fn test_check_ast_node_good() {
    let node = NodeKind::parse("Call");
    assert_eq!(Some(NodeKind::Call), node);
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_check_ast_node_bad_node` (adapted, see
/// `test_check_ast_node_good`: Python's `TypeError` becomes `None`).
#[test]
fn test_check_ast_node_bad_node() {
    assert_eq!(None, NodeKind::parse("Derp"));
}

/// Port of `tests/unit/core/test_util.py::UtilTests::test_check_ast_node_bad_type` (adapted, see
/// `test_check_ast_node_good`: Python's `TypeError` becomes `None`).
#[test]
fn test_check_ast_node_bad_type() {
    assert_eq!(None, NodeKind::parse("walk"));
}

// Not portable (see docs/plan/wp/WP-07-unit-core-util.md):
// - test_path_for_function / test_path_for_function_no_file / test_path_for_function_no_module:
//   `get_path_for_function` introspects a Python function object's `__code__.co_filename`; there is no
//   Rust equivalent (Rust functions have no runtime-inspectable source path).
// - test_deepgetattr: `deepgetattr` walks dotted attribute names via `getattr`; BanditRS has no runtime
//   object/attribute model to walk (plugins operate on the AST, not live Python objects).
