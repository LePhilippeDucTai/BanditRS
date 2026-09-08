//! `urllib.parse.quote(s, safe='/')` and `PurePath.as_uri()` (`file://` +
//! quoted absolute path) for the SARIF `artifactLocation.uri`.

/// `urllib.parse.quote(s, safe=safe)`: percent-encode every byte that is not
/// an RFC 3986 unreserved character (`A-Za-z0-9_.-~`) or in `safe`.
pub fn quote(s: &str, safe: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.as_bytes() {
        let c = *byte as char;
        if byte.is_ascii_alphanumeric() || b"_.-~".contains(byte) || safe.as_bytes().contains(byte) {
            out.push(c);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `PurePath(p).as_uri()` for an absolute POSIX path: `file://` + `quote(p, safe='/')`.
pub fn as_file_uri(abs_path: &str) -> String {
    format!("file://{}", quote(abs_path, "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_reserved_bytes() {
        assert_eq!(quote("examples/assert.py", "/"), "examples/assert.py");
        assert_eq!(quote("a b/c#d.py", "/"), "a%20b/c%23d.py");
    }

    #[test]
    fn file_uri() {
        assert_eq!(as_file_uri("/abs/path with space.py"), "file:///abs/path%20with%20space.py");
    }
}
