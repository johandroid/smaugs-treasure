//! Storage modules for disputes.

mod dispute_store;
use crate::types::{Account, MonetaryTx};
use crate::types::{ClientId, TxId};
use std::collections::HashMap;

pub use dispute_store::{
    active_count, begin_dispute, chargeback_dispute, is_active_dispute, resolve_dispute,
    DisputeStatus, DisputeStore,
};

/// Type alias for account state management.
///
/// Maps client IDs to their account state.
pub type AccountState = HashMap<ClientId, Account>;
pub type TxStore = HashMap<TxId, MonetaryTx>;

#[cfg(test)]
mod tests;
