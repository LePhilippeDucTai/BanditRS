//! Port of `tests/unit/core/test_test_set.py` (`BanditTestSet`, profils, blacklists).
//!
//! Work package: `docs/plan/wp/WP-10-unit-core-test-set.md` — port every stub below
//! (remove `#[ignore]`, keep the function name) so that
//! `docs/plan/test-inventory.md` stays the single source of truth.
//! Python reference: `/home/user/bandit/tests/unit/core/test_test_set.py`.
//!
//! The Python suite injects a fake plugin registry: one plugin (`B000`, checks `Str`)
//! plus a two-entry blacklist (`B401` telnet, `B302` marshal) valid for `Import`,
//! `ImportFrom` and `Call`. `BanditTestSet` is not injectable in Rust (the registry is
//! `static`), so every test below is re-derived from the **real** registry and computes
//! its own expected values from `registry::PLUGINS` / `blacklist::BlacklistTable::builtin()`
//! instead of hard-coding counts — the test stays true as plugins are added upstream.
//! Correspondences used throughout: `B000`/`Str` → `B105` (`hardcoded_password_string`,
//! the only property the fake plugin needs — a real id that checks `Str`); `B401`/`B302`
//! → themselves (both are real blacklist ids, telnet and marshal, unchanged).

use indexmap::{IndexMap, IndexSet};

use banditrs::ast::NodeKind;
use banditrs::core::blacklist::{self, BlacklistEntry, BlacklistTable};
use banditrs::core::config::{BanditConfig, Profile};
use banditrs::core::issue::Cwe;
use banditrs::core::registry::PLUGINS;
use banditrs::core::test_set::{TestRef, TestSet};

/// `test_set.BanditTestSet(self.config, profile)` with the default config.
fn ts(profile: Profile) -> TestSet {
    TestSet::new(&BanditConfig::default(), &profile)
}

fn ids(values: &[&str]) -> IndexSet<String> {
    values.iter().map(|s| s.to_string()).collect()
}

fn include(values: &[&str]) -> Profile {
    Profile {
        include: ids(values),
        ..Profile::default()
    }
}

fn exclude(values: &[&str]) -> Profile {
    Profile {
        exclude: ids(values),
        ..Profile::default()
    }
}

/// `registry::PLUGINS` filtered by `checks.contains(&kind)` (real plugin count for a
/// node kind, computed rather than hard-coded — see the module doc-comment).
fn n_plugins(kind: NodeKind) -> usize {
    PLUGINS.iter().filter(|p| p.checks.contains(&kind)).count()
}

/// Number of `TestRef::Blacklist` entries `ts.get_tests(kind)` carries (0 or 1: the
/// builtin `B001` test is pushed at most once per node kind).
fn n_blacklist(ts: &TestSet, kind: NodeKind) -> usize {
    ts.get_tests(kind)
        .iter()
        .filter(|r| matches!(r, TestRef::Blacklist))
        .count()
}

