//! Error types for the payment processing engine.

use thiserror::Error;

/// Result type alias for the payment engine.
pub type Result<T> = std::result::Result<T, PaymentError>;

/// Main error type for the payment processing engine.
#[derive(Error, Debug)]
pub enum PaymentError {
    /// Amount-related errors
    #[error(transparent)]
    Amount(#[from] AmountError),

    /// Parse-related errors
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// Processing errors
    #[error(transparent)]
    Processing(#[from] ProcessingError),

    /// CSV errors
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to amount operations.
#[derive(Error, Debug, PartialEq)]
pub enum AmountError {
    #[error("Amount overflow occurred")]
    Overflow,

    #[error("Amount underflow occurred")]
    Underflow,

    #[error("Failed to parse amount: {0}")]
    ParseError(String),
}

/// Errors related to transaction parsing.
#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Invalid transaction type: {0}")]
    InvalidTransactionType(String),

    #[error("Missing amount for transaction {0}")]
    MissingAmount(u32),

    #[error("Negative amount not allowed for transaction {0}")]
    NegativeAmount(u32),

    #[error("Invalid CSV row: {0}")]
    InvalidCsvRow(String),
}

/// Errors that occur during transaction processing.
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Insufficient funds for withdrawal: client {client}, available {available}, requested {requested}")]
    InsufficientFunds {
        client: u16,
        available: crate::types::Amount,
        requested: crate::types::Amount,
    },

    #[error("Transaction {tx_id} not found for dispute by client {client}")]
    TransactionNotFound { client: u16, tx_id: u32 },

    #[error("Account {client} is locked")]
    AccountLocked { client: u16 },

    #[error("Dispute not found for transaction {tx_id} by client {client}")]
    DisputeNotFound { client: u16, tx_id: u32 },

    #[error("Transaction {tx_id} already disputed (client {client})")]
    AlreadyDisputed { client: u16, tx_id: u32 },

    #[error("Dispute not active for transaction {tx_id} (client {client})")]
    DisputeNotActive { client: u16, tx_id: u32 },

    #[error("Client mismatch: transaction {tx_id} belongs to client {owner}, not {requester}")]
    ClientMismatch {
        tx_id: u32,
        owner: u16,
        requester: u16,
    },

    #[error("Duplicate transaction ID: {tx_id}")]
    DuplicateTransaction { tx_id: u32 },
}

impl ProcessingError {
    /// Creates an insufficient funds error.
    pub fn insufficient_funds(
        client: u16,
        available: crate::types::Amount,
        requested: crate::types::Amount,
    ) -> Self {
        ProcessingError::InsufficientFunds {
            client,
            available,
            requested,
        }
    }
}
