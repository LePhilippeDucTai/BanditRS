fn main() {
    std::process::exit(banditrs::cli::config_generator::main(std::env::args().skip(1).collect()));
}
