//! Documentation links (port of `bandit/core/docs_utils.py`).

use crate::core::{blacklist, registry};

/// `https://bandit.readthedocs.io/en/<DOCS_VERSION>/`.
pub fn base_url() -> String {
    format!("https://bandit.readthedocs.io/en/{}/", crate::DOCS_VERSION)
}

/// `get_url(bid)`.
pub fn get_url(bid: &str) -> String {
    let base = base_url();
    if let Some(p) = registry::plugin_by_id(bid) {
        return format!("{base}plugins/{}_{}.html", bid.to_lowercase(), p.func_name);
    }
    if let Some(e) = blacklist::entry_by_id(bid) {
        let mut id = e.id.to_string();
        let mut name = e.name.replace('_', "-");
        let kind = if id.starts_with("B3") {
            if id == "B304" || id == "B305" {
                id = "b304-b305".to_string();
                name = "ciphers-and-modes".to_string();
            }
            if matches!(
                id.as_str(),
                "B313" | "B314" | "B315" | "B316" | "B317" | "B318" | "B319" | "B320"
            ) {
                id = "b313-b320".to_string();
            }
            "calls"
        } else {
            "imports"
        };
        let ext = format!("blacklists/blacklist_{kind}.html#{id}-{name}");
        return format!("{base}{}", ext.to_lowercase());
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        let base = base_url();
        assert_eq!(get_url("B304"), get_url("B305"));
        assert_eq!(
            get_url("B304"),
            format!("{base}blacklists/blacklist_calls.html#b304-b305-ciphers-and-modes")
        );
        assert_eq!(
            get_url("B101"),
            format!("{base}plugins/b101_assert_used.html")
        );
        assert_eq!(
            get_url("B413"),
            format!("{base}blacklists/blacklist_imports.html#b413-import-pycrypto")
        );
        assert_eq!(get_url("B324"), format!("{base}plugins/b324_hashlib.html"));
        assert_eq!(
            get_url("B313"),
            format!("{base}blacklists/blacklist_calls.html#b313-b320-xml-bad-celementtree")
        );
        assert_eq!(get_url("B999"), base);
    }
}
