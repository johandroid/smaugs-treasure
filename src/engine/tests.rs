use crate::types::Amount;

use crate::engine::PaymentProcessor;
use crate::types::Transaction;

#[test]
fn test_deposit() {
    let mut processor = PaymentProcessor::new();
    let tx = Transaction::deposit(1, 1, Amount::from_raw(10_000));

    processor.process_transaction(tx).unwrap();

    let accounts = processor.finalize();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].available, Amount::from_raw(10_000));
    assert_eq!(accounts[0].total(), Amount::from_raw(10_000));
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

    let accounts = processor.finalize();
    assert_eq!(accounts[0].available, Amount::from_raw(7_000));
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

    let accounts = processor.finalize();
    assert_eq!(accounts[0].available, Amount::zero());
    assert_eq!(accounts[0].held, Amount::from_raw(10_000));
    assert_eq!(accounts[0].total(), Amount::from_raw(10_000));
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

    let accounts = processor.finalize();
    assert_eq!(accounts[0].available, Amount::from_raw(10_000));
    assert_eq!(accounts[0].held, Amount::zero());
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

    let accounts = processor.finalize();
    assert!(accounts[0].is_locked());
    assert_eq!(accounts[0].total(), Amount::zero());
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

    let accounts = processor.finalize();
    assert_eq!(accounts.len(), 2);
}

#[test]
fn test_dispute_unknown_transaction_ignored() {
    let mut processor = PaymentProcessor::new();

    // Dispute a non-existent transaction - should be silently ignored
    let result = processor.process_transaction(Transaction::dispute(1, 999));
    assert!(result.is_ok());
}

#[test]
fn test_client_mismatch() {
    let mut processor = PaymentProcessor::new();

    // Client 1 deposits
    processor
        .process_transaction(Transaction::deposit(1, 1, Amount::from_raw(10_000)))
        .unwrap();

    // Client 2 tries to dispute client 1's deposit
    let result = processor.process_transaction(Transaction::dispute(2, 1));
    assert!(result.is_err());
}
