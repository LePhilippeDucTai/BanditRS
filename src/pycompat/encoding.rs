//! Source encoding detection (PEP 263) and decoding, mirroring
//! `tokenize.detect_encoding` and the CPython tokenizer.

use std::fmt;
use std::sync::LazyLock;

use regex::Regex;

/// Failure to decode a source file; CPython raises `SyntaxError` for all of
/// these, which bandit reports as a syntax error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The first lines are not valid UTF-8 (`invalid or missing encoding declaration`).
    InvalidDeclaration,
    /// `unknown encoding: <name>`.
    UnknownEncoding(String),
    /// A UTF-8 BOM combined with a non UTF-8 cookie (`encoding problem: utf-8`).
    EncodingProblem,
    /// The body could not be decoded with the declared codec.
    Undecodable(String),
    /// Source contains NUL bytes (`source code string cannot contain null bytes`).
    NullBytes,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::InvalidDeclaration => {
                f.write_str("invalid or missing encoding declaration")
            }
            DecodeError::UnknownEncoding(e) => write!(f, "unknown encoding: {e}"),
            DecodeError::EncodingProblem => f.write_str("encoding problem: utf-8"),
            DecodeError::Undecodable(e) => write!(f, "'{e}' codec can't decode source"),
            DecodeError::NullBytes => f.write_str("source code string cannot contain null bytes"),
        }
    }
}

impl std::error::Error for DecodeError {}

static COOKIE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ \t\x0c]*#.*?coding[:=][ \t]*([-A-Za-z0-9_.]+)").unwrap());

const BOM_UTF8: &[u8] = b"\xef\xbb\xbf";

/// Detected encoding of a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    /// Normalised codec name (`utf-8`, `iso-8859-1`, or the declared name).
    pub encoding: String,
    /// Whether a UTF-8 BOM was present.
    pub bom: bool,
}

/// First line of `data` (up to and including the first `\n`), like `readline`.
fn first_line(data: &[u8]) -> (&[u8], &[u8]) {
    match data.iter().position(|&b| b == b'\n') {
        Some(i) => (&data[..=i], &data[i + 1..]),
        None => (data, &[]),
    }
}

fn is_blank(line: &[u8]) -> bool {
    // blank_re = rb'^[ \t\f]*(?:[#\r\n]|$)'
    let rest = line
        .iter()
        .position(|&b| !matches!(b, b' ' | b'\t' | b'\x0c'))
        .map_or(&[][..], |i| &line[i..]);
    rest.is_empty() || matches!(rest[0], b'#' | b'\r' | b'\n')
}

/// `_get_normal_name` from `tokenize.py`.
fn normal_name(orig: &str) -> String {
    let enc: String = orig
        .chars()
        .take(12)
        .collect::<String>()
        .to_lowercase()
        .replace('_', "-");
    if enc == "utf-8" || enc.starts_with("utf-8-") {
        return "utf-8".to_string();
    }
    if enc == "latin-1"
        || enc == "iso-8859-1"
        || enc == "iso-latin-1"
        || enc.starts_with("latin-1-")
        || enc.starts_with("iso-8859-1-")
        || enc.starts_with("iso-latin-1-")
    {
        return "iso-8859-1".to_string();
    }
    orig.to_string()
}

fn find_cookie(line: &[u8], bom: bool) -> Result<Option<String>, DecodeError> {
    let text = std::str::from_utf8(line).map_err(|_| DecodeError::InvalidDeclaration)?;
    let Some(m) = COOKIE_RE.captures(text) else {
        return Ok(None);
    };
    let encoding = normal_name(&m[1]);
    if Codec::lookup(&encoding).is_none() {
        return Err(DecodeError::UnknownEncoding(encoding));
    }
    if bom && encoding != "utf-8" {
        return Err(DecodeError::EncodingProblem);
    }
    Ok(Some(encoding))
}

/// `tokenize.detect_encoding` over an in-memory buffer.
pub fn detect_encoding(data: &[u8]) -> Result<Detected, DecodeError> {
    let (mut bom, mut data) = (false, data);
    if data.starts_with(BOM_UTF8) {
        bom = true;
        data = &data[3..];
    }
    let default = || Detected {
        encoding: "utf-8".to_string(),
        bom,
    };
    let (first, rest) = first_line(data);
    if first.is_empty() {
        return Ok(default());
    }
    if let Some(enc) = find_cookie(first, bom)? {
        return Ok(Detected { encoding: enc, bom });
    }
    if !is_blank(first) {
        return Ok(default());
    }
    let (second, _) = first_line(rest);
    if second.is_empty() {
        return Ok(default());
    }
    if let Some(enc) = find_cookie(second, bom)? {
        return Ok(Detected { encoding: enc, bom });
    }
    Ok(default())
}

