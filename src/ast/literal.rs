//! Literal values as bandit's `Context._get_literal_value` produces them,
//! with the Python semantics plugins rely on (`==`, truthiness, `str()`).

use std::borrow::Cow;

use ruff_python_ast::{self as ast, Expr, Number};

/// Error raised where Python would raise (e.g. an unhashable set element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyErr(pub String);

impl PyErr {
    pub fn type_error(msg: impl Into<String>) -> PyErr {
        PyErr(format!("TypeError: {}", msg.into()))
    }
    pub fn key_error(key: impl std::fmt::Display) -> PyErr {
        PyErr(format!("KeyError: '{key}'"))
    }
    pub fn index_error(msg: impl Into<String>) -> PyErr {
        PyErr(format!("IndexError: {}", msg.into()))
    }
    pub fn attribute_error(msg: impl Into<String>) -> PyErr {
        PyErr(format!("AttributeError: {}", msg.into()))
    }
}

impl std::fmt::Display for PyErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A Python value derived from an AST literal.
#[derive(Debug, Clone)]
pub enum PyValue<'a> {
    /// Python `None` (unsupported node types).
    None,
    /// `str` (booleans and `None` constants are converted to their string
    /// representation by bandit).
    Str(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Int(i128),
    /// Integer too large for `i128`, kept as decimal text.
    BigInt(String),
    Float(f64),
    Complex(f64, f64),
    Ellipsis,
    List(Vec<PyValue<'a>>),
    Tuple(Vec<PyValue<'a>>),
    Set(Vec<PyValue<'a>>),
    /// `dict(zip(keys, values))` of raw AST nodes.
    Dict(&'a ast::ExprDict),
}

impl<'a> PyValue<'a> {
    pub fn str(s: impl Into<Cow<'a, str>>) -> PyValue<'a> {
        PyValue::Str(s.into())
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            PyValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, PyValue::None)
    }

    pub fn is_str(&self) -> bool {
        matches!(self, PyValue::Str(_))
    }

    /// `isinstance(v, int)` (booleans are strings here, so never ints).
    pub fn as_int(&self) -> Option<i128> {
        match self {
            PyValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[PyValue<'a>]> {
        match self {
            PyValue::List(l) => Some(l),
            _ => None,
        }
    }

    fn is_hashable(&self) -> bool {
        match self {
            PyValue::List(_) | PyValue::Set(_) => false,
            PyValue::Dict(_) => false,
            PyValue::Tuple(t) => t.iter().all(PyValue::is_hashable),
            _ => true,
        }
    }

    /// Python truthiness.
    pub fn truthy(&self) -> bool {
        match self {
            PyValue::None => false,
            PyValue::Str(s) => !s.is_empty(),
            PyValue::Bytes(b) => !b.is_empty(),
            PyValue::Int(i) => *i != 0,
            PyValue::BigInt(_) => true,
            PyValue::Float(f) => *f != 0.0,
            PyValue::Complex(r, i) => *r != 0.0 || *i != 0.0,
            PyValue::Ellipsis => true,
            PyValue::List(v) | PyValue::Tuple(v) | PyValue::Set(v) => !v.is_empty(),
            PyValue::Dict(d) => !d.items.is_empty(),
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            PyValue::Int(i) => Some(*i as f64),
            PyValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Python `==`.
    pub fn py_eq(&self, other: &PyValue<'_>) -> bool {
        match (self, other) {
            (PyValue::None, PyValue::None) => true,
            (PyValue::Ellipsis, PyValue::Ellipsis) => true,
            (PyValue::Str(a), PyValue::Str(b)) => a == b,
            (PyValue::Bytes(a), PyValue::Bytes(b)) => a == b,
            (PyValue::Int(a), PyValue::Int(b)) => a == b,
            (PyValue::BigInt(a), PyValue::BigInt(b)) => a == b,
            (PyValue::Float(a), PyValue::Float(b)) => a == b,
            (PyValue::Int(_), PyValue::Float(_)) | (PyValue::Float(_), PyValue::Int(_)) => {
                self.as_f64() == other.as_f64()
            }
            (PyValue::Complex(r1, i1), PyValue::Complex(r2, i2)) => r1 == r2 && i1 == i2,
            (PyValue::Complex(r, i), o) | (o, PyValue::Complex(r, i)) => {
                *i == 0.0 && o.as_f64().is_some_and(|f| f == *r)
            }
            (PyValue::List(a), PyValue::List(b)) | (PyValue::Tuple(a), PyValue::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.py_eq(y))
            }
            (PyValue::Set(a), PyValue::Set(b)) => {
                a.len() == b.len() && a.iter().all(|x| b.iter().any(|y| x.py_eq(y)))
            }
            (PyValue::Dict(a), PyValue::Dict(b)) => std::ptr::eq(*a, *b),
            _ => false,
        }
    }

    /// Python `str(value)`.
    pub fn py_str(&self) -> String {
        match self {
            PyValue::Str(s) => s.to_string(),
            _ => self.py_repr(),
        }
    }

    /// Python `repr(value)`.
    pub fn py_repr(&self) -> String {
        match self {
            PyValue::None => "None".to_string(),
            PyValue::Str(s) => repr_str(s),
            PyValue::Bytes(b) => repr_bytes(b),
            PyValue::Int(i) => i.to_string(),
            PyValue::BigInt(s) => s.clone(),
            PyValue::Float(f) => repr_float(*f),
            PyValue::Complex(r, i) => {
                if *r == 0.0 && !r.is_sign_negative() {
                    format!("{}j", repr_float_short(*i))
                } else {
                    let sign = if *i < 0.0 || (*i == 0.0 && i.is_sign_negative()) {
                        "-"
                    } else {
                        "+"
                    };
                    format!(
                        "({}{}{}j)",
                        repr_float_short(*r),
                        sign,
                        repr_float_short(i.abs())
                    )
                }
            }
            PyValue::Ellipsis => "Ellipsis".to_string(),
            PyValue::List(v) => format!(
                "[{}]",
                v.iter()
                    .map(PyValue::py_repr)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            PyValue::Tuple(v) => {
                if v.len() == 1 {
                    format!("({},)", v[0].py_repr())
                } else {
                    format!(
                        "({})",
                        v.iter()
                            .map(PyValue::py_repr)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            PyValue::Set(v) => {
                if v.is_empty() {
                    "set()".to_string()
                } else {
                    format!(
                        "{{{}}}",
                        v.iter()
                            .map(PyValue::py_repr)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            PyValue::Dict(d) => {
                let items: Vec<String> = d
                    .items
                    .iter()
                    .map(|it| {
                        let k = it
                            .key
                            .as_ref()
                            .map(node_repr)
                            .unwrap_or_else(|| "None".to_string());
                        format!("{k}: {}", node_repr(&it.value))
                    })
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
        }
    }
}

/// `repr()` of a raw AST node (`<ast.Name object at 0x...>`); the address is
/// rendered deterministically.
pub fn node_repr(expr: &Expr) -> String {
    format!(
        "<ast.{} object at 0x0>",
        crate::ast::VNode::Expr(expr).class_name()
    )
}

/// Whether Python's `str.isprintable()` considers `c` printable
/// (approximation covering the control/format/separator characters that
/// occur in practice).
pub fn is_printable(c: char) -> bool {
    let u = c as u32;
    if u < 0x20 || (0x7f..=0xa0).contains(&u) {
        return false;
    }
    !matches!(
        u,
        0x00ad
            | 0x0600..=0x0605
            | 0x061c
            | 0x06dd
            | 0x070f
            | 0x0890..=0x0891
            | 0x08e2
            | 0x1680
            | 0x180e
            | 0x2000..=0x200f
            | 0x2028..=0x202f
            | 0x205f..=0x2064
            | 0x2066..=0x206f
            | 0x3000
            | 0xd800..=0xf8ff
            | 0xfeff
            | 0xfff9..=0xfffb
            | 0x110bd
            | 0x110cd
            | 0x13430..=0x1343f
            | 0x1bca0..=0x1bca3
            | 0x1d173..=0x1d17a
            | 0xe0001
            | 0xe0020..=0xe007f
            | 0xf0000..=0x10ffff
    )
}

/// Python `repr(str)`.
pub fn repr_str(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if is_printable(c) => out.push(c),
            c => {
                let u = c as u32;
                if u < 0x100 {
                    out.push_str(&format!("\\x{u:02x}"));
                } else if u < 0x10000 {
                    out.push_str(&format!("\\u{u:04x}"));
                } else {
                    out.push_str(&format!("\\U{u:08x}"));
                }
            }
        }
    }
    out.push(quote);
    out
}

/// Python `repr(bytes)`.
pub fn repr_bytes(b: &[u8]) -> String {
    let quote = if b.contains(&b'\'') && !b.contains(&b'"') {
        b'"'
    } else {
        b'\''
    };
    let mut out = String::from("b");
    out.push(quote as char);
    for &c in b {
        match c {
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c as char);
            }
            0x20..=0x7e => out.push(c as char),
            c => out.push_str(&format!("\\x{c:02x}")),
        }
    }
    out.push(quote as char);
    out
}

/// Python `repr(float)`.
pub fn repr_float(f: f64) -> String {
    if f.is_nan() {
        return "nan".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    let s = format!("{f:?}");
    // Rust: "1e16" / "1e-5"; Python: "1e+16" / "1e-05".
    if let Some((mant, exp)) = s.split_once('e') {
        let (sign, digits) = match exp.strip_prefix('-') {
            Some(d) => ("-", d),
            None => ("+", exp),
        };
        let digits = if digits.len() < 2 {
            format!("0{digits}")
        } else {
            digits.to_string()
        };
        let mant = mant.strip_suffix(".0").unwrap_or(mant);
        return format!("{mant}e{sign}{digits}");
    }
    s
}

/// Float formatting inside complex numbers (`1.0` renders as `1`).
fn repr_float_short(f: f64) -> String {
    let s = repr_float(f);
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

/// `Context._get_literal_value(node)`.
pub fn get_literal_value<'a>(expr: &'a Expr) -> Result<PyValue<'a>, PyErr> {
    Ok(match expr {
        Expr::StringLiteral(s) => PyValue::Str(Cow::Borrowed(s.value.to_str())),
        Expr::BytesLiteral(b) => {
            if b.value.is_implicit_concatenated() {
                PyValue::Bytes(Cow::Owned(b.value.bytes().collect()))
            } else {
                let first = b.value.iter().next().map(|p| p.as_slice()).unwrap_or(&[]);
                PyValue::Bytes(Cow::Borrowed(first))
            }
        }
        Expr::NumberLiteral(n) => match &n.value {
            Number::Int(i) => match i.as_i64() {
                Some(v) => PyValue::Int(v as i128),
                None => {
                    let text = i.to_string();
                    match text.parse::<i128>() {
                        Ok(v) => PyValue::Int(v),
                        Err(_) => PyValue::BigInt(text),
                    }
                }
            },
            Number::Float(f) => PyValue::Float(*f),
            Number::Complex { real, imag } => PyValue::Complex(*real, *imag),
        },
        Expr::BooleanLiteral(b) => {
            PyValue::Str(Cow::Borrowed(if b.value { "True" } else { "False" }))
        }
        Expr::NoneLiteral(_) => PyValue::Str(Cow::Borrowed("None")),
        Expr::EllipsisLiteral(_) => PyValue::Ellipsis,
        Expr::List(l) => PyValue::List(
            l.elts
                .iter()
                .map(get_literal_value)
                .collect::<Result<_, _>>()?,
        ),
        Expr::Tuple(t) => PyValue::Tuple(
            t.elts
                .iter()
                .map(get_literal_value)
                .collect::<Result<_, _>>()?,
        ),
        Expr::Set(s) => {
            let mut items: Vec<PyValue<'a>> = Vec::with_capacity(s.elts.len());
            for e in &s.elts {
                let v = get_literal_value(e)?;
                if !v.is_hashable() {
                    return Err(PyErr::type_error("unhashable type"));
                }
                if !items.iter().any(|x| x.py_eq(&v)) {
                    items.push(v);
                }
            }
            PyValue::Set(items)
        }
        Expr::Dict(d) => PyValue::Dict(d),
        Expr::Name(n) => PyValue::Str(Cow::Borrowed(n.id.as_str())),
        _ => PyValue::None,
    })
}

/// `getattr(node, "attr", None) or _get_literal_value(node)` — the value of
/// a call argument as bandit sees it.
pub fn arg_value<'a>(expr: &'a Expr) -> Result<PyValue<'a>, PyErr> {
    if let Expr::Attribute(a) = expr {
        let attr = a.attr.as_str();
        if !attr.is_empty() {
            return Ok(PyValue::Str(Cow::Borrowed(attr)));
        }
    }
    get_literal_value(expr)
}

/// `node.attr if hasattr(node, "attr") else _get_literal_value(node)`
/// (used for keyword values; an empty attribute name still wins here).
pub fn keyword_value<'a>(expr: &'a Expr) -> Result<PyValue<'a>, PyErr> {
    if let Expr::Attribute(a) = expr {
        return Ok(PyValue::Str(Cow::Borrowed(a.attr.as_str())));
    }
    get_literal_value(expr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruff_python_parser::parse_expression;

    fn lit(src: &str) -> String {
        let e = parse_expression(src).unwrap().into_expr();
        get_literal_value(&e).unwrap().py_repr()
    }

    #[test]
    fn literal_values() {
        assert_eq!(lit("42"), "42");
        assert_eq!(lit("'spam'"), "'spam'");
        assert_eq!(lit("True"), "'True'");
        assert_eq!(lit("None"), "'None'");
        assert_eq!(lit("[1, 'a']"), "[1, 'a']");
        assert_eq!(lit("(1,)"), "(1,)");
        assert_eq!(lit("{1, 2}"), "{1, 2}");
        assert_eq!(lit("b'x'"), "b'x'");
        assert_eq!(lit("name"), "'name'");
        assert_eq!(lit("a.b"), "None");
        assert_eq!(lit("1.5"), "1.5");
        assert_eq!(lit("1e16"), "1e+16");
        assert_eq!(lit("3j"), "3j");
        assert_eq!(lit("'it\\'s'"), "\"it's\"");
        assert_eq!(lit("'\\u202e'"), "'\\u202e'");
        let e = parse_expression("{[1]}").unwrap().into_expr();
        assert!(get_literal_value(&e).is_err());
    }

    #[test]
    fn equality_and_truth() {
        assert!(PyValue::Int(0).py_eq(&PyValue::Float(0.0)));
        assert!(!PyValue::Int(0).py_eq(&PyValue::str("0")));
        assert!(
            PyValue::List(vec![PyValue::Int(1)]).py_eq(&PyValue::List(vec![PyValue::Float(1.0)]))
        );
        assert!(!PyValue::List(vec![]).py_eq(&PyValue::Tuple(vec![])));
        assert!(!PyValue::None.truthy());
        assert!(!PyValue::str("").truthy());
        assert!(PyValue::str("False").truthy());
        assert!(!PyValue::Int(0).truthy());
    }
}
