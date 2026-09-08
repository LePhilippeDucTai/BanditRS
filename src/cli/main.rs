//! The `bandit` command (port of `bandit/cli/main.py`). See PLAN.md (M7).

/// Run the `bandit` command with the given arguments (without the program
/// name) and return the process exit code.
pub fn main(_args: Vec<String>) -> i32 {
    eprintln!("bandit: the command-line interface is not implemented yet (see PLAN.md, milestone M7)");
    2
}
