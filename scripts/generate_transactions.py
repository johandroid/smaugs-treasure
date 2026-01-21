#!/usr/bin/env python3
"""
Transaction generator for Smaug's Treasure payment processor.

Generates CSV files with configurable transaction patterns for stress testing.
"""

import argparse
import random
import sys
from typing import TextIO


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate transaction CSV files for testing",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "-n", "--transactions",
        type=int,
        default=1_000_000,
        help="Total number of transactions to generate",
    )
    parser.add_argument(
        "-c", "--clients",
        type=int,
        default=500,
        help="Number of unique clients",
    )
    parser.add_argument(
        "-w", "--warmup",
        type=float,
        default=0.0,
        help="Percentage (0-100) of initial transactions that are only deposits/withdrawals",
    )
    parser.add_argument(
        "-o", "--output",
        type=str,
        default=None,
        help="Output file (default: stdout)",
    )
    parser.add_argument(
        "-s", "--seed",
        type=int,
        default=None,
        help="Random seed for reproducibility",
    )
    return parser.parse_args()


def random_amount() -> str:
    """Generate a random amount with up to 4 decimal places."""
    amount = random.uniform(0.0001, 1000.0)
    return f"{amount:.4f}"


def generate_deposit_or_withdrawal(
    client: int, tx_id: int, deposited_txs: dict[int, list[int]]
) -> str:
    """Generate a random deposit or withdrawal transaction."""
    tx_type = random.choice(["deposit", "withdrawal"])
    amount = random_amount()
    if tx_type == "deposit":
        if client not in deposited_txs:
            deposited_txs[client] = []
        deposited_txs[client].append(tx_id)
    return f"{tx_type},{client},{tx_id},{amount}"


def generate_any_transaction(
    client: int, tx_id: int, deposited_txs: dict[int, list[int]]
) -> str:
    """Generate any type of transaction."""
    # Weight towards deposits/withdrawals (70%) vs disputes (30%)
    if random.random() < 0.7:
        return generate_deposit_or_withdrawal(client, tx_id, deposited_txs)
    else:
        # Dispute-related transaction
        tx_type = random.choice(["dispute", "resolve", "chargeback"])
        # Try to reference a valid deposit from this client
        if client in deposited_txs and deposited_txs[client]:
            ref_tx = random.choice(deposited_txs[client])
        else:
            # Reference a random tx (might not exist)
            ref_tx = random.randint(1, tx_id)
        return f"{tx_type},{client},{ref_tx},"


def generate_transactions(
    total: int, clients: int, warmup_pct: float, output: TextIO
) -> None:
    """Generate transactions with configurable parameters."""
    output.write("type,client,tx,amount\n")

    # Track deposits per client for dispute references
    deposited_txs: dict[int, list[int]] = {}

    # Calculate warmup limit
    warmup_limit = int(total * (warmup_pct / 100.0))

    # Progress reporting interval
    report_interval = max(total // 10, 1)

    for tx_id in range(1, total + 1):
        client = random.randint(1, clients)

        if tx_id <= warmup_limit:
            # Warmup phase: only deposits and withdrawals
            line = generate_deposit_or_withdrawal(client, tx_id, deposited_txs)
        else:
            # After warmup: any transaction type
            line = generate_any_transaction(client, tx_id, deposited_txs)

        output.write(line + "\n")

        # Progress indicator to stderr
        if tx_id % report_interval == 0:
            pct = (tx_id / total) * 100
            print(f"Generated {tx_id:,} / {total:,} ({pct:.0f}%)...", file=sys.stderr)

    print(f"Done! Generated {total:,} transactions for {clients} clients.", file=sys.stderr)
    if warmup_pct > 0:
        print(f"Warmup: first {warmup_limit:,} ({warmup_pct}%) are deposits/withdrawals only.", file=sys.stderr)


def main() -> None:
    args = parse_args()

    if args.seed is not None:
        random.seed(args.seed)

    if args.warmup < 0 or args.warmup > 100:
        print("Error: warmup percentage must be between 0 and 100", file=sys.stderr)
        sys.exit(1)

    if args.output:
        with open(args.output, "w") as f:
            generate_transactions(args.transactions, args.clients, args.warmup, f)
        print(f"Output written to: {args.output}", file=sys.stderr)
    else:
        generate_transactions(args.transactions, args.clients, args.warmup, sys.stdout)


if __name__ == "__main__":
    main()
