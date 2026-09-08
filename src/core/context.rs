//! The context handed to tests (port of `bandit/core/context.py`).

use std::borrow::Cow;

use ruff_python_ast::{Expr, ExprCall, StmtFunctionDef};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::ast::literal::{PyErr, PyValue, get_literal_value, keyword_value};
use crate::ast::qualname::{Aliases, qual_attr};
use crate::ast::{Pos, VNode};
use crate::core::issue::LineRange;
use crate::core::utils::escaped_bytes_representation;
use crate::pycompat::unicode_escape;
use crate::source::SourceFile;

/// `call_keywords`: a Python dict keyed by keyword name (`None` for `**kw`).
#[derive(Debug, Clone, Default)]
pub struct Keywords<'a>(pub Vec<(Option<&'a str>, PyValue<'a>)>);

impl<'a> Keywords<'a> {
    /// Dict lookup (later duplicates win, like Python dict assignment).
    pub fn get(&self, name: &str) -> Option<&PyValue<'a>> {
        self.0
            .iter()
            .rev()
            .find(|(k, _)| *k == Some(name))
            .map(|(_, v)| v)
    }

    /// `name in call_keywords`.
    pub fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|(k, _)| *k == Some(name))
    }

    /// `call_keywords.get(name, default)`.
    pub fn get_or<'b>(&'b self, name: &str, default: &'b PyValue<'a>) -> &'b PyValue<'a> {
        self.get(name).unwrap_or(default)
    }
}

/// The per-node context.
pub struct Context<'a, 'w> {
    pub file: &'w SourceFile,
    /// `None` for the synthetic file-level context.
    pub node: Option<VNode<'a>>,
    /// Ancestors from the module (index 0) down to the parent (last).
    pub ancestors: &'w [VNode<'a>],
    /// Next sibling in the parent's list field.
    pub sibling: Option<VNode<'a>>,
    /// `lineno`/`col_offset`/`end_col_offset` when the node has a position.
    pub pos: Option<Pos>,
    pub linerange: LineRange,
    pub call: Option<&'a ExprCall>,
    pub function: Option<&'a StmtFunctionDef>,
    pub qualname: Option<&'w str>,
    pub name: Option<&'w str>,
    pub module: Option<&'w str>,
    pub str_val: Option<&'a str>,
    pub bytes_val: Option<Cow<'a, [u8]>>,
    pub imports: &'w FxHashSet<String>,
    pub import_aliases: &'w Aliases,
}

impl<'a, 'w> Context<'a, 'w> {
    /// `context["lineno"]` (absent for position-less nodes).
    pub fn lineno(&self) -> Option<u32> {
        self.pos.map(|p| p.lineno)
    }

    /// `context["col_offset"]`.
    pub fn col_offset(&self) -> Option<u32> {
        self.pos.map(|p| p.col_offset)
    }

    /// `context.get("end_col_offset", 0)`.
    pub fn end_col_offset(&self) -> u32 {
        self.pos.map(|p| p.end_col_offset).unwrap_or(0)
    }

    pub fn filename(&self) -> &str {
        &self.file.name
    }

