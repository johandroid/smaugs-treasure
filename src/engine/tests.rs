use crate::engine::PaymentProcessor;
use crate::types::{Account, Amount, Transaction};

/// Parses CSV output into a Vec of Accounts.
fn parse_accounts(processor: PaymentProcessor) -> Vec<Account> {
    let mut buffer = Vec::new();
    processor.finalize_to_writer(&mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();

    output
        .lines()
        .skip(1) // Skip header
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 5 {
                Some(Account {
                    client: parts[0].parse().unwrap(),
                    available: parts[1].parse().unwrap(),
                    held: parts[2].parse().unwrap(),
                    locked: parts[4].parse().unwrap(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn find_account(accounts: &[Account], client: u16) -> &Account {
    accounts.iter().find(|a| a.client == client).unwrap()
}

#[test]
fn test_deposit() {
    let mut processor = PaymentProcessor::new();
    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();

    let accounts = parse_accounts(processor);
    assert_eq!(accounts.len(), 1);
    let account = find_account(&accounts, 1);
    assert_eq!(account.available, Amount::from_raw(10_000));
    assert_eq!(account.total(), Amount::from_raw(10_000));
}

#[test]
fn test_withdrawal() {
    let mut processor = PaymentProcessor::new();

    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();
    processor
        .process_transaction(Transaction::withdrawal(1, 2, Amount::from_raw(3_000)))
        .unwrap();

    let accounts = parse_accounts(processor);
    let account = find_account(&accounts, 1);
    assert_eq!(account.available, Amount::from_raw(7_000));
}

#[test]
fn test_insufficient_funds() {
    let mut processor = PaymentProcessor::new();

    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(5_000)))
        .unwrap();

    let result =
        processor.process_transaction(Transaction::withdrawal(1, 2, Amount::from_raw(10_000)));
    assert!(result.is_err());
}

#[test]
fn test_dispute_flow() {
    let mut processor = PaymentProcessor::new();

    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();
    processor
        .process_transaction(Transaction::dispute(1, 1))
        .unwrap();

    let accounts = parse_accounts(processor);
    let account = find_account(&accounts, 1);
    assert_eq!(account.available, Amount::zero());
    assert_eq!(account.held, Amount::from_raw(10_000));
    assert_eq!(account.total(), Amount::from_raw(10_000));
}

#[test]
fn test_resolve_flow() {
    let mut processor = PaymentProcessor::new();

    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();
    processor
        .process_transaction(Transaction::dispute(1, 1))
        .unwrap();
    processor
        .process_transaction(Transaction::resolve(1, 1))
        .unwrap();

    let accounts = parse_accounts(processor);
    let account = find_account(&accounts, 1);
    assert_eq!(account.available, Amount::from_raw(10_000));
    assert_eq!(account.held, Amount::zero());
}

#[test]
fn test_chargeback_locks_account() {
    let mut processor = PaymentProcessor::new();

    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();
    processor
        .process_transaction(Transaction::dispute(1, 1))
        .unwrap();
    processor
        .process_transaction(Transaction::chargeback(1, 1))
        .unwrap();

    let accounts = parse_accounts(processor);
    let account = find_account(&accounts, 1);
    assert!(account.locked);
    assert_eq!(account.total(), Amount::zero());
}

#[test]
fn test_locked_account_rejects_transactions() {
    let mut processor = PaymentProcessor::new();

    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();
    processor
        .process_transaction(Transaction::dispute(1, 1))
        .unwrap();
    processor
        .process_transaction(Transaction::chargeback(1, 1))
        .unwrap();

    let result = processor.process_transaction(Transaction::deposit(1, 2, Amount::from_raw(5_000)));
    assert!(result.is_err());
}

#[test]
fn test_multiple_clients() {
    let mut processor = PaymentProcessor::new();

    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();
    processor
        .process_transaction(Transaction::deposit(2, 2, Amount::from_raw(20_000)))
        .unwrap();

    let accounts = parse_accounts(processor);
    assert_eq!(accounts.len(), 2);
}

#[test]
fn test_dispute_unknown_transaction_ignored() {
    let mut processor = PaymentProcessor::new();

    let result = processor.process_transaction(Transaction::dispute(1, 999));
    assert!(result.is_ok());
}

#[test]
fn test_client_mismatch() {
    let mut processor = PaymentProcessor::new();

    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();

    let result = processor.process_transaction(Transaction::dispute(2, 1));
    assert!(result.is_err());
}
