fn main() {
    if let Err(e) = trinedocs::mcp::shim::run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
