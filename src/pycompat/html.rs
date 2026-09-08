//! `html.escape(s, quote=True)`: `& < > " '` → `&amp; &lt; &gt; &quot; &#x27;`.

/// `html.escape(s, quote=True)`.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_all_five() {
        assert_eq!(escape(r#"<tag a="b" c='d'>&</tag>"#), "&lt;tag a=&quot;b&quot; c=&#x27;d&#x27;&gt;&amp;&lt;/tag&gt;");
    }
}
