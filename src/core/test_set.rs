//! The set of tests selected by a profile (port of `bandit/core/test_set.py`).
//! See docs/spec/core.md §5. Status: implemented.
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

use indexmap::IndexSet;

use crate::ast::NodeKind;
use crate::core::blacklist::{self, BlacklistTable};
use crate::core::config::{BanditConfig, Profile};
use crate::core::plugin_config::PluginConfigs;
use crate::core::registry::{self, PLUGINS, PluginDef};

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

/// Build a [`BlacklistTable`] from the legacy `profile["blacklist"]` mapping
/// (`{"Call": [...], "Import": [...], "ImportFrom": [...]}`).
fn legacy_blacklist_table(map: &indexmap::IndexMap<String, Vec<crate::core::blacklist::BlacklistEntry>>) -> BlacklistTable {
    let get = |k: &str| map.get(k).cloned().unwrap_or_default();
    BlacklistTable::from_parts(get("Call"), get("Import"), get("ImportFrom"))
}

impl TestSet {
    /// `BanditTestSet(config, profile)`.
    pub fn new(config: &BanditConfig, profile: &Profile) -> TestSet {
        let all_blacklist_ids: IndexSet<String> = blacklist::all_entries().map(|e| e.id.to_string()).collect();

        let mut inc: IndexSet<String> = profile.include.clone();
        let mut exc: IndexSet<String> = profile.exclude.clone();
        if inc.contains("B001") {
            if inc.is_disjoint(&all_blacklist_ids) {
                inc.extend(all_blacklist_ids.iter().cloned());
            }
            inc.shift_remove("B001");
        }
        if exc.contains("B001") {
            if exc.is_disjoint(&all_blacklist_ids) {
                exc.extend(all_blacklist_ids.iter().cloned());
            }
            exc.shift_remove("B001");
        }

        let filtered: IndexSet<String> = if !inc.is_empty() {
            inc
        } else {
            let mut s: IndexSet<String> = PLUGINS.iter().map(|p| p.id.to_string()).collect();
            s.extend(registry::BUILTIN.iter().map(|s| s.to_string()));
            s.extend(all_blacklist_ids.iter().cloned());
            s
        };
        let filtered: IndexSet<String> = filtered.into_iter().filter(|id| !exc.contains(id)).collect();

        let mut tests: Vec<Vec<TestRef>> = (0..NodeKind::COUNT).map(|_| Vec::new()).collect();
        for plugin in PLUGINS.iter().filter(|p| filtered.contains(p.id)) {
            for &kind in plugin.checks {
                tests[kind as usize].push(TestRef::Plugin(plugin));
            }
        }

        let table = match &profile.blacklist {
            Some(legacy) => legacy_blacklist_table(legacy),
            None => BlacklistTable::builtin().filtered(|id| filtered.contains(id)),
        };
        let blacklist = if table.is_empty() {
            None
        } else {
            if !table.call.is_empty() {
                tests[NodeKind::Call as usize].push(TestRef::Blacklist);
            }
            if !table.import.is_empty() {
                tests[NodeKind::Import as usize].push(TestRef::Blacklist);
            }
            if !table.import_from.is_empty() {
                tests[NodeKind::ImportFrom as usize].push(TestRef::Blacklist);
            }
            Some(table)
        };

        TestSet { tests, blacklist, configs: PluginConfigs::from_config(config) }
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
