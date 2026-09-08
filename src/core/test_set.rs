//! The set of tests selected by a profile (port of `bandit/core/test_set.py`).
//! See docs/spec/core.md §5. Status: stub (M3).
//!
//! Semantics:
//! * `_get_filter`: `inc = profile.include`, `exc = profile.exclude`;
//!   `all_blacklist_tests` = every blacklist id. If `B001 in inc`: when `inc`
//!   has no blacklist id, add all of them; then discard `B001`. Same for
//!   `exc`. `filtered = inc if inc else (plugin ids ∪ builtin ∪ blacklist ids)`;
//!   result = `filtered - exc`.
//! * `plugins` = `PLUGINS` whose id is in the filter (registry order).
//! * `_load_builtins`: when the profile carries legacy `blacklist` data use
//!   it as is; otherwise keep the builtin table entries whose id is in the
//!   filter, dropping empty node types. Nothing left → no `B001` test.
//! * `_load_tests`: per node kind, the plugins registered for that kind in
//!   registry order, then `B001` (for `Call`, `Import`, `ImportFrom` when the
//!   table has entries for that type).
//! * `PluginConfigs::from_config` provides the plugin configuration.

use crate::ast::NodeKind;
use crate::core::blacklist::BlacklistTable;
use crate::core::config::{BanditConfig, Profile};
use crate::core::plugin_config::PluginConfigs;
use crate::core::registry::PluginDef;

/// A test to run for a node kind.
#[derive(Clone, Copy)]
pub enum TestRef {
    Plugin(&'static PluginDef),
    /// The builtin `B001` blacklist test.
    Blacklist,
}

/// Tests selected for a run.
pub struct TestSet {
    /// Tests per node kind (`NodeKind as usize`).
    pub tests: Vec<Vec<TestRef>>,
    /// Blacklist entries enabled by the profile (`None` when `B001` is off).
    pub blacklist: Option<BlacklistTable>,
    pub configs: PluginConfigs,
}

impl TestSet {
    /// `BanditTestSet(config, profile)`.
    /// TODO(M3): implement the filtering described in the module docs.
    pub fn new(_config: &BanditConfig, _profile: &Profile) -> TestSet {
        todo!("M3: TestSet::new")
    }

    /// `get_tests(checktype)`.
    pub fn get_tests(&self, kind: NodeKind) -> &[TestRef] {
        self.tests.get(kind as usize).map(Vec::as_slice).unwrap_or(&[])
    }

    /// `not b_mgr.b_ts.tests` — whether no test at all would run.
    pub fn is_empty(&self) -> bool {
        self.tests.iter().all(Vec::is_empty)
    }
}
