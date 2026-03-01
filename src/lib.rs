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
//! processor.finalize_to_csv();
//! ```

pub mod app;
pub mod cli;
pub mod csv_processor;
pub mod engine;
pub mod error;
pub mod storage;
pub mod types;

// Re-export commonly used types
pub use cli::{show_hire_info, Cli};
pub use csv_processor::CsvProcessor;
pub use engine::PaymentProcessor;
pub use error::{PaymentError, Result};
pub use types::{Account, Amount, Transaction, TransactionType};
