# Smaug's Treasure - Payment Processing Engine

<p align="center">
  <img src="smaug-co.png" alt="Smaug's Treasure" width="400"/>
</p>


A robust, type-safe payment processing engine written in Rust that handles deposits, withdrawals, and dispute resolution with fixed-point arithmetic precision.

## Features

- **Safe Fixed-Point Arithmetic**: Uses i64 internally with 4 decimal precision (10_000 = 1.0000)
- **Streaming CSV Processing**: Memory-efficient line-by-line processing for large files
- **Comprehensive Error Handling**: Uses `thiserror` and `anyhow` for rich error context
- **Structured Logging**: Built-in tracing support with `tracing` crate
- **Type-Safe Operations**: Trait-based design prevents misuse of the API
- **Full Test Coverage**: Extensive unit and integration tests

## Architecture

The project is structured as a single crate with both library and binary:

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Public API and re-exports
├── cli.rs               # Command-line argument parsing
├── csv_processor.rs     # Streaming CSV reader
├── error.rs             # Error types (PaymentError, ProcessingError, etc.)
├── types/               # Core data types
│   ├── mod.rs           # Type aliases (TxId, ClientId) and StoredDeposit
│   ├── amount.rs        # Fixed-point arithmetic (Amount type)
│   ├── transaction.rs   # Transaction and TransactionType
│   └── account.rs       # Account state management
├── engine/              # Payment processing engine
│   ├── mod.rs           # Module exports
│   └── processor.rs     # PaymentProcessor (deposits, withdrawals, disputes)
└── storage/             # In-memory storage and dispute tracking
    ├── mod.rs           # Type aliases (AccountState, TxStore, DisputeStore)
    └── dispute_store.rs # DisputeStatus enum and state machine functions
```

## Storage

The processor uses three in-memory HashMap-based stores:

### AccountState (`HashMap<ClientId, Account>`)
Maps client IDs to their account state. Each `Account` contains:
- `available`: Funds available for withdrawal
- `held`: Funds held due to active disputes
- `locked`: Whether the account is frozen (after chargeback)

### TxStore (`HashMap<TxId, StoredDeposit>`)
Stores deposit transactions for potential dispute resolution. Each `StoredDeposit` contains:
- `client_id`: The client who made the deposit
- `amount`: The deposit amount

Only deposits are stored since withdrawals cannot be disputed (funds already left the account).

### DisputeStore (`HashMap<TxId, DisputeStatus>`)
Tracks the status of disputes with a simple state machine:
- `Active`: Dispute is ongoing, funds are held
- `Resolved`: Dispute was resolved, funds released back to available
- `Chargedback`: Dispute resulted in chargeback, funds removed and account locked

Valid transitions: `None → Active → Resolved` or `None → Active → Chargedback`. Once resolved or chargedback, a transaction cannot be disputed again.

## Building

```bash
cargo build --release
```

## Usage

### Basic Usage

```bash
# Process transactions from a CSV file
cargo run -- transactions.csv

# Or using the compiled binary
./target/release/smaugs-treasure transactions.csv
```

### Command Line Options

```bash
# Enable verbose logging (outputs to stderr)
cargo run -- transactions.csv --verbose
```

### Verbose Logging

Enable detailed logging to stderr with the `--verbose` flag:

```bash
cargo run -- transactions.csv --verbose
```

When verbose mode is enabled, the processor logs:
- Each transaction as it's processed (deposits, withdrawals, disputes, etc.)
- Transaction errors (insufficient funds, duplicate IDs, etc.)
- Final statistics: number of accounts, transactions processed, and active disputes

### Hiring Information ;)

```bash
cargo run -- --hire
```

## CSV Format

### Input Format

The input CSV must have the following columns:
- `type`: Transaction type (deposit, withdrawal, dispute, resolve, chargeback)
- `client`: Client ID (u16)
- `tx`: Transaction ID (u32)
- `amount`: Amount with up to 4 decimal places (only for deposit/withdrawal)

Example:
```csv
type,client,tx,amount
deposit,1,1,10.0
deposit,2,2,20.0
withdrawal,1,3,5.0
dispute,1,1,
resolve,1,1,
```

### Output Format

The program outputs account states to stdout:
```csv
client,available,held,total,locked
1,5.0000,0.0000,5.0000,false
2,20.0000,0.0000,20.0000,false
```

## Transaction Types

### Deposit
Credits the client's available balance with the specified amount.

### Withdrawal
Debits the client's available balance by the specified amount. Fails if insufficient funds.

### Dispute
Places a hold on funds from a deposit. Moves funds from available to held.

### Resolve
Releases held funds from a dispute back to available balance.

### Chargeback
Reverses a disputed deposit, removes held funds, and locks the account permanently.

## Assumptions and Design Decisions

Based on typical payment processing logic and interpretation of the specification:

### 1. Only Deposits Can Be Disputed
**Rationale**: Withdrawals represent funds that have already left the account. There are no funds to "hold" for a dispute. The specification mentions "a client's claim that a transaction was erroneous and should be reversed" - reversing a withdrawal doesn't make sense as the funds are already gone.

**Behavior**: Only deposits are stored for potential disputes. Attempting to dispute a withdrawal or non-existent transaction is silently ignored.

### 2. Chargebacks Can Be Applied Directly to Active Disputes
**Rationale**: The specification states a chargeback is "the final state of a dispute", not "the final state after a resolve". This allows two valid dispute resolution paths:
- `Dispute → Resolve`
- `Dispute → Chargeback`

**Behavior**: A resolve is not required before a chargeback. However, once a dispute is resolved or chargedback, it cannot be disputed again.

### 3. Transaction IDs Are Globally Unique
**Rationale**: The specification states "transaction IDs (tx) are globally unique". This means a transaction ID cannot be reused across different clients.

**Behavior**: Attempting to create a transaction with a duplicate ID will result in a `DuplicateTransaction` error.

### 4. Locked Accounts Remain Locked Permanently
**Rationale**: The specification states "If a chargeback occurs the client's account should be immediately frozen" without mentioning any unlock mechanism.

**Behavior**: Once an account is locked due to a chargeback, all subsequent transactions for that client will fail with an `AccountLocked` error.

### 5. Errors Do Not Stop Processing
**Rationale**: To maximize throughput and provide the most complete output possible, individual transaction errors should not halt the entire processing pipeline.

**Behavior**: When a transaction fails (e.g., insufficient funds, invalid dispute), the error is logged (if `--verbose` is enabled) and processing continues with the next transaction.

## Error Handling

The processor continues processing even when individual transactions fail. Errors are logged to stderr when `--verbose` is enabled.

Common errors:
- **Insufficient funds**: Withdrawal amount exceeds available balance
- **Transaction not found**: Resolve/chargeback references a non-existent deposit
- **Account locked**: Transaction attempted on a locked account
- **Duplicate transaction**: Transaction ID already exists
- **Already disputed**: Transaction has already been disputed
- **Dispute not active**: Attempting resolve/chargeback on a non-active dispute
- **Client mismatch**: Client attempting to dispute another client's deposit
- **Amount overflow/underflow**: Arithmetic operation would overflow i64

## Testing

### Run All Tests

```bash
cargo test
```

### Run Integration Tests Only

```bash
cargo test --test integration_test
```

### Run Specific Test

```bash
cargo test test_dispute_and_resolve
```

### Test with Example Data

```bash
cargo run -- tests/fixtures/example.csv
```

### Stress Testing with Generated Data

A Python script is included to generate large CSV files for stress testing:

```bash
# Generate 1 million transactions for 500 clients (100% random)
python3 scripts/generate_transactions.py -o stress_test.csv

