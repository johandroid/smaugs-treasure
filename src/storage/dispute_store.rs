//! Storage for dispute status tracking.

use crate::error::{ProcessingError, Result};
use crate::types::TxId;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Status of a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeStatus {
    /// Dispute is currently active (funds held)
    Active,

    /// Dispute was resolved (funds released back to available)
    Resolved,

    /// Dispute resulted in chargeback (funds returned to client, account locked)
    Chargedback,
}

/// Tracks disputes by transaction ID.
pub type DisputeStore = HashMap<TxId, DisputeStatus>;

/// Marks a transaction as disputed.
///
/// # Arguments
/// * `tx_id` - The transaction ID to mark as disputed
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err(ProcessingError::AlreadyDisputed)` if already disputed
pub fn mark_disputed(store: &mut DisputeStore, client: u16, tx_id: u32) -> Result<()> {
    if store.contains_key(&tx_id) {
        warn!(
            "Transaction {} already has a dispute status, cannot dispute again",
            tx_id
        );
        return Err(ProcessingError::AlreadyDisputed { client, tx_id }.into());
    }

    debug!("Marking transaction {} as disputed", tx_id);
    store.insert(tx_id, DisputeStatus::Active);
    Ok(())
}

/// Transitions an active dispute to a new status.
fn transition_active(
    store: &mut DisputeStore,
    client: u16,
    tx_id: u32,
    new_status: DisputeStatus,
) -> Result<()> {
    match store.get(&tx_id) {
        Some(DisputeStatus::Active) => {
            debug!("Transitioning dispute {} to {:?}", tx_id, new_status);
            store.insert(tx_id, new_status);
            Ok(())
        }
        Some(_) => {
            warn!("Dispute for transaction {} is not active", tx_id);
            Err(ProcessingError::DisputeNotActive { client, tx_id }.into())
        }
        None => {
            warn!("No dispute found for transaction {}", tx_id);
            Err(ProcessingError::DisputeNotFound { client, tx_id }.into())
        }
    }
}

/// Marks a dispute as resolved.
pub fn mark_resolved(store: &mut DisputeStore, client: u16, tx_id: u32) -> Result<()> {
    transition_active(store, client, tx_id, DisputeStatus::Resolved)
}

/// Marks a dispute as chargedback.
pub fn mark_chargedback(store: &mut DisputeStore, client: u16, tx_id: u32) -> Result<()> {
    transition_active(store, client, tx_id, DisputeStatus::Chargedback)
}

/// Returns the number of active disputes.
pub fn active_count(store: &DisputeStore) -> usize {
    store
        .values()
        .filter(|&&status| status == DisputeStatus::Active)
        .count()
}
