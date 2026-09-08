//! `bandit` command-line entry point.
//!
//! The full CLI (`banditrs::cli::main`) is not implemented yet; the hidden
//! `--dump-walk FILE` option is available for traversal comparisons with the
//! Python implementation (see `scripts/dump_walk.py`).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "--dump-walk" {
        let compat = banditrs::ast::PyCompat::from_env();
        match banditrs::ast::trace::dump_walk(&args[1], compat) {
            Ok(text) => {
                println!("{text}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(2);
            }
        }
    }
    std::process::exit(banditrs::cli::main::main(args));
}
