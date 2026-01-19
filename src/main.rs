use anyhow::{Context, Result};
use clap::Parser;
use smaugs_treasure::cli::{show_hire_info, Cli};
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

/// Sets up the tracing subscriber for logging.
/// All the logs will be written to stderr since the stdout is used for CSV output.
fn setup_tracing(verbose: bool) -> Result<()> {
    let filter = if verbose {
        EnvFilter::new("smaugs_treasure=debug,warn") // Not included info to reduce noise
    } else {
        // Only show errors by default
        EnvFilter::new("smaugs_treasure=error")
    };

    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr) // Write logs to stderr
        .with_target(false)
        .with_level(true)
        .init();

    Ok(())
}

fn main() -> Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse();

    // Handle --hire flag
    if cli.hire {
        show_hire_info();
        return Ok(());
    }

    // Setup tracing/logging
    setup_tracing(cli.verbose)?;

    Ok(())
}
