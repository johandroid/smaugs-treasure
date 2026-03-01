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

/// Type of a monetary transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonetaryTxKind {
    Deposit,
    Withdrawal,
}

/// Stored monetary transaction data used as source-of-truth for references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonetaryTx {
    /// Client ID that owns this transaction.
    pub client_id: ClientId,
    /// Transaction amount.
    pub amount: Amount,
    /// Transaction kind.
    pub kind: MonetaryTxKind,
}

impl MonetaryTx {
    /// Returns true when this transaction is a deposit.
    pub fn is_deposit(&self) -> bool {
        matches!(self.kind, MonetaryTxKind::Deposit)
    }
}

#[cfg(test)]
mod tests;
