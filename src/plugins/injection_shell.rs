//! Port of `bandit/plugins/injection_shell.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `subprocess_popen_with_shell_equals_true` (docs/spec/plugins.md).
pub fn subprocess_popen_with_shell_equals_true(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_shell:subprocess_popen_with_shell_equals_true")
}

/// TODO(M3/M4): port `subprocess_without_shell_equals_true` (docs/spec/plugins.md).
pub fn subprocess_without_shell_equals_true(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_shell:subprocess_without_shell_equals_true")
}

/// TODO(M3/M4): port `any_other_function_with_shell_equals_true` (docs/spec/plugins.md).
pub fn any_other_function_with_shell_equals_true(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_shell:any_other_function_with_shell_equals_true")
}

/// TODO(M3/M4): port `start_process_with_a_shell` (docs/spec/plugins.md).
pub fn start_process_with_a_shell(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_shell:start_process_with_a_shell")
}

/// TODO(M3/M4): port `start_process_with_no_shell` (docs/spec/plugins.md).
pub fn start_process_with_no_shell(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_shell:start_process_with_no_shell")
}

/// TODO(M3/M4): port `start_process_with_partial_path` (docs/spec/plugins.md).
pub fn start_process_with_partial_path(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_shell:start_process_with_partial_path")
}
