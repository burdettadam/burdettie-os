fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(burdettie_xcode_tools::run_xcodebuild(&args));
}
