//! Port of `bandit/plugins/asserts.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyErr;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::{ListOpt, PluginConfigs};
use crate::plugins::PluginResult;
use crate::pycompat::fnmatch::fnmatch;

/// `assert_used` (B101): `config.get("skips", [])` — a missing `skips` key
/// behaves like the empty-list default, an explicit `null` still raises
/// (Python would try to iterate `None`).
pub fn assert_used(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let skips: &[String] = match &cfg.assert_used.skips {
        ListOpt::Missing => &[],
        ListOpt::Null => return Err(PyErr::type_error("'NoneType' object is not iterable")),
        ListOpt::Items(items) => items,
    };
    if skips.iter().any(|skip| fnmatch(ctx.filename(), skip)) {
        return Ok(None);
    }
    Ok(Some(IssueDraft::new(
        Rank::Low,
        Rank::High,
        Cwe::IMPROPER_CHECK_OF_EXCEPT_COND,
        "Use of assert detected. The enclosed code will be removed when compiling to optimised byte code.",
    )))
}
