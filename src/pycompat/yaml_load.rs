//! PyYAML `safe_load` semantics on top of `saphyr_parser` events: plain
//! scalars resolved with the YAML 1.1 implicit resolvers (`yes/no/on/off/
//! true/false` → bool, `0o17`/`0x1f`/`0b1`/`1_000` → int, floats, `~`/
//! `null`/empty → null; everything else str), quoted scalars are always
//! strings; mappings keep insertion order (`ConfigValue::Map`). Errors →
//! `ConfigError("Error parsing file.")` at the caller.

use std::sync::OnceLock;

use regex::Regex;
use saphyr_parser::{Event, Parser, ScalarStyle};

use crate::core::config::ConfigValue;
use indexmap::IndexMap;

fn bool_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(?:yes|Yes|YES|no|No|NO|true|True|TRUE|false|False|FALSE|on|On|ON|off|Off|OFF)$").unwrap())
}

fn null_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(?:~|null|Null|NULL|)$").unwrap())
}

fn int_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(?:[-+]?0b[0-1_]+|[-+]?0[0-7_]+|[-+]?(?:0|[1-9][0-9_]*)|[-+]?0x[0-9a-fA-F_]+)$").unwrap())
}

fn float_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(?:[-+]?(?:[0-9][0-9_]*)\.[0-9_]*(?:[eE][-+][0-9]+)?|\.[0-9][0-9_]*(?:[eE][-+][0-9]+)?|[-+]?\.(?:inf|Inf|INF)|\.(?:nan|NaN|NAN))$").unwrap()
    })
}

fn parse_int(s: &str) -> Option<i64> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let s = s.replace('_', "");
    let v = if let Some(rest) = s.strip_prefix("0x") {
        i64::from_str_radix(rest, 16).ok()?
    } else if let Some(rest) = s.strip_prefix("0b") {
        i64::from_str_radix(rest, 2).ok()?
    } else if s.starts_with('0') && s.len() > 1 && s.bytes().all(|b| b.is_ascii_digit()) {
        i64::from_str_radix(&s, 8).ok()?
    } else {
        s.parse().ok()?
    };
    Some(if neg { -v } else { v })
}

fn parse_float(s: &str) -> Option<f64> {
    let s2 = s.replace('_', "");
    if s2.ends_with("inf") || s2.ends_with("Inf") || s2.ends_with("INF") {
        return Some(if s2.starts_with('-') { f64::NEG_INFINITY } else { f64::INFINITY });
    }
    if s2.ends_with("nan") || s2.ends_with("NaN") || s2.ends_with("NAN") {
        return Some(f64::NAN);
    }
    s2.parse().ok()
}

fn resolve_plain_scalar(s: &str) -> ConfigValue {
    if s.is_empty() || null_re().is_match(s) {
        return ConfigValue::Null;
    }
    if bool_re().is_match(s) {
        return ConfigValue::Bool(matches!(s.to_ascii_lowercase().as_str(), "yes" | "true" | "on"));
    }
    if int_re().is_match(s) {
        if let Some(v) = parse_int(s) {
            return ConfigValue::Int(v);
        }
    }
    if float_re().is_match(s) {
        if let Some(v) = parse_float(s) {
            return ConfigValue::Float(v);
        }
    }
    ConfigValue::Str(s.to_string())
}

/// `yaml.safe_load(text)`.
pub fn safe_load(text: &str) -> Result<ConfigValue, String> {
    let mut parser = Parser::new_from_str(text);
    let mut events = Vec::new();
    loop {
        match parser.next() {
            Some(Ok((event, _span))) => {
                let is_end = matches!(event, Event::StreamEnd);
                events.push(event);
                if is_end {
                    break;
                }
            }
            Some(Err(e)) => return Err(e.to_string()),
            None => break,
        }
    }
    let mut iter = events.into_iter().peekable();
    // Skip StreamStart / DocumentStart.
    while matches!(iter.peek(), Some(Event::StreamStart) | Some(Event::DocumentStart(_))) {
        iter.next();
    }
    if matches!(iter.peek(), Some(Event::StreamEnd) | Some(Event::DocumentEnd) | None) {
        return Ok(ConfigValue::Null);
    }
    build_node(&mut iter)
}

fn build_node(iter: &mut std::iter::Peekable<std::vec::IntoIter<Event<'_>>>) -> Result<ConfigValue, String> {
    match iter.next() {
        Some(Event::Scalar(value, style, _, _)) => Ok(match style {
            ScalarStyle::Plain => resolve_plain_scalar(&value),
            _ => ConfigValue::Str(value.into_owned()),
        }),
        Some(Event::SequenceStart(..)) => {
            let mut items = Vec::new();
            while !matches!(iter.peek(), Some(Event::SequenceEnd)) {
                items.push(build_node(iter)?);
            }
            iter.next();
            Ok(ConfigValue::List(items))
        }
        Some(Event::MappingStart(..)) => {
            let mut map = IndexMap::new();
            while !matches!(iter.peek(), Some(Event::MappingEnd)) {
                let key = build_node(iter)?;
                let key = match key {
                    ConfigValue::Str(s) => s,
                    other => other.py_str(),
                };
                let value = build_node(iter)?;
                map.insert(key, value);
            }
            iter.next();
            Ok(ConfigValue::Map(map))
        }
        Some(Event::Alias(_)) => Err("aliases are not supported".to_string()),
        Some(Event::DocumentStart(_)) => build_node(iter),
        other => Err(format!("unexpected YAML event: {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalars_and_types() {
        assert_eq!(safe_load("~").unwrap(), ConfigValue::Null);
        assert_eq!(safe_load("true").unwrap(), ConfigValue::Bool(true));
        assert_eq!(safe_load("no").unwrap(), ConfigValue::Bool(false));
        assert_eq!(safe_load("5").unwrap(), ConfigValue::Int(5));
        assert_eq!(safe_load("-5").unwrap(), ConfigValue::Int(-5));
        assert_eq!(safe_load("0x1f").unwrap(), ConfigValue::Int(31));
        assert_eq!(safe_load("1_000").unwrap(), ConfigValue::Int(1000));
        assert_eq!(safe_load("3.5").unwrap(), ConfigValue::Float(3.5));
        assert_eq!(safe_load("'123'").unwrap(), ConfigValue::Str("123".into()));
        assert_eq!(safe_load("hello").unwrap(), ConfigValue::Str("hello".into()));
    }

    #[test]
    fn parses_mapping_and_sequence() {
        let v = safe_load("a: 1\nb:\n  - 1\n  - 2\nc:\n  d: true\n").unwrap();
        let m = v.as_map().unwrap();
        assert_eq!(m.get("a"), Some(&ConfigValue::Int(1)));
        assert_eq!(m.get("b").unwrap().as_list().unwrap().len(), 2);
        assert_eq!(m.get("c").unwrap().as_map().unwrap().get("d"), Some(&ConfigValue::Bool(true)));
    }

    #[test]
    fn invalid_yaml_errors() {
        assert!(safe_load("a: [").is_err());
    }
}
