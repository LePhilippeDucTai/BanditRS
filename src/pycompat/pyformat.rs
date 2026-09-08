//! Python `string.Formatter().parse()` (template tokenizing) plus the
//! `str`/`int` subset of the format-spec mini-language (`{line:03}`,
//! `{test_id:^8}`, `{relpath:20.20s}`, `{msg:>20}`). Used by the `custom`
//! formatter (`--msg-template`).

/// One token of a parsed template.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Literal(String),
    Field { name: String, spec: String, conv: Option<char> },
}

/// `string.Formatter().parse(template)`, tokenizing `{{`/`}}` escapes and
/// `{name[!conv][:spec]}` fields. Errors mirror CPython's `ValueError`
/// messages.
pub fn parse_template(template: &str) -> Result<Vec<Segment>, String> {
    let chars: Vec<char> = template.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut out = Vec::new();
    let mut literal = String::new();
    while i < n {
        match chars[i] {
            '{' => {
                if i + 1 < n && chars[i + 1] == '{' {
                    literal.push('{');
                    i += 2;
                    continue;
                }
                if !literal.is_empty() {
                    out.push(Segment::Literal(std::mem::take(&mut literal)));
                }
                i += 1;
                let start = i;
                let mut depth = 0i32;
                while i < n {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    }
                    i += 1;
                }
                if i >= n {
                    return Err("expected '}' before end of string".to_string());
                }
                let field_text: String = chars[start..i].iter().collect();
                i += 1;
                let (name_conv, spec) = match field_text.find(':') {
                    Some(pos) => (&field_text[..pos], field_text[pos + 1..].to_string()),
                    None => (field_text.as_str(), String::new()),
                };
                let (name, conv) = match name_conv.rfind('!') {
                    Some(pos) if pos == name_conv.len() - 2 => {
                        (&name_conv[..pos], name_conv[pos + 1..].chars().next())
                    }
                    _ => (name_conv, None),
                };
                out.push(Segment::Field { name: name.to_string(), spec, conv });
            }
            '}' => {
                if i + 1 < n && chars[i + 1] == '}' {
                    literal.push('}');
                    i += 2;
                    continue;
                }
                return Err("Single '}' encountered in format string".to_string());
            }
            c => {
                literal.push(c);
                i += 1;
            }
        }
    }
    if !literal.is_empty() {
        out.push(Segment::Literal(literal));
    }
    Ok(out)
}

/// A parsed format spec (`[[fill]align][sign][#][0][width][,_][.precision][type]`).
#[derive(Debug, Clone, Copy)]
pub struct FormatSpec {
    pub fill: char,
    pub align: Option<char>,
    pub sign: char,
    pub zero: bool,
    pub width: Option<usize>,
    pub grouping: Option<char>,
    pub precision: Option<usize>,
    pub ty: Option<char>,
}

/// Parse a format spec string.
pub fn parse_spec(spec: &str) -> Result<FormatSpec, String> {
    let chars: Vec<char> = spec.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut fill = ' ';
    let mut align = None;
    if n >= 2 && "<>=^".contains(chars[1]) {
        fill = chars[0];
        align = Some(chars[1]);
        i = 2;
    } else if n >= 1 && "<>=^".contains(chars[0]) {
        align = Some(chars[0]);
        i = 1;
    }
    let mut sign = '-';
    if i < n && "+- ".contains(chars[i]) {
        sign = chars[i];
        i += 1;
    }
    if i < n && chars[i] == '#' {
        i += 1;
    }
    let mut zero = false;
    if i < n && chars[i] == '0' {
        zero = true;
        if align.is_none() {
            align = Some('=');
            fill = '0';
        }
        i += 1;
    }
    let start = i;
    while i < n && chars[i].is_ascii_digit() {
        i += 1;
    }
    let width = if i > start { Some(chars[start..i].iter().collect::<String>().parse().unwrap()) } else { None };
    let mut grouping = None;
    if i < n && (chars[i] == ',' || chars[i] == '_') {
        grouping = Some(chars[i]);
        i += 1;
    }
    let mut precision = None;
    if i < n && chars[i] == '.' {
        i += 1;
        let start = i;
        while i < n && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return Err("Format specifier missing precision".to_string());
        }
        precision = Some(chars[start..i].iter().collect::<String>().parse().unwrap());
    }
    let ty = if i < n {
        let t = chars[i];
        i += 1;
        Some(t)
    } else {
        None
    };
    if i != n {
        return Err(format!("Invalid format specifier '{spec}'"));
    }
    Ok(FormatSpec { fill, align, sign, zero, width, grouping, precision, ty })
}

