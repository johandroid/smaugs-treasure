//! Account types and state management.

use crate::types::Amount;
use serde::{Deserialize, Serialize};
use tracing::error;

/// Represents a client account with available, held, and locked status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// Client ID
    pub client: u16,

    /// Available funds
    pub available: Amount,

    /// Held funds (disputed transactions)
    pub held: Amount,

    /// Whether the account is locked (due to chargeback)
    pub locked: bool,
}

impl Account {
    /// Creates a new account with zero balances.
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: Amount::zero(),
            held: Amount::zero(),
            locked: false,
        }
    }

    /// Calculates the total funds (available + held).
    pub fn total(&self) -> Amount {
        match self.available.add_checked(self.held) {
            Ok(total) => total,
            Err(e) => {
                error!(
                    "Account invariant error for client {} while calculating total: {}",
                    self.client, e
                );
                Amount::zero()
            }
        }
    }

    /// Checks if the account has sufficient available funds.
    pub fn has_sufficient_funds(&self, amount: Amount) -> bool {
        self.available.gte(&amount)
    }

    /// Returns true if the account is locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Locks the account (typically due to chargeback).
    pub fn lock(&mut self) {
        self.locked = true;
    }
}
