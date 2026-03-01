//! Storage for dispute status tracking.

use crate::types::TxId;
use std::collections::HashMap;

/// Status of a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeStatus {
    /// Dispute is currently active (funds held).
    Active,
    /// Dispute was resolved (funds released back to available).
    Resolved,
    /// Dispute resulted in chargeback (funds removed, account locked).
    Chargedback,
}

/// Tracks disputes by referenced transaction ID.
pub type DisputeStore = HashMap<TxId, DisputeStatus>;

/// Tries to mark a transaction as disputed.
///
/// Returns `true` when transition succeeds (`None -> Active`).
pub fn begin_dispute(store: &mut DisputeStore, tx_id: TxId) -> bool {
    if store.contains_key(&tx_id) {
        return false;
    }
    store.insert(tx_id, DisputeStatus::Active);
    true
}

/// Tries to resolve a dispute.
///
/// Returns `true` when transition succeeds (`Active -> Resolved`).
pub fn resolve_dispute(store: &mut DisputeStore, tx_id: TxId) -> bool {
    if !matches!(store.get(&tx_id), Some(DisputeStatus::Active)) {
        return false;
    }

    store.insert(tx_id, DisputeStatus::Resolved);
    true
}

/// Tries to chargeback a dispute.
///
/// Returns `true` when transition succeeds (`Active -> Chargedback`).
pub fn chargeback_dispute(store: &mut DisputeStore, tx_id: TxId) -> bool {
    if !matches!(store.get(&tx_id), Some(DisputeStatus::Active)) {
        return false;
    }

    store.insert(tx_id, DisputeStatus::Chargedback);
    true
}

/// Returns true when a transaction currently has an active dispute.
pub fn is_active_dispute(store: &DisputeStore, tx_id: TxId) -> bool {
    matches!(store.get(&tx_id), Some(DisputeStatus::Active))
}

/// Returns the number of active disputes.
pub fn active_count(store: &DisputeStore) -> usize {
    store
        .values()
        .filter(|&&status| status == DisputeStatus::Active)
        .count()
}
