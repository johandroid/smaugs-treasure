//! Transaction types and parsing for the payment processing engine.

use crate::error::{ParseError, Result};
use crate::types::Amount;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Types of transactions supported by the payment engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    /// Deposit funds into a client account
    Deposit,
    /// Withdraw funds from a client account
    Withdrawal,
    /// Dispute a transaction (hold funds)
    Dispute,
    /// Resolve a disputed transaction (release held funds)
    Resolve,
    /// Chargeback a disputed transaction (reverse and lock account)
    Chargeback,
}

impl FromStr for TransactionType {
    type Err = ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, ParseError> {
        match s.trim().to_lowercase().as_str() {
            "deposit" => Ok(TransactionType::Deposit),
            "withdrawal" => Ok(TransactionType::Withdrawal),
            "dispute" => Ok(TransactionType::Dispute),
            "resolve" => Ok(TransactionType::Resolve),
            "chargeback" => Ok(TransactionType::Chargeback),
            _ => Err(ParseError::InvalidTransactionType(s.to_string())),
        }
    }
}

/// Represents a single transaction in the payment system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction type
    #[serde(rename = "type")]
    pub tx_type: TransactionType,

    /// Client ID (u16)
    pub client: u16,

    /// Transaction ID (u32)
    pub tx: u32,

    /// Amount (only for deposit/withdrawal, None for dispute/resolve/chargeback)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
}

impl Transaction {
    /// Creates a new transaction.
    pub fn new(tx_type: TransactionType, client: u16, tx: u32, amount: Option<Amount>) -> Self {
        Self {
            tx_type,
            client,
            tx,
            amount,
        }
    }

    /// Creates a deposit transaction.
    pub fn deposit(client: u16, tx: u32, amount: Amount) -> Self {
        Self::new(TransactionType::Deposit, client, tx, Some(amount))
    }

    /// Creates a withdrawal transaction.
    pub fn withdrawal(client: u16, tx: u32, amount: Amount) -> Self {
        Self::new(TransactionType::Withdrawal, client, tx, Some(amount))
    }

    /// Creates a dispute transaction.
    pub fn dispute(client: u16, tx: u32) -> Self {
        Self::new(TransactionType::Dispute, client, tx, None)
    }

    /// Creates a resolve transaction.
    pub fn resolve(client: u16, tx: u32) -> Self {
        Self::new(TransactionType::Resolve, client, tx, None)
    }

    /// Creates a chargeback transaction.
    pub fn chargeback(client: u16, tx: u32) -> Self {
        Self::new(TransactionType::Chargeback, client, tx, None)
    }

    /// Validates the transaction structure.
    ///
    /// Ensures that:
    /// - Deposits and withdrawals have amounts
    /// - Disputes, resolves, and chargebacks do not have amounts
    pub fn validate(&self) -> Result<()> {
        match self.tx_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if self.amount.is_none() {
                    return Err(ParseError::MissingAmount(self.tx).into());
                }
                if let Some(amount) = self.amount {
                    if amount.is_negative() {
                        return Err(ParseError::NegativeAmount(self.tx).into());
                    }
                }
            }
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                // These transaction types should not have amounts
                // But we'll be lenient and ignore amounts if provided
            }
        }
        Ok(())
    }

    /// Returns true if this transaction type modifies account balances directly.
    pub fn is_balance_transaction(&self) -> bool {
        matches!(
            self.tx_type,
            TransactionType::Deposit | TransactionType::Withdrawal
        )
    }

    /// Returns true if this transaction type is dispute-related.
    pub fn is_dispute_transaction(&self) -> bool {
        matches!(
            self.tx_type,
            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback
        )
    }
}
