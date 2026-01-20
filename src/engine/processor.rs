//! Core payment processing engine.

use crate::error::{ProcessingError, Result};
use crate::storage::{active_count, mark_chargedback, mark_disputed, mark_resolved, DisputeStore};
use crate::storage::{AccountState, TxStore};
use crate::types::{Account, StoredDeposit, Transaction, TransactionType, TxId};
use tracing::{debug, error, info, warn};

/// Main payment processor that handles all transaction types.
pub struct PaymentProcessor {
    state: AccountState,
    tx_store: TxStore,
    dispute_store: DisputeStore,
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
        }
    }

    /// Checks if the account is locked and rejects the transaction if needed.
    fn check_account_not_locked(&self, tx: &Transaction) -> Result<()> {
        if let Some(account) = self.state.get(&tx.client) {
            if account.is_locked() {
                warn!(
                    "Account {} is locked, rejecting {:?} transaction",
                    tx.client, tx.tx_type
                );
                return Err(ProcessingError::AccountLocked { client: tx.client }.into());
            }
        }
        Ok(())
    }

    /// Gets a deposit and verifies the client owns it.
    fn get_verified_deposit(&self, client: u16, tx_id: TxId) -> Result<&StoredDeposit> {
        let deposit = self
            .tx_store
            .get(&tx_id)
            .ok_or(ProcessingError::TransactionNotFound { client, tx_id })?;

        if deposit.client_id != client {
            return Err(ProcessingError::ClientMismatch {
                tx_id,
                owner: deposit.client_id,
                requester: client,
            }
            .into());
        }

        Ok(deposit)
    }

    /// Handles a deposit transaction.
    fn handle_deposit(&mut self, tx: Transaction) -> Result<()> {
        debug!(
            "Processing deposit: client={}, tx={}, amount={:?}",
            tx.client, tx.tx, tx.amount
        );

        let amount = tx.amount.ok_or(ProcessingError::TransactionNotFound {
            client: tx.client,
            tx_id: tx.tx,
        })?;

        let account = self.state.entry(tx.client).or_insert_with(|| {
            debug!("Creating new account for client {}", tx.client);
            Account::new(tx.client)
        });

        account.available = account.available.add_checked(amount)?;

        let tx_id = tx.tx;
        let client_id = tx.client;
        if self.tx_store.contains_key(&tx_id) {
            warn!("Duplicate transaction ID detected: {}", tx_id);
            return Err(ProcessingError::DuplicateTransaction { tx_id }.into());
        }
        self.tx_store
            .insert(tx_id, StoredDeposit { client_id, amount });

        info!(
            "Deposit processed: client={}, tx={}, amount={}",
            client_id, tx_id, amount
        );

        Ok(())
    }

    /// Handles a withdrawal transaction.
    fn handle_withdrawal(&mut self, tx: Transaction) -> Result<()> {
        debug!(
            "Processing withdrawal: client={}, tx={}, amount={:?}",
            tx.client, tx.tx, tx.amount
        );

        let amount = tx.amount.ok_or(ProcessingError::TransactionNotFound {
            client: tx.client,
            tx_id: tx.tx,
        })?;

        let account = self.state.entry(tx.client).or_insert_with(|| {
            debug!("Creating new account for client {}", tx.client);
            Account::new(tx.client)
        });

        if !account.has_sufficient_funds(amount) {
            warn!(
                "Insufficient funds for withdrawal: client={}, available={}, requested={}",
                tx.client, account.available, amount
            );
            return Err(
                ProcessingError::insufficient_funds(tx.client, account.available, amount).into(),
            );
        }

        account.available = account.available.sub_checked(amount)?;

        info!(
            "Withdrawal processed: client={}, tx={}, amount={}",
            tx.client, tx.tx, amount
        );

        Ok(())
    }

    /// Handles a dispute transaction.
    ///
    /// Only deposits can be disputed. If the transaction doesn't exist
    /// in the deposit store, the dispute is silently ignored.
    fn handle_dispute(&mut self, tx: Transaction) -> Result<()> {
        let (client, tx_id) = (tx.client, tx.tx);
        debug!("Processing dispute: client={}, tx={}", client, tx_id);

        let Some(deposit) = self.tx_store.get(&tx_id) else {
            warn!("Transaction {} not found, ignoring dispute", tx_id);
            return Ok(());
        };

        if deposit.client_id != client {
            return Err(ProcessingError::ClientMismatch {
                tx_id,
                owner: deposit.client_id,
                requester: client,
            }
            .into());
        }

        let amount = deposit.amount;
        mark_disputed(&mut self.dispute_store, client, tx_id)?;

        let account = self
            .state
            .entry(client)
            .or_insert_with(|| Account::new(client));
        account.available = account.available.sub_checked(amount)?;
        account.held = account.held.add_checked(amount)?;

        info!(
            "Disputed tx {}: held {} for client {}",
            tx_id, amount, client
        );
        Ok(())
    }

    /// Handles a resolve transaction.
    fn handle_resolve(&mut self, tx: Transaction) -> Result<()> {
        let (client, tx_id) = (tx.client, tx.tx);
        debug!("Processing resolve: client={}, tx={}", client, tx_id);

        let amount = self.get_verified_deposit(client, tx_id)?.amount;
        mark_resolved(&mut self.dispute_store, client, tx_id)?;

        let account = self
            .state
            .entry(client)
            .or_insert_with(|| Account::new(client));
        account.held = account.held.sub_checked(amount)?;
        account.available = account.available.add_checked(amount)?;

        info!(
            "Resolved tx {}: released {} for client {}",
            tx_id, amount, client
        );
        Ok(())
    }

    /// Handles a chargeback transaction.
    fn handle_chargeback(&mut self, tx: Transaction) -> Result<()> {
        let (client, tx_id) = (tx.client, tx.tx);
        debug!("Processing chargeback: client={}, tx={}", client, tx_id);

        let amount = self.get_verified_deposit(client, tx_id)?.amount;
        mark_chargedback(&mut self.dispute_store, client, tx_id)?;

        let account = self
            .state
            .entry(client)
            .or_insert_with(|| Account::new(client));
        account.held = account.held.sub_checked(amount)?;
        account.lock();

        info!(
            "Chargedback tx {}: removed {} held, locked client {}",
            tx_id, amount, client
        );
        Ok(())
    }

    /// Returns statistics about the processor state.
    pub fn stats(&self) -> ProcessorStats {
        ProcessorStats {
            accounts: self.state.len(),
            transactions: self.tx_store.len(),
            active_disputes: active_count(&self.dispute_store),
        }
    }

    /// Processes a single transaction.
    pub fn process_transaction(&mut self, tx: Transaction) -> Result<()> {
        tx.validate()?;
        self.check_account_not_locked(&tx)?;

        let result = match tx.tx_type {
            TransactionType::Deposit => self.handle_deposit(tx),
            TransactionType::Withdrawal => self.handle_withdrawal(tx),
            TransactionType::Dispute => self.handle_dispute(tx),
            TransactionType::Resolve => self.handle_resolve(tx),
            TransactionType::Chargeback => self.handle_chargeback(tx),
        };

        if let Err(ref e) = result {
            error!("Transaction processing error: {}", e);
        }

        result
    }

    /// Finalizes processing and returns the final account state.
    pub fn finalize(self) -> Vec<Account> {
        info!("Finalizing payment processor");
        let stats = self.stats();
        info!(
            "Final stats: {} accounts, {} transactions, {} active disputes",
            stats.accounts, stats.transactions, stats.active_disputes
        );

        let mut accounts: Vec<Account> = self.state.into_values().collect();
        accounts.sort_by_key(|a| a.client);
        accounts
    }

    /// Finalizes and prints accounts as CSV to stdout.
    pub fn finalize_to_csv(self) {
        let mut stdout = std::io::stdout().lock();
        self.finalize_to_writer(&mut stdout).unwrap();
    }

    /// Finalizes and writes accounts as CSV to the given writer.
    pub fn finalize_to_writer<W: std::io::Write>(self, writer: &mut W) -> std::io::Result<()> {
        let accounts = self.finalize();
        writeln!(writer, "client,available,held,total,locked")?;
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

/// Statistics about the processor state.
#[derive(Debug, Clone, Copy)]
pub struct ProcessorStats {
    pub accounts: usize,
    pub transactions: usize,
    pub active_disputes: usize,
}