/// A codec able to decode Python source.
#[derive(Debug, Clone)]
pub enum Codec {
    Utf8,
    Latin1,
    Ascii,
    Other(&'static encoding_rs::Encoding),
}

impl Codec {
    /// Python `codecs.lookup` for the aliases that matter for source files.
    pub fn lookup(name: &str) -> Option<Codec> {
        let n = name.to_lowercase().replace('_', "-");
        let n = n.as_str();
        match n {
            "utf-8" | "utf8" | "u8" | "utf" | "cp65001" | "utf-8-sig" => return Some(Codec::Utf8),
            "iso-8859-1" | "latin-1" | "latin1" | "l1" | "iso8859-1" | "8859" | "cp819"
            | "iso88591" | "iso-latin-1" | "latin" => return Some(Codec::Latin1),
            "ascii" | "us-ascii" | "646" | "ansi-x3.4-1968" | "iso646-us" | "us" => {
                return Some(Codec::Ascii);
            }
            _ => {}
        }
        // Python accepts a few forms encoding_rs does not know.
        let candidate = if let Some(rest) = n.strip_prefix("iso8859-") {
            format!("iso-8859-{rest}")
        } else if let Some(rest) = n.strip_prefix("latin") {
            match rest {
                "2" => "iso-8859-2".to_string(),
                "3" => "iso-8859-3".to_string(),
                "4" => "iso-8859-4".to_string(),
                "5" => "iso-8859-9".to_string(),
                "6" => "iso-8859-10".to_string(),
                "7" => "iso-8859-13".to_string(),
                "8" => "iso-8859-14".to_string(),
                "9" => "iso-8859-15".to_string(),
                "10" => "iso-8859-16".to_string(),
                _ => n.to_string(),
            }
        } else if let Some(rest) = n.strip_prefix("cp") {
            format!("windows-{rest}")
        } else {
            n.to_string()
        };
        let enc = encoding_rs::Encoding::for_label(candidate.as_bytes())?;
        // encoding_rs maps a few labels to replacement/UTF-16 which cannot
        // decode Python source meaningfully; keep them anyway.
        Some(Codec::Other(enc))
    }

    /// Strictly decode `data`.
    pub fn decode(&self, data: &[u8]) -> Result<String, DecodeError> {
        match self {
            Codec::Utf8 => std::str::from_utf8(data)
                .map(str::to_string)
                .map_err(|_| DecodeError::Undecodable("utf-8".into())),
            Codec::Latin1 => Ok(data.iter().map(|&b| b as char).collect()),
            Codec::Ascii => {
                if data.is_ascii() {
                    Ok(String::from_utf8_lossy(data).into_owned())
                } else {
                    Err(DecodeError::Undecodable("ascii".into()))
                }
            }
            Codec::Other(enc) => enc
                .decode_without_bom_handling_and_without_replacement(data)
                .map(|c| c.into_owned())
                .ok_or_else(|| DecodeError::Undecodable(enc.name().to_string())),
        }
    }
}

/// Decode a Python source file the way CPython's tokenizer does: honour a
/// BOM and a PEP 263 cookie, default to UTF-8, strip the BOM.
pub fn decode_source(data: &[u8]) -> Result<String, DecodeError> {
    let detected = detect_encoding(data)?;
    let body = if detected.bom { &data[3..] } else { data };
    let codec = Codec::lookup(&detected.encoding)
        .ok_or_else(|| DecodeError::UnknownEncoding(detected.encoding.clone()))?;
    let text = codec.decode(body)?;
    if text.as_bytes().contains(&0) {
        return Err(DecodeError::NullBytes);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cookies() {
        assert_eq!(detect_encoding(b"x = 1\n").unwrap().encoding, "utf-8");
        assert_eq!(
            detect_encoding(b"# -*- coding: latin-1 -*-\nx\n")
                .unwrap()
                .encoding,
            "iso-8859-1"
        );
        assert_eq!(
            detect_encoding(b"#!/usr/bin/env python3\n# -*- coding: latin-1 -*-\n")
                .unwrap()
                .encoding,
            "iso-8859-1"
        );
        // cookie on line 2 only counts when line 1 is blank or a comment
        assert_eq!(
            detect_encoding(b"x = 1\n# coding: latin-1\n")
                .unwrap()
                .encoding,
            "utf-8"
        );
        assert_eq!(
            detect_encoding(b"# vim: set fileencoding=utf8 :\n")
                .unwrap()
                .encoding,
            "utf8"
        );
        assert_eq!(
            detect_encoding(b"\xef\xbb\xbfx = 1\n").unwrap(),
            Detected {
                encoding: "utf-8".into(),
                bom: true
            }
        );
        assert_eq!(
            detect_encoding(b"\xef\xbb\xbf# coding: latin-1\n"),
            Err(DecodeError::EncodingProblem)
        );
        assert_eq!(
            detect_encoding(b"# coding: nonsense-x\n"),
            Err(DecodeError::UnknownEncoding("nonsense-x".into()))
        );
        assert_eq!(
            detect_encoding(b"\x1f\x8b\x08\x08\xff\n"),
            Err(DecodeError::InvalidDeclaration)
        );
    }

    #[test]
    fn decodes() {
        assert_eq!(
            decode_source(b"# coding: latin-1\nx = '\xe9'\n").unwrap(),
            "# coding: latin-1\nx = '\u{e9}'\n"
        );
        assert_eq!(decode_source(b"\xef\xbb\xbfx = 1\n").unwrap(), "x = 1\n");
        assert!(decode_source(b"x = '\xe9'\n").is_err());
        assert!(decode_source(b"# coding: ascii\nx = '\xe9'\n").is_err());
        assert_eq!(
            decode_source(b"# coding: cp1252\nx = '\x80'\n").unwrap(),
            "# coding: cp1252\nx = '\u{20ac}'\n"
        );
        assert!(decode_source(b"x = 1\x00\n").is_err());
        assert_eq!(decode_source(b"").unwrap(), "");
    }
}
