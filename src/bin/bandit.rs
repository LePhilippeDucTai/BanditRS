//! `bandit` command-line entry point.
//!
//! Drop-in replacement for the PyCQA `bandit` command. The body is shared with
//! the `banditrs` alias (see `banditrs::cli::entrypoint`).

fn main() {
    std::process::exit(banditrs::cli::entrypoint(
        std::env::args().skip(1).collect(),
    ));
}
