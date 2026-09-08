//! Subset of PyYAML's `Emitter` for `safe_dump(default_flow_style=False)`:
//! block style, sorted keys, indentless sequences inside mappings, `[]`/`{}`
//! for empty containers, scalar style selection (plain / single-quoted /
//! double-quoted) using the YAML 1.1 core-schema implicit resolvers, line
//! folding at width 80 for plain and single-quoted scalars (the exact
//! column-tracking algorithm from `emitter.py`'s `write_plain` /
//! `write_single_quoted`). Double-quoted scalars (used only for text with
//! control/non-ASCII characters, since `allow_unicode=False`) are not
//! folded — a deliberate simplification, since bandit's own fields never
//! require it in practice.
//!
//! Used by the yaml formatter and `bandit-config-generator`.

use serde_json::{Map, Value};
use std::sync::OnceLock;

use regex::Regex;

const BEST_WIDTH: usize = 80;

fn non_string_regexes() -> &'static [Regex] {
    static RE: OnceLock<Vec<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        vec![
            Regex::new(r"^(?:yes|Yes|YES|no|No|NO|true|True|TRUE|false|False|FALSE|on|On|ON|off|Off|OFF)$").unwrap(),
            Regex::new(
                r"^(?:[-+]?(?:[0-9][0-9_]*)\.[0-9_]*(?:[eE][-+][0-9]+)?|\.[0-9][0-9_]*(?:[eE][-+][0-9]+)?|[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$",
            )
            .unwrap(),
            Regex::new(r"^(?:[-+]?0b[0-1_]+|[-+]?0[0-7_]+|[-+]?(?:0|[1-9][0-9_]*)|[-+]?0x[0-9a-fA-F_]+|[-+]?[1-9][0-9_]*(?::[0-5]?[0-9])+)$").unwrap(),
            Regex::new(r"^(?:<<)$").unwrap(),
            Regex::new(r"^(?:~|null|Null|NULL|)$").unwrap(),
            Regex::new(
                r"^(?:[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]|[0-9][0-9][0-9][0-9]-[0-9][0-9]?-[0-9][0-9]?(?:[Tt]|[ \t]+)[0-9][0-9]?:[0-9][0-9]:[0-9][0-9](?:\.[0-9]*)?(?:[ \t]*(?:Z|[-+][0-9][0-9]?(?::[0-9][0-9])?))?)$",
            )
            .unwrap(),
            Regex::new(r"^(?:=)$").unwrap(),
        ]
    })
}

/// Whether a plain scalar with this exact text would resolve to a non-`str`
/// tag (bool/int/float/null/timestamp/merge/value), forcing it to be quoted.
fn looks_like_non_string(s: &str) -> bool {
    non_string_regexes().iter().any(|r| r.is_match(s))
}

struct Analysis {
    special: bool,
    leading_ws: bool,
    trailing_ws: bool,
    break_space: bool,
    space_break: bool,
    has_breaks: bool,
    block_indicator: bool,
}

fn analyze(s: &str) -> Analysis {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let leading_ws = matches!(chars.first(), Some(' ') | Some('\n'));
    let trailing_ws = matches!(chars.last(), Some(' ') | Some('\n'));
    let mut special = false;
    let mut break_space = false;
    let mut space_break = false;
    let mut has_breaks = false;
    for i in 0..n {
        let c = chars[i];
        if c == '\n' {
            has_breaks = true;
            if i > 0 && chars[i - 1] == ' ' {
                space_break = true;
            }
        } else if !('\u{20}'..='\u{7e}').contains(&c) {
            special = true;
        }
        if c == ' ' && i > 0 && chars[i - 1] == '\n' {
            break_space = true;
        }
    }

    let followed_by_ws = |i: usize| i + 1 >= n || matches!(chars[i + 1], ' ' | '\t' | '\r' | '\n');
    let mut block_indicator = false;
    if s.starts_with("---") || s.starts_with("...") {
        block_indicator = true;
    }
    if n > 0 {
        let c0 = chars[0];
        if "#,[]{}&*!|>'\"%@`".contains(c0) {
            block_indicator = true;
        }
        if (c0 == '?' || c0 == ':') && followed_by_ws(0) {
            block_indicator = true;
        }
        if c0 == '-' && followed_by_ws(0) {
            block_indicator = true;
        }
    }
    let mut preceded_by_ws = true;
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 {
            if c == ':' && followed_by_ws(i) {
                block_indicator = true;
            }
            if c == '#' && preceded_by_ws {
                block_indicator = true;
            }
        }
        preceded_by_ws = matches!(c, ' ' | '\t' | '\r' | '\n');
    }

    Analysis {
        special,
        leading_ws,
        trailing_ws,
        break_space,
        space_break,
        has_breaks,
        block_indicator,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Plain,
    Single,
    Double,
}

