use clap::Parser;
use log::{error, info};

mod connections;
mod core;
mod ui;
mod utils;

use ui::cli::cli_commands::Args;
use utils::logging::init_logging;

fn main() {
    init_logging();

    // Parse arguments
    let args = Args::parse();

    if args.gui {
        // Launch GUI
        match ui::gui::window::launch_gui(args) {
            Ok(_) => info!("GUI closed gracefully."),
            Err(e) => error!("Failed to launch GUI: {:?}", e),
        }
    } else {
        // Run CLI
        if let Err(e) = ui::cli::cli_commands::run_cli(args) {
            error!("CLI error: {:?}", e);
        } else {
            info!("CLI run completed successfully.");
        }
    }
}