# Custom parameters
python3 scripts/generate_transactions.py -n 500000 -c 100 -o test.csv

# With warmup: first 80% are only deposits/withdrawals
python3 scripts/generate_transactions.py -n 1000000 -c 500 -w 80 -o warmup.csv

# Reproducible output with seed
python3 scripts/generate_transactions.py -s 42 -o reproducible.csv
```

**Options:**
| Flag | Description | Default |
|------|-------------|---------|
| `-n, --transactions` | Total number of transactions | 1,000,000 |
| `-c, --clients` | Number of unique clients | 500 |
| `-w, --warmup` | Percentage of initial transactions that are only deposits/withdrawals | 0 |
| `-o, --output` | Output file (stdout if not specified) | - |
| `-s, --seed` | Random seed for reproducibility | - |

**Run stress test:**
```bash
python3 scripts/generate_transactions.py -n 1000000 -c 500 -o /tmp/stress.csv
time cargo run --release -- /tmp/stress.csv > /tmp/output.csv
```

## Performance Considerations

- **Memory Efficiency**: Uses streaming CSV processing to handle files larger than RAM
- **Fixed-Point Arithmetic**: Avoids floating-point precision issues
- **Zero-Copy Operations**: Minimizes allocations where possible
- **Compiled Binary**: Release builds are optimized for performance

## Dependencies

- `csv`: CSV parsing
- `serde`: Serialization/deserialization for CSV input
- `clap`: Command-line argument parsing
- `thiserror`: Error type derivation
- `anyhow`: Error handling context
- `tracing`: Structured logging
- `tracing-subscriber`: Logging implementation

## Safety Guarantees

- **No Panics**: All errors are handled gracefully via Result types
- **Type Safety**: Amount newtype prevents direct i64 manipulation
- **Memory Safety**: No unsafe code
- **Overflow Protection**: All arithmetic uses checked operations (add_checked, sub_checked)

## Examples

### Simple Deposits and Withdrawals

```csv
type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,30.5
```

Output:
```csv
client,available,held,total,locked
1,69.5000,0.0000,69.5000,false
```

### Dispute and Resolve Flow

```csv
type,client,tx,amount
deposit,1,1,50.0
dispute,1,1,
resolve,1,1,
```

Output:
```csv
client,available,held,total,locked
1,50.0000,0.0000,50.0000,false
```

### Chargeback (Locks Account)

```csv
type,client,tx,amount
deposit,1,1,100.0
dispute,1,1,
chargeback,1,1,
deposit,1,2,50.0
```

Output (deposit 2 rejected because account is locked):
```csv
client,available,held,total,locked
1,0.0000,0.0000,0.0000,true
```

## Notes

**Code Style**: The code may appear verbose in some areas due to comprehensive tracing instrumentation. This is intentional to provide detailed observability when running with `--verbose`.

**AI Assistance**: AI tools were used in a measured way to generate tests and some utility functions under my direction. The architecture, design decisions, and core logic are 100% my own work.

## License

This project is under GPLv3 with attribution.

## Author

Johan Alexis Duque Cadena

For more information, run `cargo run -- --hire`
