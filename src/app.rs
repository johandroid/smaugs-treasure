//! Application orchestration for the payment engine binary.

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

use crate::{
    cli::{show_hire_info, Cli},
    csv_processor::CsvProcessor,
    engine::{PaymentProcessor, ProcessingOutcome},
};

/// Runs the CLI application.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.hire {
        show_hire_info();
        return Ok(());
    }

    setup_tracing(cli.verbose)?;

    let result = run_processor(cli);
    if let Err(ref e) = result {
        error!("Application error: {}", e);
    }
    result
}

/// Sets up tracing when verbose mode is enabled.
fn setup_tracing(verbose: bool) -> Result<()> {
    if !verbose {
        return Ok(());
    }

    let filter = EnvFilter::new("smaugs_treasure=info,warn,error");

    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_level(true)
        .init();

    Ok(())
}

fn run_processor(cli: Cli) -> Result<()> {
    info!("Starting payment processor");

    let mut processor = PaymentProcessor::new();
    let input_path = cli.input_path().context("failed to get input path")?;
    info!("Reading transactions from file: {:?}", input_path);

    let mut csv_processor =
        CsvProcessor::from_path(&input_path).context("failed to open input file")?;
    process_csv(&mut csv_processor, &mut processor)?;
    if let Err(e) = processor.finalize_to_csv() {
        error!("Failed to write accounts CSV to stdout: {}", e);
        return Err(e).context("failed to write accounts CSV to stdout");
    }
    Ok(())
}

fn process_csv(csv_processor: &mut CsvProcessor, processor: &mut PaymentProcessor) -> Result<()> {
    csv_processor.process_stream(|result| match result {
        Ok(transaction) => match processor.process_transaction(transaction) {
            Ok(ProcessingOutcome::Applied | ProcessingOutcome::Ignored(_)) => {}
            Err(e) => error!("Transaction processing error: {}", e),
        },
        Err(e) => error!("Transaction parsing error: {}", e),
    })?;

    Ok(())
}
