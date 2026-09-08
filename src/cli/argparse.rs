//! Minimal argparse-compatible argument parser. See PLAN.md (M7).
//!
//! Requirements (the runtime tests check the literal strings
//! `usage: bandit [-h]` and `positional arguments:`):
//! * options: `-x/--long` with actions store / store_true / count / version /
//!   help, `nargs="?"` (bare `-o` → `None`), `choices`, `default`,
//!   `metavar`, `dest`; positionals with `nargs="*"`/`"+"`;
//! * mutually exclusive groups (`-l` vs `--severity-level`, `-i` vs
//!   `--confidence-level`, `-v` vs `-q/--quiet/--silent`);
//! * bundled short flags (`-lll`, `-iii`, `-rv`), `--opt=value`, `--` end of
//!   options, unambiguous long-option abbreviations;
//! * `-h` prints usage + description + `positional arguments:` + `options:`
//!   sections (argparse `HelpFormatter`: two-column layout, 24-char help
//!   position, terminal width `COLUMNS - 2` defaulting to 78) + epilog
//!   (`RawDescriptionHelpFormatter` keeps the epilog verbatim), exit 0;
//! * errors print `usage: ...` then `bandit: error: <message>` to stderr and
//!   exit 2 (`argument -f/--format: invalid choice: 'x' (choose from 'csv', ...)`,
//!   `unrecognized arguments: ...`, `argument -n/--number: expected one argument`,
//!   `argument -q/--quiet: not allowed with argument -v/--verbose`).
//!
//! Status: stub.

/// Parsed values (string, list of strings, count, bool).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Str(String),
    List(Vec<String>),
    Count(u32),
    Bool(bool),
}

/// TODO(M7): parser definition (options, positionals, groups) and `parse(args) -> Result<Parsed, ExitCode>`.
pub struct Parser;