fn choose_style(s: &str, a: &Analysis) -> Style {
    let allow_block_plain = !a.leading_ws
        && !a.trailing_ws
        && !a.break_space
        && !a.space_break
        && !a.special
        && !a.has_breaks
        && !a.block_indicator;
    let allow_single = !a.break_space && !a.space_break && !a.special;
    if allow_block_plain && !looks_like_non_string(s) {
        Style::Plain
    } else if allow_single {
        Style::Single
    } else {
        Style::Double
    }
}

struct Writer {
    out: String,
    column: usize,
}

impl Writer {
    fn new() -> Writer {
        Writer {
            out: String::new(),
            column: 0,
        }
    }

    fn push(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
        self.out.push_str(s);
    }

    fn newline_and_indent(&mut self, indent: usize) {
        if !self.out.is_empty() {
            self.push("\n");
        }
        self.push(&" ".repeat(indent));
    }

    fn write_indent(&mut self, indent: usize) {
        self.push("\n");
        self.push(&" ".repeat(indent));
    }
}

/// `write_plain(text)`: single-space-run folding at `best_width`.
/// `foldable` mirrors Python's `split` (`False` for a mapping key —
/// `simple_key_context` — which never folds regardless of width).
fn write_plain(w: &mut Writer, text: &str, indent: usize, foldable: bool) {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut spaces = false;
    let mut start = 0usize;
    let mut end = 0usize;
    while end <= n {
        let ch = if end < n { Some(chars[end]) } else { None };
        if spaces {
            if ch != Some(' ') {
                if foldable && start + 1 == end && w.column > BEST_WIDTH {
                    w.write_indent(indent);
                } else {
                    w.push(&chars[start..end].iter().collect::<String>());
                }
                start = end;
            }
        } else if ch.is_none() || ch == Some(' ') {
            w.push(&chars[start..end].iter().collect::<String>());
            start = end;
        }
        spaces = ch == Some(' ');
        end += 1;
    }
}

/// `write_single_quoted(text)`.
fn write_single_quoted(w: &mut Writer, text: &str, indent: usize, foldable: bool) {
    w.push("'");
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut spaces = false;
    let mut breaks = false;
    let mut start = 0usize;
    let mut end = 0usize;
    while end <= n {
        let ch = if end < n { Some(chars[end]) } else { None };
        if spaces {
            if ch != Some(' ') {
                if foldable && start + 1 == end && w.column > BEST_WIDTH && start != 0 && end != n {
                    w.write_indent(indent);
                } else {
                    w.push(&chars[start..end].iter().collect::<String>());
                }
                start = end;
            }
        } else if breaks {
            if ch != Some('\n') {
                if chars[start] == '\n' {
                    w.push("\n");
                }
                for _ in start..end {
                    w.push("\n");
                }
                w.push(&" ".repeat(indent));
                start = end;
            }
        } else if ch.is_none() || ch == Some(' ') || ch == Some('\n') || ch == Some('\'') {
            if start < end {
                w.push(&chars[start..end].iter().collect::<String>());
            }
            start = end;
        }
        if ch == Some('\'') {
            w.push("''");
            start = end + 1;
        }
        if let Some(c) = ch {
            spaces = c == ' ';
            breaks = c == '\n';
        }
        end += 1;
    }
    w.push("'");
}

const DQ_ESCAPES: &[(char, char)] = &[
    ('\0', '0'),
    ('\u{07}', 'a'),
    ('\u{08}', 'b'),
    ('\t', 't'),
    ('\n', 'n'),
    ('\u{0b}', 'v'),
    ('\u{0c}', 'f'),
    ('\r', 'r'),
    ('\u{1b}', 'e'),
    ('"', '"'),
    ('\\', '\\'),
];

