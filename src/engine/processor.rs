//! Core payment processing engine.

use crate::error::{ParseError, ProcessingError, Result};
use crate::storage::{
    begin_dispute, chargeback_dispute, resolve_dispute, AccountState, DisputeStore, TxStore,
};
use crate::types::{
    Account, Amount, MonetaryTx, MonetaryTxKind, Transaction, TransactionType, TxId,
};
use std::collections::HashSet;
use tracing::{debug, info, warn};

/// Main payment processor that handles all transaction types.
pub struct PaymentProcessor {
    state: AccountState,
    tx_store: TxStore,
    dispute_store: DisputeStore,
    seen_monetary_txs: HashSet<TxId>,
}

/// Non-fatal processing outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingOutcome {
    Applied,
    Ignored(IgnoredReason),
}

/// Reasons for non-fatal ignored rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoredReason {
    UnknownReference { tx_id: TxId },
    NonDepositReference { tx_id: TxId },
    DisputeAlreadyFinalized { tx_id: TxId },
    DisputeNotActive { tx_id: TxId },
}

impl Default for PaymentProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentProcessor {
    /// Creates a new payment processor.
    pub fn new() -> Self {
        info!("Initializing payment processor");
        Self {
            state: AccountState::new(),
            tx_store: TxStore::new(),
            dispute_store: DisputeStore::new(),
            seen_monetary_txs: HashSet::new(),
        }
    }

    /// Gets or creates an account for a client.
    fn get_or_create_account(&mut self, client: u16) -> &mut Account {
        self.state
            .entry(client)
            .or_insert_with(|| Account::new(client))
    }

    fn ensure_account_not_locked(&self, client: u16) -> Result<()> {
        if self.state.get(&client).is_some_and(Account::is_locked) {
            return Err(ProcessingError::AccountLocked { client }.into());
        }
        Ok(())
    }

    /// Extracts the required amount from a validated transaction.
    fn validated_amount(tx: &Transaction) -> Result<Amount> {
        tx.amount
            .ok_or_else(|| ParseError::MissingAmount(tx.tx).into())
    }

    /// Reserves a monetary transaction ID, enforcing global uniqueness.
    fn reserve_monetary_tx_id(&mut self, tx_id: TxId) -> Result<()> {
        if !self.seen_monetary_txs.insert(tx_id) {
            return Err(ProcessingError::DuplicateTransaction { tx_id }.into());
        }
        Ok(())
    }

    fn get_referenced_tx(&self, tx_id: TxId) -> Option<MonetaryTx> {
        self.tx_store.get(&tx_id).copied()
    }

    /// Handles a deposit transaction.
    fn handle_deposit(&mut self, tx: Transaction) -> Result<ProcessingOutcome> {
        let (client, tx_id) = (tx.client, tx.tx);
        let amount = Self::validated_amount(&tx)?;
        debug!(
            "Processing deposit: client={}, tx={}, amount={}",
            client, tx_id, amount
        );

        self.reserve_monetary_tx_id(tx_id)?;
        self.ensure_account_not_locked(client)?;

        let account = self.get_or_create_account(client);
        account.available = account.available.add_checked(amount)?;

        self.tx_store.insert(
            tx_id,
            MonetaryTx {
                client_id: client,
                amount,
                kind: MonetaryTxKind::Deposit,
            },
        );

        info!(
            "Applied deposit: client={}, tx={}, amount={}",
            client, tx_id, amount
        );
        Ok(ProcessingOutcome::Applied)
    }

    /// Handles a withdrawal transaction.
    fn handle_withdrawal(&mut self, tx: Transaction) -> Result<ProcessingOutcome> {
        let (client, tx_id) = (tx.client, tx.tx);
        let amount = Self::validated_amount(&tx)?;
        debug!(
            "Processing withdrawal: client={}, tx={}, amount={}",
            client, tx_id, amount
        );

        self.reserve_monetary_tx_id(tx_id)?;
        self.ensure_account_not_locked(client)?;

        let account = self.get_or_create_account(client);
        if !account.has_sufficient_funds(amount) {
            return Err(
                ProcessingError::insufficient_funds(client, account.available, amount).into(),
            );
        }

        account.available = account.available.sub_checked(amount)?;
        self.tx_store.insert(
            tx_id,
            MonetaryTx {
                client_id: client,
                amount,
                kind: MonetaryTxKind::Withdrawal,
            },
        );

        info!(
            "Applied withdrawal: client={}, tx={}, amount={}",
            client, tx_id, amount
        );
        Ok(ProcessingOutcome::Applied)
    }