    /// `node._bandit_parent`.
    pub fn parent(&self) -> Option<VNode<'a>> {
        self.ancestors.last().copied()
    }

    /// `n`-th ancestor (1 = parent, 2 = grandparent, ...).
    pub fn ancestor(&self, n: usize) -> Option<VNode<'a>> {
        let len = self.ancestors.len();
        if n == 0 || n > len {
            None
        } else {
            Some(self.ancestors[len - n])
        }
    }

    /// `call_args`: positional argument values (attribute name for
    /// attribute nodes, literal value otherwise).
    pub fn call_args(&self) -> Result<Vec<PyValue<'a>>, PyErr> {
        match self.call {
            Some(call) => call.arguments.args.iter().map(keyword_value).collect(),
            None => Ok(Vec::new()),
        }
    }

    /// `call_args_count`.
    pub fn call_args_count(&self) -> Option<usize> {
        self.call.map(|c| c.arguments.args.len())
    }

    /// `call_function_name`.
    pub fn call_function_name(&self) -> Option<&str> {
        self.name
    }

    /// `call_function_name_qual`.
    pub fn call_function_name_qual(&self) -> Option<&str> {
        self.qualname
    }

    /// `call_keywords` (`None` when there is no call).
    pub fn call_keywords(&self) -> Result<Option<Keywords<'a>>, PyErr> {
        let Some(call) = self.call else {
            return Ok(None);
        };
        let mut out = Vec::with_capacity(call.arguments.keywords.len());
        for kw in &call.arguments.keywords {
            out.push((
                kw.arg.as_ref().map(|a| a.as_str()),
                keyword_value(&kw.value)?,
            ));
        }
        Ok(Some(Keywords(out)))
    }

    /// `get_call_arg_value(name)`.
    pub fn get_call_arg_value(&self, name: &str) -> Result<Option<PyValue<'a>>, PyErr> {
        Ok(self.call_keywords()?.and_then(|kws| kws.get(name).cloned()))
    }

    /// `check_call_arg_value(name, values)`: `None` when the argument is
    /// absent (or its value is Python `None`), otherwise whether it equals
    /// one of `values`.
    pub fn check_call_arg_value(
        &self,
        name: &str,
        values: &[PyValue<'_>],
    ) -> Result<Option<bool>, PyErr> {
        match self.get_call_arg_value(name)? {
            Some(v) if !v.is_none() => Ok(Some(values.iter().any(|x| v.py_eq(x)))),
            _ => Ok(None),
        }
    }

    /// `check_call_arg_value(name)` with a single value.
    pub fn check_call_arg_is(
        &self,
        name: &str,
        value: &PyValue<'_>,
    ) -> Result<Option<bool>, PyErr> {
        self.check_call_arg_value(name, std::slice::from_ref(value))
    }

    /// `get_lineno_for_call_arg(name)`: line of the keyword's value.
    pub fn get_lineno_for_call_arg(&self, name: &str) -> Option<u32> {
        let call = self.call?;
        call.arguments
            .keywords
            .iter()
            .find(|k| k.arg.as_ref().is_some_and(|a| a.as_str() == name))
            .map(|k| self.file.line_index(k.value.start().to_u32()))
    }

    /// `get_call_arg_at_position(n)`.
    pub fn get_call_arg_at_position(&self, n: usize) -> Result<Option<PyValue<'a>>, PyErr> {
        let Some(call) = self.call else {
            return Ok(None);
        };
        match call.arguments.args.get(n) {
            Some(arg) => Ok(Some(crate::ast::literal::arg_value(arg)?)),
            None => Ok(None),
        }
    }

    /// Raw positional argument node.
    pub fn call_arg_node(&self, n: usize) -> Option<&'a Expr> {
        self.call.and_then(|c| c.arguments.args.get(n))
    }

    /// `is_module_being_imported(module)`.
    pub fn is_module_being_imported(&self, module: &str) -> bool {
        self.module == Some(module)
    }

    /// `is_module_imported_exact(module)`.
    pub fn is_module_imported_exact(&self, module: &str) -> bool {
        self.imports.contains(module)
    }

    /// `is_module_imported_like(module)`: substring match over the imports.
    pub fn is_module_imported_like(&self, module: &str) -> bool {
        self.imports.iter().any(|imp| imp.contains(module))
    }

    /// `string_val`.
    pub fn string_val(&self) -> Option<&'a str> {
        self.str_val
    }

    /// `bytes_val`.
    pub fn bytes_val(&self) -> Option<&[u8]> {
        self.bytes_val.as_deref()
    }

    /// `string_val_as_escaped_bytes`.
    pub fn string_val_as_escaped_bytes(&self) -> Option<Vec<u8>> {
        if let Some(s) = self.str_val {
            return Some(unicode_escape::encode(s));
        }
        self.bytes_val
            .as_deref()
            .and_then(|b| escaped_bytes_representation(b).ok())
    }

    /// `function_def_defaults_qual`.
    pub fn function_def_defaults_qual(&self) -> Vec<String> {
        let Some(f) = self.function else {
            return Vec::new();
        };
        f.parameters
            .posonlyargs
            .iter()
            .chain(f.parameters.args.iter())
            .filter_map(|p| p.default.as_deref())
            .map(|d| qual_attr(d, self.import_aliases))
            .collect()
    }

    /// Literal value of an arbitrary expression (`_get_literal_value`).
    pub fn literal_value(&self, expr: &'a Expr) -> Result<PyValue<'a>, PyErr> {
        get_literal_value(expr)
    }

    /// Start line of an arbitrary expression of this file.
    pub fn line_of(&self, expr: &Expr) -> u32 {
        self.file.line_index(expr.start().to_u32())
    }
}
