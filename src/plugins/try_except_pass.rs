//! Port of `bandit/plugins/try_except_pass.py` — see docs/spec/plugins.md.

use ruff_python_ast::{Expr, Stmt};

use crate::ast::VNode;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `try_except_pass` (B110).
pub fn try_except_pass(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let Some(VNode::ExceptHandler(node)) = ctx.node else {
        return Ok(None);
    };
    if node.body.len() != 1 {
        return Ok(None);
    }
    let check_typed_exception = cfg.try_except_pass.check_typed_exception.clone()?;
    if !check_typed_exception && let Some(ty) = &node.type_ {
        let is_exception = matches!(&**ty, Expr::Name(n) if n.id.as_str() == "Exception");
        if !is_exception {
            return Ok(None);
        }
    }
    if matches!(node.body[0], Stmt::Pass(_)) {
        return Ok(Some(IssueDraft::new(
            Rank::Low,
            Rank::High,
            Cwe::IMPROPER_CHECK_OF_EXCEPT_COND,
            "Try, Except, Pass detected.",
        )));
    }
    Ok(None)
}
