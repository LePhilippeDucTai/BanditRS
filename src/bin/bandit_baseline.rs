fn main() {
    std::process::exit(banditrs::cli::baseline::main(
        std::env::args().skip(1).collect(),
    ));
}
