//! Integration tests for the payment processor.

use smaugs_treasure::{csv_processor::CsvProcessor, Amount, PaymentProcessor};
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Helper function to get fixture path
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Helper function to create a temporary CSV file
fn create_temp_csv(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

/// Parses CSV output into a Vec of Accounts.
fn parse_accounts(processor: PaymentProcessor) -> Vec<smaugs_treasure::Account> {
    let mut buffer = Vec::new();
    processor.finalize_to_writer(&mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();

    output
        .lines()
        .skip(1) // Skip header
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 5 {
                Some(smaugs_treasure::Account {
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

/// Helper function to process a CSV file and return accounts
/// Esto es lo mismo que lo que hago en el main, pero aquí devuelvo las cuentas para testear
fn process_csv_file(path: PathBuf) -> Vec<smaugs_treasure::Account> {
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(&path).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    parse_accounts(processor)
}

#[test]
fn test_simple_deposits_and_withdrawals() {
    let accounts = process_csv_file(fixture_path("simple.csv"));

    assert_eq!(accounts.len(), 2);

    // Client 1: deposit 1.0 + deposit 2.0 - withdrawal 1.5 = 1.5
    let client1 = accounts.iter().find(|a| a.client == 1).unwrap();
    assert_eq!(client1.available, Amount::from_raw(15_000));
    assert_eq!(client1.held, Amount::from_raw(0));
    assert_eq!(client1.total(), Amount::from_raw(15_000));
    assert!(!client1.locked);

    // Client 2: deposit 2.0 - withdrawal 3.0 = insufficient funds, so still 2.0
    let client2 = accounts.iter().find(|a| a.client == 2).unwrap();
    assert_eq!(client2.available, Amount::from_raw(20_000));
    assert_eq!(client2.held, Amount::from_raw(0));
    assert_eq!(client2.total(), Amount::from_raw(20_000));
}

#[test]
fn test_dispute_and_resolve() {
    let accounts = process_csv_file(fixture_path("dispute.csv"));

    assert_eq!(accounts.len(), 1);

    let client1 = &accounts[0];
    // deposit 10.0 + deposit 5.0 - withdrawal 3.0 = 12.0
    // After dispute and resolve, all funds should be available
    assert_eq!(client1.available, Amount::from_raw(120_000));
    assert_eq!(client1.held, Amount::from_raw(0));
    assert_eq!(client1.total(), Amount::from_raw(120_000));
    assert!(!client1.locked);
}

#[test]
fn test_chargeback_locks_account() {
    let accounts = process_csv_file(fixture_path("chargeback.csv"));

    assert_eq!(accounts.len(), 1);

    let client1 = &accounts[0];
    // After chargeback, the account should be locked
    // deposit 10.0 + deposit 5.0 = 15.0
    // dispute 10.0 (held)
    // chargeback 10.0 (removed from held, account locked)
    // deposit 10.0 should fail because account is locked
    assert_eq!(client1.available, Amount::from_raw(50_000));
    assert_eq!(client1.held, Amount::from_raw(0));
    assert_eq!(client1.total(), Amount::from_raw(50_000));
    assert!(client1.locked);
}

#[test]
fn test_csv_output_format() {
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(&fixture_path("simple.csv")).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    // Test CSV output
    let mut buffer = Vec::new();
    processor.finalize_to_writer(&mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();

    // Check header
    assert!(output.contains("client"));
    assert!(output.contains("available"));
    assert!(output.contains("held"));
    assert!(output.contains("total"));
    assert!(output.contains("locked"));

    // Check values with 4 decimal places
    assert!(output.contains(".0000") || output.contains(".5000"));
}

#[test]
fn test_example_from_spec() {
    let accounts = process_csv_file(fixture_path("example.csv"));

    assert_eq!(accounts.len(), 2);

    let client1 = accounts.iter().find(|a| a.client == 1).unwrap();
    assert_eq!(client1.available, Amount::from_raw(15_000)); // 1.5
    assert_eq!(client1.total(), Amount::from_raw(15_000));

    let client2 = accounts.iter().find(|a| a.client == 2).unwrap();
    // Client 2 tries to withdraw 3.0 but only has 2.0, so withdrawal should fail
    assert_eq!(client2.available, Amount::from_raw(20_000)); // 2.0
    assert_eq!(client2.total(), Amount::from_raw(20_000));
}

#[test]
fn test_fixed_point_precision() {
    let csv_data = "type,client,tx,amount\ndeposit,1,1,1.2345\n";
    let file = create_temp_csv(csv_data);
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(file.path()).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    let accounts = parse_accounts(processor);
    assert_eq!(accounts[0].available, Amount::from_raw(12_345));
}

// This test is according to clarified requirement
#[test]
fn test_insufficient_funds_continues_processing() {
    let csv_data =
        "type,client,tx,amount\ndeposit,1,1,10.0\nwithdrawal,1,2,20.0\ndeposit,1,3,5.0\n";
    let file = create_temp_csv(csv_data);
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(file.path()).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    let accounts = parse_accounts(processor);
    // Should have 10.0 + 5.0 = 15.0 (withdrawal failed)
    assert_eq!(accounts[0].available, Amount::from_raw(150_000));
}

#[test]
fn test_multiple_disputes_same_transaction() {
    let csv_data = "type,client,tx,amount
deposit,1,1,10.0
dispute,1,1,
dispute,1,1,
";
    let file = create_temp_csv(csv_data);
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(file.path()).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    let accounts = parse_accounts(processor);
    // Second dispute should fail, only first one should be applied
    assert_eq!(accounts[0].available, Amount::from_raw(0));
    assert_eq!(accounts[0].held, Amount::from_raw(100_000));
}

#[test]
fn test_chargeback_without_resolve() {
    let csv_data = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
chargeback,1,1,
";
    let file = create_temp_csv(csv_data);
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(file.path()).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    let accounts = parse_accounts(processor);

    // Chargeback without resolve should work
    assert_eq!(accounts[0].available, Amount::from_raw(0));
    assert_eq!(accounts[0].held, Amount::from_raw(0));
    assert_eq!(accounts[0].total(), Amount::from_raw(0));
    assert!(accounts[0].locked);
}

#[test]
fn test_chargeback_without_dispute_fails() {
    let csv_data = "type,client,tx,amount
deposit,1,1,100.0
chargeback,1,1,
";
    let file = create_temp_csv(csv_data);
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(file.path()).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    let accounts = parse_accounts(processor);

    // Chargeback without dispute should fail, funds remain available
    assert_eq!(accounts[0].available, Amount::from_raw(1_000_000)); // 100.0
    assert_eq!(accounts[0].held, Amount::from_raw(0));
    assert!(!accounts[0].locked);
}

#[test]
fn test_chargeback_after_resolve_fails() {
    let csv_data = "type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
resolve,1,1,
chargeback,1,1,
";
    let file = create_temp_csv(csv_data);
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(file.path()).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    let accounts = parse_accounts(processor);

    // Chargeback after resolve should fail
    assert_eq!(accounts[0].available, Amount::from_raw(1_000_000)); // 100.0
    assert_eq!(accounts[0].held, Amount::from_raw(0));
    assert!(!accounts[0].locked);
}

#[test]
fn test_dispute_withdrawal_ignored() {
    let csv_data = "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,50.0
dispute,1,2,
";
    let file = create_temp_csv(csv_data);
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(file.path()).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    let accounts = parse_accounts(processor);

    // Dispute on withdrawal (tx 2) is ignored (only deposits can be disputed)
    // Account should have 50.0 available (100.0 - 50.0 withdrawal)
    assert_eq!(accounts[0].available, Amount::from_raw(500_000)); // 50.0
    assert_eq!(accounts[0].held, Amount::from_raw(0)); // No funds held
    assert!(!accounts[0].locked);
}

#[test]
fn test_dispute_after_withdrawal_allows_negative() {
    let csv_data = "type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,80.0
dispute,1,1,
";
    let file = create_temp_csv(csv_data);
    let mut processor = PaymentProcessor::new();
    let mut csv_processor = CsvProcessor::from_path(file.path()).unwrap();

    csv_processor
        .process_stream(|result| {
            if let Ok(tx) = result {
                let _ = processor.process_transaction(tx);
            }
        })
        .unwrap();

    let accounts = parse_accounts(processor);

    // After deposit 100.0, withdrawal 80.0, available is 20.0
    // Then dispute on tx 1 (100.0) should hold 100.0
    // This makes available = 20.0 - 100.0 = -80.0
    assert_eq!(accounts[0].available, Amount::from_raw(-800_000)); // -80.0
    assert_eq!(accounts[0].held, Amount::from_raw(1_000_000)); // 100.0
    assert_eq!(accounts[0].total(), Amount::from_raw(200_000)); // 20.0 total
    assert!(!accounts[0].locked);
}