/// `write_double_quoted(text)`, without width-based folding (see module docs).
fn write_double_quoted(w: &mut Writer, text: &str) {
    w.push("\"");
    for ch in text.chars() {
        let printable = ('\u{20}'..='\u{7e}').contains(&ch);
        if ch == '"' || ch == '\\' || !printable {
            if let Some((_, esc)) = DQ_ESCAPES.iter().find(|(c, _)| *c == ch) {
                w.push(&format!("\\{esc}"));
            } else if (ch as u32) <= 0xff {
                w.push(&format!("\\x{:02X}", ch as u32));
            } else if (ch as u32) <= 0xffff {
                w.push(&format!("\\u{:04X}", ch as u32));
            } else {
                w.push(&format!("\\U{:08X}", ch as u32));
            }
        } else {
            w.push(&ch.to_string());
        }
    }
    w.push("\"");
}

/// Emit a string scalar. `fold_indent` is the continuation indent used when
/// folding (ignored, no folding, when `foldable` is false — mapping keys).
fn emit_string(w: &mut Writer, s: &str, fold_indent: usize, foldable: bool) {
    if s.is_empty() {
        w.push("''");
        return;
    }
    let a = analyze(s);
    match choose_style(s, &a) {
        Style::Plain => write_plain(w, s, fold_indent, foldable),
        Style::Single => write_single_quoted(w, s, fold_indent, foldable),
        Style::Double => write_double_quoted(w, s),
    }
}

fn emit_key(w: &mut Writer, s: &str) {
    emit_string(w, s, 0, false);
}

fn emit_scalar_value(w: &mut Writer, value: &Value, fold_indent: usize) {
    match value {
        Value::Null => w.push("null"),
        Value::Bool(b) => w.push(if *b { "true" } else { "false" }),
        Value::Number(n) => w.push(&n.to_string()),
        Value::String(s) => emit_string(w, s, fold_indent, true),
        Value::Array(_) | Value::Object(_) => {
            unreachable!("emit_scalar_value called on a container")
        }
    }
}

fn is_scalar(value: &Value) -> bool {
    !matches!(value, Value::Array(_) | Value::Object(_))
}

fn sorted_entries(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut v: Vec<(&String, &Value)> = map.iter().collect();
    v.sort_by(|a, b| a.0.cmp(b.0));
    v
}

fn emit_mapping_entries(
    w: &mut Writer,
    map: &Map<String, Value>,
    indent: usize,
    first_inline: bool,
) {
    for (i, (k, v)) in sorted_entries(map).into_iter().enumerate() {
        if i > 0 || !first_inline {
            w.newline_and_indent(indent);
        }
        emit_key(w, k);
        w.push(":");
        emit_mapping_value(w, v, indent);
    }
}

fn emit_mapping_value(w: &mut Writer, v: &Value, key_indent: usize) {
    match v {
        Value::Object(m) if !m.is_empty() => {
            w.write_indent(key_indent + 2);
            emit_mapping_entries(w, m, key_indent + 2, true);
        }
        Value::Object(_) => w.push(" {}"),
        Value::Array(items) if !items.is_empty() => {
            w.write_indent(key_indent);
            emit_sequence_entries(w, items, key_indent, true);
        }
        Value::Array(_) => w.push(" []"),
        scalar => {
            w.push(" ");
            emit_scalar_value(w, scalar, key_indent + 2);
        }
    }
}

fn emit_sequence_entries(w: &mut Writer, items: &[Value], indent: usize, first_inline: bool) {
    for (i, item) in items.iter().enumerate() {
        if i > 0 || !first_inline {
            w.newline_and_indent(indent);
        }
        w.push("- ");
        emit_sequence_item(w, item, indent + 2);
    }
}

fn emit_sequence_item(w: &mut Writer, item: &Value, item_indent: usize) {
    match item {
        Value::Object(m) if !m.is_empty() => emit_mapping_entries(w, m, item_indent, true),
        Value::Object(_) => w.push("{}"),
        Value::Array(items) if !items.is_empty() => {
            emit_sequence_entries(w, items, item_indent, true)
        }
        Value::Array(_) => w.push("[]"),
        scalar => {
            debug_assert!(is_scalar(scalar));
            emit_scalar_value(w, scalar, item_indent);
        }
    }
}