/// Length of the `BlacklistTable` column for `kind` (0 when `ts.blacklist` is `None`).
fn table_len(ts: &TestSet, kind: NodeKind) -> usize {
    ts.blacklist
        .as_ref()
        .map(|t| match kind {
            NodeKind::Call => t.call.len(),
            NodeKind::Import => t.import.len(),
            NodeKind::ImportFrom => t.import_from.len(),
            _ => 0,
        })
        .unwrap_or(0)
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_has_defaults`
/// (adapted: fake registry `B000`/`Str` → real registry, `Str` plugin count computed
/// from `registry::PLUGINS` instead of the fixed `1` the mock gave).
#[test]
fn test_has_defaults() {
    let t = ts(Profile::default());
    let n = n_plugins(NodeKind::Str);
    assert!(n > 0, "the real registry must have at least one Str plugin");
    assert_eq!(t.get_tests(NodeKind::Str).len(), n);
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_include_id`
/// (adapted: `include: ["B000"]` → `include: ["B105"]`, a real plugin checking `Str`).
#[test]
fn test_profile_include_id() {
    let t = ts(include(&["B105"]));
    assert_eq!(t.get_tests(NodeKind::Str).len(), 1);
    // B105 checks only Str and B001 is absent from the include set, so Call gets
    // neither the plugin nor the builtin blacklist test.
    assert_eq!(t.get_tests(NodeKind::Call).len(), 0);
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_exclude_id`
/// (adapted: `exclude: ["B000"]` → `exclude: ["B105"]`).
#[test]
fn test_profile_exclude_id() {
    let t = ts(exclude(&["B105"]));
    assert_eq!(
        t.get_tests(NodeKind::Str).len(),
        n_plugins(NodeKind::Str) - 1
    );
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_include_none`
/// (adapted: same registre-réel substitution as `test_has_defaults`).
#[test]
fn test_profile_include_none() {
    let t = ts(include(&[]));
    assert_eq!(t.get_tests(NodeKind::Str).len(), n_plugins(NodeKind::Str));
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_exclude_none`
/// (adapted: same registre-réel substitution as `test_has_defaults`).
#[test]
fn test_profile_exclude_none() {
    let t = ts(exclude(&[]));
    assert_eq!(t.get_tests(NodeKind::Str).len(), n_plugins(NodeKind::Str));
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_has_builtin_blacklist`
/// (adapted: registre réel — the builtin `B001` test is present, and last, on the three
/// node kinds it applies to).
#[test]
fn test_profile_has_builtin_blacklist() {
    let t = ts(Profile::default());
    for kind in [NodeKind::Import, NodeKind::ImportFrom, NodeKind::Call] {
        assert_eq!(n_blacklist(&t, kind), 1);
        assert!(matches!(t.get_tests(kind).last(), Some(TestRef::Blacklist)));
    }
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_exclude_builtin_blacklist`
/// (adapted: registre réel).
#[test]
fn test_profile_exclude_builtin_blacklist() {
    let t = ts(exclude(&["B001"]));
    for kind in [NodeKind::Import, NodeKind::ImportFrom, NodeKind::Call] {
        assert_eq!(n_blacklist(&t, kind), 0);
    }
    assert!(t.blacklist.is_none());
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_exclude_builtin_blacklist_specific`
/// (adapted: the Python test excludes every id the fake registry's blacklist carries
/// — here every real blacklist id, from `blacklist::all_entries()` — instead of a
/// hard-coded `["B302", "B401"]`).
#[test]
fn test_profile_exclude_builtin_blacklist_specific() {
    let all_ids: Vec<&str> = blacklist::all_entries().map(|e| e.id).collect();
    let t = ts(exclude(&all_ids));
    for kind in [NodeKind::Import, NodeKind::ImportFrom, NodeKind::Call] {
        assert_eq!(n_blacklist(&t, kind), 0);
    }
    assert!(t.blacklist.is_none());
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_filter_blacklist_none`
/// (adapted: registre réel — compare against `BlacklistTable::builtin()` instead of the
/// mock's fixed `2`).
#[test]
fn test_profile_filter_blacklist_none() {
    let t = ts(Profile::default());
    let builtin = BlacklistTable::builtin();
    assert_eq!(table_len(&t, NodeKind::Import), builtin.import.len());
    assert_eq!(
        table_len(&t, NodeKind::ImportFrom),
        builtin.import_from.len()
    );
    assert_eq!(table_len(&t, NodeKind::Call), builtin.call.len());
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_filter_blacklist_one`
/// (adapted: registre réel, `exclude: ["B401"]` unchanged — a real blacklist id).
///
/// Verified empirically against the reference interpreter (`bandit.core.test_set` with
/// the real, unmocked registry): `B401` (telnetlib) is a member of the `imports.py`
/// blacklist, which `extension_loader.Manager.load_blacklists` merges into `Call` as
/// well as `Import`/`ImportFrom` (`bandit/blacklists/imports.py::gen_blacklist` returns
/// the *same* list for all three keys) — so excluding it drops the `Call` table by one
/// entry too, not just `Import`/`ImportFrom`.
#[test]
fn test_profile_filter_blacklist_one() {
    let builtin = BlacklistTable::builtin();
    let t = ts(exclude(&["B401"]));
    assert_eq!(table_len(&t, NodeKind::Import), builtin.import.len() - 1);
    assert_eq!(
        table_len(&t, NodeKind::ImportFrom),
        builtin.import_from.len() - 1
    );
    assert_eq!(table_len(&t, NodeKind::Call), builtin.call.len() - 1);
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_filter_blacklist_include`
/// (adapted: registre réel, `include: ["B001", "B401"]` unchanged).
///
/// Verified empirically against the reference interpreter (see `test_profile_filter_blacklist_one`):
/// `B401` also belongs to the real `Call` blacklist table, so `table_len(Call)` is `1`
/// here too (not `0`) and the builtin test is present on `Call`.
#[test]
fn test_profile_filter_blacklist_include() {
    let t = ts(include(&["B001", "B401"]));
    assert_eq!(table_len(&t, NodeKind::Import), 1);
    assert_eq!(table_len(&t, NodeKind::ImportFrom), 1);
    assert_eq!(table_len(&t, NodeKind::Call), 1);
    assert_eq!(n_blacklist(&t, NodeKind::Import), 1);
    assert_eq!(n_blacklist(&t, NodeKind::ImportFrom), 1);
    assert_eq!(n_blacklist(&t, NodeKind::Call), 1);
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_filter_blacklist_all`
/// (adapted: the Python test excludes every id the fake registry's blacklist carries —
/// here every real blacklist id, same construction as
/// `test_profile_exclude_builtin_blacklist_specific`).
#[test]
fn test_profile_filter_blacklist_all() {
    let all_ids: Vec<&str> = blacklist::all_entries().map(|e| e.id).collect();
    let t = ts(exclude(&all_ids));
    // if there is no blacklist data for a node type then we wont add a
    // blacklist test to it, as this would be pointless.
    for kind in [NodeKind::Import, NodeKind::ImportFrom, NodeKind::Call] {
        assert_eq!(n_blacklist(&t, kind), 0);
    }
    assert!(t.blacklist.is_none());
}

/// Port of `tests/unit/core/test_test_set.py::BanditTestSetTests::test_profile_blacklist_compat`
/// (adapted: registre réel; the legacy `blacklist` entry given by the profile is the
/// same `marshal`/`B302` datum the Python test builds via `build_conf_dict`).
#[test]
fn test_profile_blacklist_compat() {
    let marshal = BlacklistEntry {
        name: "marshal".into(),
        id: "B302".into(),
        cwe: Cwe::DESERIALIZATION_OF_UNTRUSTED_DATA,
        qualnames: vec!["marshal.load".into(), "marshal.loads".into()],
        message: "Deserialization with the marshal module is possibly dangerous.".into(),
        level: "MEDIUM".into(),
    };
    let mut legacy: IndexMap<String, Vec<BlacklistEntry>> = IndexMap::new();
    legacy.insert("Call".to_string(), vec![marshal]);

    let profile = Profile {
        include: ids(&["B001"]),
        blacklist: Some(legacy),
        ..Profile::default()
    };
    let t = ts(profile);

    assert_eq!(table_len(&t, NodeKind::Call), 1);
    assert_eq!(table_len(&t, NodeKind::Import), 0);
    assert_eq!(table_len(&t, NodeKind::ImportFrom), 0);
    assert_eq!(n_blacklist(&t, NodeKind::Import), 0);
    assert_eq!(n_blacklist(&t, NodeKind::ImportFrom), 0);
    assert_eq!(n_blacklist(&t, NodeKind::Call), 1);
}
