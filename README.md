# hello-world

A Rust learning project.

## Layout

```
src/
  lib.rs            # Ledger domain: Ledger, Money, FeePolicy, FeeSchedule,
                    # TransactionChannel, Account + inline unit tests
  bin/ledger.rs     # Demo: open accounts, deposit, transfer, print balances
  bin/hello.rs      # Syntax exercise: hello world
  bin/print.rs      # Syntax exercise: println! formatting
  bin/variables.rs  # Syntax exercise: mutability, shadowing, types
```

## Run a binary

Every file in `src/bin/` is its own program:

```sh
cargo run --bin ledger       # the ledger demo
cargo run --bin hello
cargo run --bin print
cargo run --bin variables
```

## Run the tests

Ledger unit tests live in `src/lib.rs`:

```sh
cargo test          # lib tests
cargo test --all-targets   # including bin tests
```

## Checks (CI runs these)

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

## Details

- The ledger is **event-sourced**: a `Ledger` holds transactions, each with
  `entries` (account + signed `Money` in integer cents). Balances are cached
  in a `HashMap` for O(1) lookups, updated on each transaction.
- **Double-entry bookkeeping** — every transaction entry sums to zero.
  `Ledger::new(currency, fee_account, external_account)` takes an external
  vault account that absorbs the offsetting entries for deposits and withdrawals.
- **Reversibility** — only transfers can be reversed via `reverse()`.
  Deposits and withdrawals are final. Double-reversal is prevented.
- **Timestamps** — each transaction has a `SystemTime` timestamp, displayed
  as `YYYY-MM-DD HH:MM:SS UTC` via chrono.
- **Idempotency keys** — `deposit`, `transfer`, `withdraw`, `reverse` accept
  an optional idempotency key. Duplicate keys are rejected.
- **One ledger per currency** — all accounts share the ledger's currency.
  The ledger owns its `FeeSchedule` (per-bank rate card), fee account,
  and external vault account.
- Free/zero fees produce no ledger entry.
- CI: `.github/workflows/ci.yml` (fmt → clippy → test on `ubuntu-latest`).