/// `yaml.safe_dump(value, default_flow_style=False)`. `value` must be a JSON
/// object, array, or scalar built from `Issue::as_dict`/`Metrics::to_json`
/// data (strings, non-negative integers, bools, null).
pub fn safe_dump_block(value: &Value) -> String {
    let mut w = Writer::new();
    match value {
        Value::Object(m) if !m.is_empty() => emit_mapping_entries(&mut w, m, 0, true),
        Value::Object(_) => w.push("{}"),
        Value::Array(items) if !items.is_empty() => emit_sequence_entries(&mut w, items, 0, true),
        Value::Array(_) => w.push("[]"),
        scalar => emit_scalar_value(&mut w, scalar, 0),
    }
    w.push("\n");
    w.out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn matches_pyyaml_reference_cases() {
        assert_eq!(safe_dump_block(&json!({"a": "hi"})), "a: hi\n");
        assert_eq!(safe_dump_block(&json!({"a": ""})), "a: ''\n");
        assert_eq!(safe_dump_block(&json!({"a": "a: b"})), "a: 'a: b'\n");
        assert_eq!(safe_dump_block(&json!({"a": "123"})), "a: '123'\n");
        assert_eq!(safe_dump_block(&json!({"a": "true"})), "a: 'true'\n");
        assert_eq!(safe_dump_block(&json!({"a": "null"})), "a: 'null'\n");
        assert_eq!(
            safe_dump_block(&json!({"a": "it's a test"})),
            "a: it's a test\n"
        );
        assert_eq!(
            safe_dump_block(&json!({"a": "has \"quotes\""})),
            "a: has \"quotes\"\n"
        );
        assert_eq!(safe_dump_block(&json!({"a": "abc "})), "a: 'abc '\n");
        assert_eq!(safe_dump_block(&json!({"a": "- item"})), "a: '- item'\n");
        assert_eq!(
            safe_dump_block(&json!({"a": "value # not a comment"})),
            "a: 'value # not a comment'\n"
        );
        assert_eq!(safe_dump_block(&json!({"a": null})), "a: null\n");
        assert_eq!(safe_dump_block(&json!({"a": true})), "a: true\n");
        assert_eq!(safe_dump_block(&json!({"a": 5})), "a: 5\n");
        assert_eq!(safe_dump_block(&json!({"a": []})), "a: []\n");
        assert_eq!(safe_dump_block(&json!({"a": {}})), "a: {}\n");
        assert_eq!(
            safe_dump_block(&json!({"a": [1, 2, 3]})),
            "a:\n- 1\n- 2\n- 3\n"
        );
        assert_eq!(safe_dump_block(&json!({"a": "café"})), "a: \"caf\\xE9\"\n");
        assert_eq!(
            safe_dump_block(&json!({"a": "tab\ttab"})),
            "a: \"tab\\ttab\"\n"
        );
    }

    #[test]
    fn folds_long_plain_scalars_at_width_80() {
        let s = "Use of unsafe yaml load. Allows instantiation of arbitrary objects. Consider yaml.safe_load().";
        assert_eq!(safe_dump_block(&json!({"a": s})), format!("a: {s}\n"));
        assert_eq!(
            safe_dump_block(&json!({"issue_text": s})),
            "issue_text: Use of unsafe yaml load. Allows instantiation of arbitrary objects. Consider\n  yaml.safe_load().\n"
        );
    }

    #[test]
    fn embedded_newline_uses_single_quoted_double_break() {
        let s = "Use of unsafe yaml load. Allows instantiation of arbitrary objects.\nConsider yaml.safe_load().";
        assert_eq!(
            safe_dump_block(&json!({"a": s})),
            "a: 'Use of unsafe yaml load. Allows instantiation of arbitrary objects.\n\n  Consider yaml.safe_load().'\n"
        );
    }

    #[test]
    fn nested_list_of_maps_indentless() {
        let v = json!({"results": [{"issue_text": "Use of unsafe yaml load. Allows instantiation of arbitrary objects. Consider yaml.safe_load()."}]});
        assert_eq!(
            safe_dump_block(&v),
            "results:\n- issue_text: Use of unsafe yaml load. Allows instantiation of arbitrary objects.\n    Consider yaml.safe_load().\n"
        );
    }
}
