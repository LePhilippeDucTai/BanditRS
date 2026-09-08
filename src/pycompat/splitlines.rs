//! `bytes.splitlines()` and `str.splitlines()` semantics.

/// Iterator over the lines of a byte string, splitting on `\n`, `\r` and
/// `\r\n` only (Python's `bytes.splitlines()`), without the terminators.
pub struct BytesLines<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for BytesLines<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.pos >= self.data.len() {
            return None;
        }
        let rest = &self.data[self.pos..];
        let mut end = rest.len();
        let mut next = rest.len();
        for (i, &b) in rest.iter().enumerate() {
            if b == b'\n' {
                end = i;
                next = i + 1;
                break;
            }
            if b == b'\r' {
                end = i;
                next = if rest.get(i + 1) == Some(&b'\n') { i + 2 } else { i + 1 };
                break;
            }
        }
        self.pos += next;
        Some(&rest[..end])
    }
}

/// `data.splitlines()` for bytes.
pub fn bytes_splitlines(data: &[u8]) -> BytesLines<'_> {
    BytesLines { data, pos: 0 }
}

/// Whether `c` is a line boundary for `str.splitlines()`.
pub fn is_str_line_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\x0b' | '\x0c' | '\x1c' | '\x1d' | '\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

/// `text.splitlines()` for `str` (no keepends).
pub fn str_splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut iter = text.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        if is_str_line_break(c) {
            lines.push(&text[start..i]);
            let mut next = i + c.len_utf8();
            if c == '\r' && iter.peek().is_some_and(|&(_, n)| n == '\n') {
                iter.next();
                next += 1;
            }
            start = next;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// Python `bytes.strip()` (ASCII whitespace: space, \t, \n, \r, \x0b, \x0c).
pub fn bytes_strip(data: &[u8]) -> &[u8] {
    let is_ws = |b: &u8| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c');
    let start = data.iter().position(|b| !is_ws(b)).unwrap_or(data.len());
    let end = data.iter().rposition(|b| !is_ws(b)).map_or(start, |p| p + 1);
    &data[start..end.max(start)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_lines() {
        let v: Vec<&[u8]> = bytes_splitlines(b"a\nb\r\nc\rd").collect();
        assert_eq!(v, vec![&b"a"[..], b"b", b"c", b"d"]);
        let v: Vec<&[u8]> = bytes_splitlines(b"a\n\n").collect();
        assert_eq!(v, vec![&b"a"[..], b""]);
        let v: Vec<&[u8]> = bytes_splitlines(b"").collect();
        assert!(v.is_empty());
        // \x0c is not a boundary for bytes
        let v: Vec<&[u8]> = bytes_splitlines(b"a\x0cb").collect();
        assert_eq!(v, vec![&b"a\x0cb"[..]]);
    }

    #[test]
    fn str_lines() {
        assert_eq!(str_splitlines("a\nb\r\nc\x0cd\u{2028}e"), vec!["a", "b", "c", "d", "e"]);
        assert_eq!(str_splitlines("x\n"), vec!["x"]);
        assert_eq!(str_splitlines(""), Vec::<&str>::new());
    }

    #[test]
    fn strip() {
        assert_eq!(bytes_strip(b"  # x \r\n"), b"# x");
        assert_eq!(bytes_strip(b" \t "), b"");
    }
}
