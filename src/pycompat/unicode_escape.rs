//! Python's `unicode_escape` codec.

/// `text.encode("unicode_escape")`.
pub fn encode(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            ' '..='~' => out.push(c as u8),
            c => {
                let u = c as u32;
                if u < 0x100 {
                    out.extend_from_slice(format!("\\x{u:02x}").as_bytes());
                } else if u < 0x10000 {
                    out.extend_from_slice(format!("\\u{u:04x}").as_bytes());
                } else {
                    out.extend_from_slice(format!("\\U{u:08x}").as_bytes());
                }
            }
        }
    }
    out
}

/// `data.decode("unicode_escape")`. Unknown escapes are kept verbatim (as
/// Python does, with a warning); invalid `\x`/`\u` escapes are an error.
pub fn decode(data: &[u8]) -> Result<String, String> {
    let mut out = String::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        if b != b'\\' {
            out.push(b as char);
            i += 1;
            continue;
        }
        i += 1;
        let Some(&e) = data.get(i) else {
            return Err("\\ at end of string".to_string());
        };
        i += 1;
        let hex = |n: usize, i: &mut usize, kind: &str| -> Result<u32, String> {
            let slice = data
                .get(*i..*i + n)
                .ok_or_else(|| format!("truncated {kind} escape"))?;
            let s = std::str::from_utf8(slice).map_err(|_| format!("truncated {kind} escape"))?;
            let v = u32::from_str_radix(s, 16).map_err(|_| format!("truncated {kind} escape"))?;
            *i += n;
            Ok(v)
        };
        match e {
            b'\n' => {}
            b'\\' => out.push('\\'),
            b'\'' => out.push('\''),
            b'"' => out.push('"'),
            b'a' => out.push('\x07'),
            b'b' => out.push('\x08'),
            b'f' => out.push('\x0c'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'v' => out.push('\x0b'),
            b'0'..=b'7' => {
                let mut v = (e - b'0') as u32;
                let mut n = 1;
                while n < 3 && i < data.len() && (b'0'..=b'7').contains(&data[i]) {
                    v = v * 8 + (data[i] - b'0') as u32;
                    i += 1;
                    n += 1;
                }
                out.push(char::from_u32(v).unwrap_or('\u{fffd}'));
            }
            b'x' => {
                let v = hex(2, &mut i, "\\xXX")?;
                out.push(char::from_u32(v).unwrap_or('\u{fffd}'));
            }
            b'u' => {
                let v = hex(4, &mut i, "\\uXXXX")?;
                out.push(char::from_u32(v).unwrap_or('\u{fffd}'));
            }
            b'U' => {
                let v = hex(8, &mut i, "\\UXXXXXXXX")?;
                out.push(char::from_u32(v).ok_or_else(|| "illegal Unicode character".to_string())?);
            }
            other => {
                out.push('\\');
                out.push(other as char);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        assert_eq!(encode("ascii"), b"ascii");
        assert_eq!(encode("\u{0}"), b"\\x00");
        assert_eq!(encode("é\n"), b"\\xe9\\n");
        assert_eq!(encode("\u{202e}"), b"\\u202e");
        assert_eq!(decode(b"\\u0000").unwrap(), "\u{0}");
        assert_eq!(decode(b"\\uffff").unwrap(), "\u{ffff}");
        assert_eq!(decode(b"ascii\\x41\\n").unwrap(), "asciiA\n");
        assert!(decode(b"\\x4").is_err());
    }
}
