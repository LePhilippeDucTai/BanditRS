//! A CPython-flavoured view over ruff's Python AST.
//!
//! bandit's behaviour is defined in terms of CPython's `ast` module: the
//! traversal order of `ast.iter_fields`, the node class names, the presence or
//! absence of positions, the merging of literal segments in f-strings, etc.
//! This module reproduces those semantics on top of ruff's AST so that the
//! scanning engine sees the same tree Python bandit would.

pub mod children;
pub mod joined_str;
pub mod linerange;
pub mod literal;
pub mod positions;
pub mod qualname;
pub mod trace;
pub mod vnode;
pub mod walker;

pub use joined_str::{JoinedPart, JoinedStrView};
pub use positions::Pos;
pub use vnode::{NodeKind, VNode};

use ruff_python_ast::PythonVersion;

/// Which CPython version's AST positions to emulate.
///
/// CPython 3.12 (PEP 701) reports real positions for the literal segments of
/// f-strings; CPython 3.11 gives every value of a `JoinedStr` the range of the
/// whole string. The default follows modern CPython; the 3.11 policy exists
/// so that results can be compared byte-for-byte with a Python 3.11 bandit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PyCompat {
    Py311,
    #[default]
    Py312,
}

impl PyCompat {
    /// Read `BANDITRS_PYTHON_COMPAT` (`3.11` or `3.12`, anything else means
    /// the default).
    pub fn from_env() -> PyCompat {
        match std::env::var("BANDITRS_PYTHON_COMPAT").as_deref() {
            Ok("3.11") | Ok("311") | Ok("py311") => PyCompat::Py311,
            _ => PyCompat::Py312,
        }
    }

    /// Target version handed to the parser (version-specific syntax errors).
    pub fn target_version(self) -> PythonVersion {
        match self {
            PyCompat::Py311 => PythonVersion::PY311,
            PyCompat::Py312 => PythonVersion::latest(),
        }
    }
}
