use crate::types::Account;
use crate::types::Amount;
use crate::types::Transaction;
use crate::types::TransactionType;
use std::str::FromStr;

// Account related tests
#[test]
fn test_new_account() {
    let account = Account::new(1);
    assert_eq!(account.client, 1);
    assert_eq!(account.available, Amount::zero());
    assert_eq!(account.held, Amount::zero());
    assert_eq!(account.total(), Amount::zero());
    assert!(!account.locked);
}

#[test]
fn test_total_calculation() {
    let mut account = Account::new(1);
    account.available = Amount::from_raw(10_000);
    account.held = Amount::from_raw(5_000);
    assert_eq!(account.total(), Amount::from_raw(15_000));
}

#[test]
fn test_has_sufficient_funds() {
    let mut account = Account::new(1);
    account.available = Amount::from_raw(10_000);

    assert!(account.has_sufficient_funds(Amount::from_raw(5_000)));
    assert!(account.has_sufficient_funds(Amount::from_raw(10_000)));
    assert!(!account.has_sufficient_funds(Amount::from_raw(15_000)));
}

#[test]
fn test_lock_account() {
    let mut account = Account::new(1);
    assert!(!account.is_locked());

    account.lock();
    assert!(account.is_locked());
}

// Amount tests

#[test]
fn test_amount_from_str() {
    assert_eq!(Amount::from_str("1.0").unwrap().as_raw(), 10_000);
    assert_eq!(Amount::from_str("1.2345").unwrap().as_raw(), 12_345);
    assert_eq!(Amount::from_str("0.0001").unwrap().as_raw(), 1);
    assert_eq!(Amount::from_str("100").unwrap().as_raw(), 1_000_000);
    assert_eq!(Amount::from_str("-5.5").unwrap().as_raw(), -55_000);
    assert_eq!(Amount::from_str("-0.5").unwrap().as_raw(), -5_000);
    assert_eq!(Amount::from_str("-0.0001").unwrap().as_raw(), -1);
    assert_eq!(Amount::from_str("+0.5").unwrap().as_raw(), 5_000);
}

#[test]
fn test_amount_display() {
    assert_eq!(Amount::from_raw(10_000).to_string(), "1.0000");
    assert_eq!(Amount::from_raw(12_345).to_string(), "1.2345");
    assert_eq!(Amount::from_raw(1).to_string(), "0.0001");
    assert_eq!(Amount::from_raw(-55_000).to_string(), "-5.5000");
}

#[test]
fn test_amount_ops_add() {
    let a = Amount::from_raw(10_000); // 1.0
    let b = Amount::from_raw(5_000); // 0.5
    let c = a.add_checked(b).unwrap();
    assert_eq!(c.as_raw(), 15_000); // 1.5
}

#[test]
fn test_amount_ops_sub() {
    let a = Amount::from_raw(10_000); // 1.0
    let b = Amount::from_raw(3_000); // 0.3
    let c = a.sub_checked(b).unwrap();
    assert_eq!(c.as_raw(), 7_000); // 0.7
}

#[test]
fn test_amount_ops_overflow() {
    let a = Amount::from_raw(i64::MAX);
    let b = Amount::from_raw(1);
    assert!(a.add_checked(b).is_err());
}

#[test]
fn test_amount_ops_underflow() {
    let a = Amount::from_raw(i64::MIN);
    let b = Amount::from_raw(1);
    assert!(a.sub_checked(b).is_err());
}

#[test]
fn test_amount_comparison() {
    let a = Amount::from_raw(10_000);
    let b = Amount::from_raw(5_000);
    assert!(a.gte(&b));
    assert!(!b.gte(&a));
}

#[test]
fn test_parse_error_too_many_decimals() {
    assert!(Amount::from_str("1.12345").is_err());
}

#[test]
fn test_parse_error_invalid_format() {
    assert!(Amount::from_str("abc").is_err());
    assert!(Amount::from_str("1.2.3").is_err());
    assert!(Amount::from_str("-").is_err());
    assert!(Amount::from_str("+").is_err());
    assert!(Amount::from_str("-.5").is_err());
}

// Transaction tests

#[test]
fn test_transaction_type_from_str() {
    assert_eq!(
        TransactionType::from_str("deposit").unwrap(),
        TransactionType::Deposit
    );
    assert_eq!(
        TransactionType::from_str("WITHDRAWAL").unwrap(),
        TransactionType::Withdrawal
    );
    assert_eq!(
        TransactionType::from_str(" dispute ").unwrap(),
        TransactionType::Dispute
    );
    assert!(TransactionType::from_str("invalid").is_err());
}

#[test]
fn test_transaction_validation_deposit() {
    let tx = Transaction::deposit(1, 1, Amount::from_raw(10_000));
    assert!(tx.validate().is_ok());

    let invalid_tx = Transaction::new(TransactionType::Deposit, 1, 1, None);
    assert!(invalid_tx.validate().is_err());
}

#[test]
fn test_transaction_validation_withdrawal() {
    let tx = Transaction::withdrawal(1, 2, Amount::from_raw(5_000));
    assert!(tx.validate().is_ok());

    let invalid_tx = Transaction::new(TransactionType::Withdrawal, 1, 2, None);
    assert!(invalid_tx.validate().is_err());
}

#[test]
fn test_transaction_validation_dispute() {
    let tx = Transaction::dispute(1, 3);
    assert!(tx.validate().is_ok());
}

#[test]
fn test_transaction_helpers() {
    let deposit = Transaction::deposit(1, 1, Amount::zero());
    assert!(deposit.is_balance_transaction());
    assert!(!deposit.is_dispute_transaction());

    let dispute = Transaction::dispute(1, 1);
    assert!(!dispute.is_balance_transaction());
    assert!(dispute.is_dispute_transaction());
}

#[test]
fn test_transaction_negative_amount() {
    let tx = Transaction::deposit(1, 1, Amount::from_raw(-10_000));
    assert!(tx.validate().is_err());
}
