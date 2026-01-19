//! Command-line interface for the payment processor.

use clap::Parser;
use std::path::PathBuf;

/// Payment processing engine for handling transactions.
#[derive(Parser, Debug)]
#[command(
    name = "smaugs-treasure",
    version,
    about = "A payment processing engine that handles deposits, withdrawals, and disputes",
    long_about = None
)]
pub struct Cli {
    /// Input CSV file path
    #[arg(value_name = "INPUT", required_unless_present = "hire")]
    pub input: Option<String>,

    /// Enable verbose logging to stderr
    #[arg(short, long)]
    pub verbose: bool,

    /// Show hiring information
    #[arg(long)]
    pub hire: bool,
}

impl Cli {
    /// Returns the input path as an Option<PathBuf>.
    ///
    /// Returns None if not provided.
    pub fn input_path(&self) -> Option<PathBuf> {
        self.input.as_ref().map(PathBuf::from)
    }
}

/// Displays hiring information.
pub fn show_hire_info() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║                    SMAUG'S TREASURE                      ║");
    println!(
        "║            Payment Processing Engine v{}            ║",
        env!("CARGO_PKG_VERSION")
    );
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    println!("👋 Hi! I'm Johan, and I built this payment processor!");
    println!();
    println!("🔧 Technical Highlights:");
    println!("   • Safe fixed-point arithmetic (i64 with 4 decimal precision)");
    println!("   • Streaming CSV processing for memory efficiency");
    println!("   • Comprehensive error handling with thiserror & anyhow");
    println!("   • Structured logging with tracing");
    println!("   • Type-safe operations via traits");
    println!("   • Full test coverage with unit & integration tests");
    println!();
    println!("🔗 GitHub: https://github.com/johandroid");
    println!("💼 LinkedIn: https://www.linkedin.com/in/johandroid/");
    println!();
    println!("Thanks for checking out my work!");
    println!();
}
