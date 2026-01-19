//! Smaug's Treasure - A payment processing engine
//!
//! This library provides a robust payment processing system that handles:
//! - Deposits and withdrawals
//! - Disputes, resolves, and chargebacks
//! - Fixed-point arithmetic for accurate monetary calculations
//! - Streaming CSV processing for memory efficiency
//!
//! # Example
//!
//! ```no_run
//! use smaugs_treasure::{PaymentProcessor, CsvProcessor};
//!
//! let mut processor = PaymentProcessor::new();
//! let mut csv = CsvProcessor::from_path("transactions.csv").unwrap();
//!
//! csv.process_stream(|result| {
//!     if let Ok(tx) = result {
//!         let _ = processor.process_transaction(tx);
//!     }
//! }).unwrap();
//!
//! let accounts = processor.finalize();
//! ```

pub mod cli;

// Re-export commonly used types
pub use cli::{show_hire_info, Cli};
