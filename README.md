# hello-world

A Rust learning project. **Stdlib only — no dependencies.**

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
  `entries` (account + signed `Money` in integer cents). Balances are derived by
  summing entries, never stored.
- **One ledger per currency** — `Ledger::new(currency, fee_account)`. The ledger
  also owns its `FeeSchedule` (a per-bank rate card) and the fee account where
  collected fees land.
- Free/zero fees produce no ledger entry.
- CI: `.github/workflows/ci.yml` (fmt → clippy → test on `ubuntu-latest`).