fn pad(s: &str, width: usize, align: char, fill: char) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let padding = width - len;
    match align {
        '<' => format!("{s}{}", fill.to_string().repeat(padding)),
        '>' => format!("{}{s}", fill.to_string().repeat(padding)),
        '^' => {
            let left = padding / 2;
            let right = padding - left;
            format!("{}{s}{}", fill.to_string().repeat(left), fill.to_string().repeat(right))
        }
        _ => format!("{}{s}", fill.to_string().repeat(padding)),
    }
}

/// Apply a format spec to a string value (`str.__format__`).
pub fn format_str(value: &str, spec: &FormatSpec) -> Result<String, String> {
    if let Some(ty) = spec.ty {
        if ty != 's' {
            return Err(format!("Unknown format code '{ty}' for object of type 'str'"));
        }
    }
    if spec.align == Some('=') {
        return Err("'=' alignment not allowed in string format specifier".to_string());
    }
    let mut s = value.to_string();
    if let Some(p) = spec.precision {
        s = s.chars().take(p).collect();
    }
    let width = spec.width.unwrap_or(0);
    Ok(pad(&s, width, spec.align.unwrap_or('<'), spec.fill))
}

/// Apply a format spec to an integer value (`int.__format__`).
pub fn format_int(value: i64, spec: &FormatSpec) -> Result<String, String> {
    if let Some(ty) = spec.ty {
        if ty != 'd' {
            return Err(format!("Unknown format code '{ty}' for object of type 'int'"));
        }
    }
    let neg = value < 0;
    let mut digits = value.unsigned_abs().to_string();
    if let Some(g) = spec.grouping {
        digits = group_digits(&digits, g);
    }
    let sign_str = if neg {
        "-"
    } else {
        match spec.sign {
            '+' => "+",
            ' ' => " ",
            _ => "",
        }
    };
    let width = spec.width.unwrap_or(0);
    let body_len = sign_str.chars().count() + digits.chars().count();
    if body_len >= width {
        return Ok(format!("{sign_str}{digits}"));
    }
    let padding = width - body_len;
    let fill = spec.fill;
    Ok(match spec.align.unwrap_or('>') {
        '=' => format!("{sign_str}{}{digits}", fill.to_string().repeat(padding)),
        '<' => format!("{sign_str}{digits}{}", fill.to_string().repeat(padding)),
        '^' => {
            let left = padding / 2;
            let right = padding - left;
            format!("{}{sign_str}{digits}{}", fill.to_string().repeat(left), fill.to_string().repeat(right))
        }
        _ => format!("{}{sign_str}{digits}", fill.to_string().repeat(padding)),
    })
}

fn group_digits(digits: &str, sep: char) -> String {
    let bytes: Vec<char> = digits.chars().collect();
    let mut out = Vec::new();
    for (i, c) in bytes.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(sep);
        }
        out.push(*c);
    }
    out.reverse();
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_literal_and_fields() {
        let segs = parse_template("{abspath}:{line}: {test_id}[bandit]: {severity}: {msg}").unwrap();
        assert_eq!(segs.len(), 9);
        assert_eq!(segs[0], Segment::Field { name: "abspath".into(), spec: "".into(), conv: None });
        assert_eq!(segs[1], Segment::Literal(":".into()));
    }

    #[test]
    fn handles_doubled_braces_and_conv_spec() {
        let segs = parse_template("{{lit}} {a!r:>5}").unwrap();
        assert_eq!(segs[0], Segment::Literal("{lit} ".into()));
        assert_eq!(segs[1], Segment::Field { name: "a".into(), spec: ">5".into(), conv: Some('r') });
    }

    #[test]
    fn unmatched_braces_error() {
        assert_eq!(parse_template("{abc").unwrap_err(), "expected '}' before end of string");
        assert_eq!(parse_template("abc}").unwrap_err(), "Single '}' encountered in format string");
    }

    #[test]
    fn formats_examples_from_docs() {
        let spec = parse_spec("03").unwrap();
        assert_eq!(format_int(5, &spec).unwrap(), "005");
        let spec = parse_spec("^8").unwrap();
        assert_eq!(format_str("ab", &spec).unwrap(), "   ab   ");
        let spec = parse_spec("20.20s").unwrap();
        assert_eq!(format_str("hi", &spec).unwrap(), format!("hi{}", " ".repeat(18)));
        let spec = parse_spec(">20").unwrap();
        assert_eq!(format_str("hi", &spec).unwrap(), format!("{}hi", " ".repeat(18)));
    }
}
