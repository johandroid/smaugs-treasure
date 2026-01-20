//! Storage modules for disputes.

mod dispute_store;
use crate::types::{Account, StoredDeposit};
use crate::types::{ClientId, TxId};
use std::collections::HashMap;

pub use dispute_store::{
    active_count, mark_chargedback, mark_disputed, mark_resolved, DisputeStatus, DisputeStore,
};

/// Type alias for account state management.
///
/// Maps client IDs to their account state.
pub type AccountState = HashMap<ClientId, Account>;
pub type TxStore = HashMap<TxId, StoredDeposit>;

#[cfg(test)]
mod tests;