    /// Handles a dispute transaction.
    fn handle_dispute(&mut self, tx: Transaction) -> Result<ProcessingOutcome> {
        let tx_id = tx.tx;
        debug!(
            "Processing dispute request: input_client={}, ref_tx={}",
            tx.client, tx_id
        );

        let Some(referenced_tx) = self.get_referenced_tx(tx_id) else {
            warn!("Ignored dispute: referenced tx {} not found", tx_id);
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::UnknownReference { tx_id },
            ));
        };

        if !referenced_tx.is_deposit() {
            warn!("Ignored dispute: referenced tx {} is not a deposit", tx_id);
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::NonDepositReference { tx_id },
            ));
        }

        self.ensure_account_not_locked(referenced_tx.client_id)?;
        if !begin_dispute(&mut self.dispute_store, tx_id) {
            warn!(
                "Ignored dispute: transaction {} has already been disputed/finalized",
                tx_id
            );
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::DisputeAlreadyFinalized { tx_id },
            ));
        }

        let account = self.get_or_create_account(referenced_tx.client_id);
        account.available = account.available.sub_checked(referenced_tx.amount)?;
        account.held = account.held.add_checked(referenced_tx.amount)?;

        info!(
            "Applied dispute: owner_client={}, ref_tx={}, amount={}",
            referenced_tx.client_id, tx_id, referenced_tx.amount
        );
        Ok(ProcessingOutcome::Applied)
    }

    /// Handles a resolve transaction.
    fn handle_resolve(&mut self, tx: Transaction) -> Result<ProcessingOutcome> {
        let tx_id = tx.tx;
        debug!(
            "Processing resolve request: input_client={}, ref_tx={}",
            tx.client, tx_id
        );

        let Some(referenced_tx) = self.get_referenced_tx(tx_id) else {
            warn!("Ignored resolve: referenced tx {} not found", tx_id);
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::UnknownReference { tx_id },
            ));
        };

        if !referenced_tx.is_deposit() {
            warn!("Ignored resolve: referenced tx {} is not a deposit", tx_id);
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::NonDepositReference { tx_id },
            ));
        }

        self.ensure_account_not_locked(referenced_tx.client_id)?;
        if !resolve_dispute(&mut self.dispute_store, tx_id) {
            warn!("Ignored resolve: dispute is not active for tx {}", tx_id);
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::DisputeNotActive { tx_id },
            ));
        }

        let account = self.get_or_create_account(referenced_tx.client_id);
        account.held = account.held.sub_checked(referenced_tx.amount)?;
        account.available = account.available.add_checked(referenced_tx.amount)?;

        info!(
            "Applied resolve: owner_client={}, ref_tx={}, amount={}",
            referenced_tx.client_id, tx_id, referenced_tx.amount
        );
        Ok(ProcessingOutcome::Applied)
    }

    /// Handles a chargeback transaction.
    fn handle_chargeback(&mut self, tx: Transaction) -> Result<ProcessingOutcome> {
        let tx_id = tx.tx;
        debug!(
            "Processing chargeback request: input_client={}, ref_tx={}",
            tx.client, tx_id
        );

        let Some(referenced_tx) = self.get_referenced_tx(tx_id) else {
            warn!("Ignored chargeback: referenced tx {} not found", tx_id);
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::UnknownReference { tx_id },
            ));
        };

        if !referenced_tx.is_deposit() {
            warn!(
                "Ignored chargeback: referenced tx {} is not a deposit",
                tx_id
            );
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::NonDepositReference { tx_id },
            ));
        }

        self.ensure_account_not_locked(referenced_tx.client_id)?;
        if !chargeback_dispute(&mut self.dispute_store, tx_id) {
            warn!("Ignored chargeback: dispute is not active for tx {}", tx_id);
            return Ok(ProcessingOutcome::Ignored(
                IgnoredReason::DisputeNotActive { tx_id },
            ));
        }

        let account = self.get_or_create_account(referenced_tx.client_id);
        account.held = account.held.sub_checked(referenced_tx.amount)?;
        account.lock();

        info!(
            "Applied chargeback: owner_client={}, ref_tx={}, amount={}",
            referenced_tx.client_id, tx_id, referenced_tx.amount
        );
        Ok(ProcessingOutcome::Applied)
    }

    /// Processes a single transaction.
    pub fn process_transaction(&mut self, tx: Transaction) -> Result<ProcessingOutcome> {
        tx.validate()?;

        match tx.tx_type {
            TransactionType::Deposit => self.handle_deposit(tx),
            TransactionType::Withdrawal => self.handle_withdrawal(tx),
            TransactionType::Dispute => self.handle_dispute(tx),
            TransactionType::Resolve => self.handle_resolve(tx),
            TransactionType::Chargeback => self.handle_chargeback(tx),
        }
    }

    /// Finalizes and prints accounts as CSV to stdout.
    pub fn finalize_to_csv(self) -> std::io::Result<()> {
        let mut stdout = std::io::stdout().lock();
        self.finalize_to_writer(&mut stdout)
    }

    /// Finalizes and writes accounts as CSV to the given writer.
    pub fn finalize_to_writer<W: std::io::Write>(self, writer: &mut W) -> std::io::Result<()> {
        info!("Finalizing payment processor");

        writeln!(writer, "client,available,held,total,locked")?;

        let mut accounts: Vec<_> = self.state.into_values().collect();
        accounts.sort_unstable_by_key(|account| account.client);

        for account in accounts {
            writeln!(
                writer,
                "{},{},{},{},{}",
                account.client,
                account.available,
                account.held,
                account.total(),
                account.locked
            )?;
        }

        Ok(())
    }
}
