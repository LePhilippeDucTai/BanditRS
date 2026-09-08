//! `xml.etree.ElementTree` escaping: text (`_escape_cdata`) escapes `& < >`;
//! attribute values (`_escape_attrib`) escape `& < > "` and `\r \n \t` as
//! `&#13; &#10; &#09;`.

/// Escape element text (`_escape_cdata`).
pub fn escape_cdata(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Escape an attribute value (`_escape_attrib`).
pub fn escape_attrib(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#09;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdata_escapes_three() {
        assert_eq!(escape_cdata("a & b < c > d \" e"), "a &amp; b &lt; c &gt; d \" e");
    }

    #[test]
    fn attrib_escapes_all() {
        assert_eq!(escape_attrib("a&b<c>d\"e\rf\ng\th"), "a&amp;b&lt;c&gt;d&quot;e&#13;f&#10;g&#09;h");
    }
}
