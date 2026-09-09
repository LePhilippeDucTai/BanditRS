//! Command-line interfaces (`bandit`, `bandit-baseline`,
//! `bandit-config-generator`).

pub mod argparse;
pub mod baseline;
pub mod config_generator;
pub mod main;

/// Shared body of the `bandit` and `banditrs` executables.
///
/// `banditrs` is an alias installed alongside `bandit` by the Python wheel, so
/// that BanditRS can coexist with the PyCQA `bandit` package in the same
/// environment. Both names must behave identically, hence this single body.
///
/// The hidden `--dump-walk FILE` option is a development aid (AST walk trace,
/// compared against `scripts/dump_walk.py`); everything else is delegated to
/// [`main::main`].
pub fn entrypoint(args: Vec<String>) -> i32 {
    if args.len() == 2 && args[0] == "--dump-walk" {
        let compat = crate::ast::PyCompat::from_env();
        return match crate::ast::trace::dump_walk(&args[1], compat) {
            Ok(text) => {
                println!("{text}");
                0
            }
            Err(e) => {
                eprintln!("error: {e}");
                2
            }
        };
    }
    main::main(args)
}
