use env_logger;
use log::LevelFilter;
// use std::fs::File;

/// Initialize logging using env_logger.
/// By default, this reads the RUST_LOG environment variable for filtering.
/// e.g., `RUST_LOG=my_putty_clone=debug cargo run -- --port /dev/ttyUSB0`
pub fn init_logging() {
    // let log_file = File::create("my.log").unwrap();
    env_logger::Builder::from_default_env()
        // .target(Target::Pipe(Box::new(log_file)))
        .filter(None, LevelFilter::Info)
        .init();
}
