//! `banditrs` command-line entry point.
//!
//! Strictly identical to `bandit`; it exists so that BanditRS can be installed
//! in a virtualenv that already provides the PyCQA `bandit` command without the
//! two packages fighting over the same file name.

fn main() {
    std::process::exit(banditrs::cli::entrypoint(
        std::env::args().skip(1).collect(),
    ));
}
