//! `json.dumps(obj, sort_keys=True, indent=2, separators=(",", ": "))`: recursive key
//! sort, 2-space indentation, `[]`/`{}` for empty containers, `ensure_ascii` (every
//! char >= 0x7f escaped as `\uXXXX`, surrogate pairs above the BMP). Build on
//! `serde_json::Value` + a custom writer (serde_json's own pretty-printer neither
//! sorts keys nor escapes non-ASCII).

use serde_json::Value;

/// `json.dumps(value, sort_keys=True, indent=2, separators=(",", ": "))`.
/// No trailing newline (matches `json.dumps`, unlike `serde_json::to_string_pretty`).
pub fn dumps_sorted_indent2(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0);
    out
}

/// Same rendering (2-space indent, `ensure_ascii`, no trailing newline) but
/// preserving object insertion order instead of sorting keys — used by the
/// SARIF formatter (`jschema_to_python.to_json`, whose field order follows
/// each `sarif_om` class's attribute declaration, not alphabetical order).
pub fn dumps_indent2_preserve_order(value: &Value) -> String {
    let mut out = String::new();
    write_value_ordered(&mut out, value, 0);
    out
}

fn write_value_ordered(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Object(map) => {
            let entries: Vec<(&String, &Value)> = map.iter().collect();
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            let inner = indent + 2;
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&" ".repeat(inner));
                write_escaped_string(out, k);
                out.push_str(": ");
                write_value_ordered(out, v, inner);
            }
            out.push('\n');
            out.push_str(&" ".repeat(indent));
            out.push('}');
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            let inner = indent + 2;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&" ".repeat(inner));
                write_value_ordered(out, item, inner);
            }
            out.push('\n');
            out.push_str(&" ".repeat(indent));
            out.push(']');
        }
        _ => write_value(out, value, indent),
    }
}

fn write_value(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_escaped_string(out, s),
        Value::Array(items) => write_array(out, items, indent),
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            write_object(out, &entries, indent);
        }
    }
}

fn write_array(out: &mut String, items: &[Value], indent: usize) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    out.push('[');
    let inner = indent + 2;
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('\n');
        out.push_str(&" ".repeat(inner));
        write_value(out, item, inner);
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push(']');
}

fn write_object(out: &mut String, entries: &[(&String, &Value)], indent: usize) {
    if entries.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push('{');
    let inner = indent + 2;
    for (i, (k, v)) in entries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('\n');
        out.push_str(&" ".repeat(inner));
        write_escaped_string(out, k);
        out.push_str(": ");
        write_value(out, v, inner);
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push('}');
}

/// `ensure_ascii=True` string escaping: `"` `\` and control chars via their
/// short escapes (`\n \r \t \b \f`) or `\u00XX`; everything >= 0x7f as
/// `\uXXXX` (surrogate pairs for code points beyond the BMP).
fn write_escaped_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if (c as u32) < 0x7f => out.push(c),
            c => {
                let cp = c as u32;
                if cp <= 0xffff {
                    out.push_str(&format!("\\u{cp:04x}"));
                } else {
                    let v = cp - 0x10000;
                    let high = 0xd800 + (v >> 10);
                    let low = 0xdc00 + (v & 0x3ff);
                    out.push_str(&format!("\\u{high:04x}\\u{low:04x}"));
                }
            }
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_keys_and_indents() {
        let v = json!({"b": 1, "a": [1, 2], "c": {}, "d": []});
        assert_eq!(
            dumps_sorted_indent2(&v),
            "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": 1,\n  \"c\": {},\n  \"d\": []\n}"
        );
    }

    #[test]
    fn escapes_non_ascii_and_control_chars() {
        let v = json!({"s": "café\n\t\"\\\u{1f600}"});
        let out = dumps_sorted_indent2(&v);
        assert!(out.contains("caf\\u00e9\\n\\t\\\"\\\\\\ud83d\\ude00"));
    }

    #[test]
    fn top_level_empty_containers() {
        assert_eq!(dumps_sorted_indent2(&json!([])), "[]");
        assert_eq!(dumps_sorted_indent2(&json!({})), "{}");
    }
}
