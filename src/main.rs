//! Main entry point for the Smaug's Treasure payment processor.

use anyhow::{Context, Result};
use clap::Parser;
use smaugs_treasure::{
    cli::{show_hire_info, Cli},
    csv_processor::CsvProcessor,
    engine::PaymentProcessor,
};
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

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

    // Run the main processing logic
    run_processor(cli)
}

/// Sets up the tracing subscriber for logging.
/// Tracing is only enabled when verbose mode is active.
/// All logs are written to stderr since stdout is used for CSV output.
fn setup_tracing(verbose: bool) -> Result<()> {
    if !verbose {
        return Ok(());
    }

    let filter = EnvFilter::new("smaugs_treasure=debug,warn,error");

    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_level(true)
        .init();

    Ok(())
}

/// Main processing logic.
fn run_processor(cli: Cli) -> Result<()> {
    info!("Starting Smaug's Treasure payment processor");

    // Create payment processor
    let mut processor = PaymentProcessor::new();

    // Get input path
    let input_path = cli.input_path().context("Failed to get input path")?;
    info!("Reading transactions from file: {:?}", input_path);

    // Process transactions from CSV
    let mut csv_processor =
        CsvProcessor::from_path(&input_path).context("Failed to open input file")?;
    process_csv(&mut csv_processor, &mut processor, cli.verbose)?;

    // Finalize and print CSV to stdout
    processor.finalize_to_csv();

    Ok(())
}

/// Processes transactions from a CSV processor.
fn process_csv(
    csv_processor: &mut CsvProcessor,
    processor: &mut PaymentProcessor,
    verbose: bool,
) -> Result<()> {
    csv_processor.process_stream(|result| {
        match result {
            Ok(transaction) => {
                // Process the transaction
                if let Err(e) = processor.process_transaction(transaction) {
                    // Log error to stderr if verbose
                    if verbose {
                        error!("Transaction processing error: {}", e);
                    }
                    // Continue processing despite errors
                }
            }
            Err(e) => {
                // Log parsing error to stderr if verbose
                if verbose {
                    error!("Transaction parsing error: {}", e);
                }
                // Continue processing despite errors
            }
        }
    })?;

    Ok(())
}
