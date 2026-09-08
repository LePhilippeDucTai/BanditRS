//! Qualified name resolution (`utils.get_call_name`, `_get_attr_qual_name`,
//! `get_qual_attr`, `get_called_name`).

use ruff_python_ast::{Expr, ExprCall};
use rustc_hash::FxHashMap;

/// Alias map: local name → imported qualified name.
pub type Aliases = FxHashMap<String, String>;

/// `_get_attr_qual_name(node, aliases)` written into `buf`.
pub fn attr_qual_name_into(expr: &Expr, aliases: &Aliases, buf: &mut String) {
    match expr {
        Expr::Name(n) => {
            buf.clear();
            match aliases.get(n.id.as_str()) {
                Some(v) => buf.push_str(v),
                None => buf.push_str(n.id.as_str()),
            }
        }
        Expr::Attribute(a) => {
            attr_qual_name_into(&a.value, aliases, buf);
            buf.push('.');
            buf.push_str(a.attr.as_str());
            if let Some(v) = aliases.get(buf.as_str()) {
                buf.clear();
                buf.push_str(v);
            }
        }
        _ => buf.clear(),
    }
}

/// `_get_attr_qual_name` returning a new string.
pub fn attr_qual_name(expr: &Expr, aliases: &Aliases) -> String {
    let mut buf = String::new();
    attr_qual_name_into(expr, aliases, &mut buf);
    buf
}

/// `get_call_name(node, aliases)` written into `buf`.
pub fn call_name_into(call: &ExprCall, aliases: &Aliases, buf: &mut String) {
    match &*call.func {
        Expr::Name(n) => {
            buf.clear();
            match aliases.get(n.id.as_str()) {
                Some(v) => buf.push_str(v),
                None => buf.push_str(n.id.as_str()),
            }
        }
        Expr::Attribute(_) => attr_qual_name_into(&call.func, aliases, buf),
        _ => buf.clear(),
    }
}

/// `get_call_name` returning a new string.
pub fn call_name(call: &ExprCall, aliases: &Aliases) -> String {
    let mut buf = String::new();
    call_name_into(call, aliases, &mut buf);
    buf
}

/// `get_qual_attr(node, aliases)`: `"<prefix>.<attr>"` for attributes (the
/// prefix is the alias-resolved base name, empty when the base is not a
/// plain name), `""` otherwise.
pub fn qual_attr(expr: &Expr, aliases: &Aliases) -> String {
    match expr {
        Expr::Attribute(a) => {
            let prefix = match &*a.value {
                Expr::Name(n) => aliases
                    .get(n.id.as_str())
                    .map(String::as_str)
                    .unwrap_or(n.id.as_str()),
                _ => "",
            };
            format!("{prefix}.{}", a.attr.as_str())
        }
        _ => String::new(),
    }
}

/// `get_called_name(node)`: the attribute or identifier being called.
pub fn called_name(call: &ExprCall) -> &str {
    match &*call.func {
        Expr::Attribute(a) => a.attr.as_str(),
        Expr::Name(n) => n.id.as_str(),
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruff_python_parser::parse_expression;

    fn call_of(src: &str) -> ruff_python_ast::Expr {
        parse_expression(src).unwrap().into_expr()
    }

    fn aliases(pairs: &[(&str, &str)]) -> Aliases {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn get_call_name() {
        let e = call_of("a.b.c.d(x,y)");
        let Expr::Call(c) = &e else { panic!() };
        assert_eq!(call_name(c, &aliases(&[])), "a.b.c.d");
        assert_eq!(
            call_name(c, &aliases(&[("a", "alias.x.y")])),
            "alias.x.y.b.c.d"
        );
        assert_eq!(
            call_name(c, &aliases(&[("a.b", "alias.x.y")])),
            "alias.x.y.c.d"
        );
        assert_eq!(
            call_name(c, &aliases(&[("a.b.c.d", "alias.x.y")])),
            "alias.x.y"
        );
        let e = call_of("a.list[0](x,y)");
        let Expr::Call(c) = &e else { panic!() };
        assert_eq!(attr_qual_name(&c.func, &aliases(&[])), "");
        assert_eq!(call_name(c, &aliases(&[])), "");
        let e = call_of("x[0].bar(1)");
        let Expr::Call(c) = &e else { panic!() };
        assert_eq!(call_name(c, &aliases(&[])), ".bar");
        assert_eq!(called_name(c), "bar");
    }

    #[test]
    fn get_qual_attr() {
        let e = call_of("ssl.PROTOCOL_SSLv2");
        assert_eq!(qual_attr(&e, &aliases(&[])), "ssl.PROTOCOL_SSLv2");
        assert_eq!(
            qual_attr(&e, &aliases(&[("ssl", "OpenSSL.SSL")])),
            "OpenSSL.SSL.PROTOCOL_SSLv2"
        );
        let e = call_of("a.b.c");
        assert_eq!(qual_attr(&e, &aliases(&[])), ".c");
        let e = call_of("name");
        assert_eq!(qual_attr(&e, &aliases(&[])), "");
    }
}
