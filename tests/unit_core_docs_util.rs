//! Port of `tests/unit/core/test_docs_util.py` (`docs_utils.get_url`).
//!
//! Work package: `docs/plan/wp/WP-11-unit-core-issue-blacklisting-docs.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/unit/core/test_docs_util.py`.

use banditrs::core::docs_utils::{base_url, get_url};

/// Port of `tests/unit/core/test_docs_util.py::DocsUtilTests::test_import_call_bib`.
#[test]
fn test_import_call_bib() {
    let expected_url = format!(
        "{}{}",
        base_url(),
        "blacklists/blacklist_imports.html#b413-import-pycrypto"
    );
    assert_eq!(expected_url, get_url("B413"));
}

/// Port of `tests/unit/core/test_docs_util.py::DocsUtilTests::test_overwrite_bib_info`.
#[test]
fn test_overwrite_bib_info() {
    let expected_url = format!(
        "{}{}",
        base_url(),
        "blacklists/blacklist_calls.html#b304-b305-ciphers-and-modes"
    );
    assert_eq!(get_url("B304"), get_url("B305"));
    assert_eq!(expected_url, get_url("B304"));
}

/// Port of `tests/unit/core/test_docs_util.py::DocsUtilTests::test_plugin_call_bib`.
#[test]
fn test_plugin_call_bib() {
    let expected_url = format!("{}{}", base_url(), "plugins/b101_assert_used.html");
    assert_eq!(expected_url, get_url("B101"));
}
