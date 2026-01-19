//! Core types used throughout the payment processing engine.

mod account;
mod amount;
mod transaction;

pub use account::Account;
pub use amount::Amount;
pub use transaction::{Transaction, TransactionType};

/// Type alias for transaction IDs.
pub type TxId = u32;
pub type ClientId = u16;

/// Stored deposit data for dispute resolution.
///
/// Only deposits are stored since withdrawals cannot be disputed.
/// This minimal representation optimizes memory usage by storing only
/// the essential fields needed for dispute operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredDeposit {
    /// Client ID that owns the deposit
    pub client_id: u16,
    /// Deposit amount
    pub amount: Amount,
}

#[cfg(test)]
mod tests;
