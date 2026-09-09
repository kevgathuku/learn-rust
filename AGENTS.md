<!-- graft:start -->
## Graft — repo context graph

This repo is indexed in `graft/`: small linked markdown nodes that explain each
system and carry exact file:line spans, kept in sync with the code through git.

For ANY task here — understanding how something works, finding where code lives,
or scoping a change — get context from the graph before grepping or opening
source files. Re-ask freely (it's cheap) and reuse literal identifiers you
already have (symbol, error string, file name) as the query. New to this repo?
Run `graft map` first — a token-budgeted orientation (dir clusters, hubs,
hotspots), no LLM, no key.

- Run `graft ask "<your question>" --source` → ranked nodes with the relevant
  code spans inlined (each hit's ≤8-line crux by default; `--full` for whole
  definitions when the crux isn't enough). Match the tool to the task shape:
  for understanding or editing, the top node IS the answer — cite its
  `covers:` file:line spans and edit straight from `--source`. For
  exhaustive tasks ("every occurrence / every caller of this pattern"), ranked
  results are top-N, not complete — run `graft grep "<literal>"` instead
  (exhaustive over indexed files, grouped by enclosing symbol), falling back
  to raw `grep -rn` only for unindexed files.
- `graft skeleton <file>` → every definition's signature + span, ~10× cheaper
  than reading the file; use it to skim an API surface.
- `graft callers <symbol>` gives precomputed, exact edges — who calls this.
  Add `--direction out` for what it calls, or `--depth N` to walk
  transitively for the full blast radius. For structural questions, skip
  ranking and use this directly.
- Or browse: `graft/INDEX.md` lists every node; follow the links.
- Monorepos and folders of multiple repos rank fairly across sub-projects —
  hits carry `[scope/]` labels naming which one they're from. Narrow with
  `graft ask "<task>" --in <scope>/` once you know where you're working.

If a returned span is truncated ("+N more lines"), open the file at that exact
range before finalizing. Only open source files when a node genuinely lacks a
needed detail, and then at the exact file:line the node points to — never
re-read whole files.

After big code changes, refresh the graph with `graft build` (deterministic,
no API key, $0).
<!-- graft:end -->

## Project conventions

Rust learning project. Write idiomatic Rust.

### Structure

- `src/lib.rs` — the ledger domain: `Ledger`, `Money`, `FeePolicy`, `FeeSchedule`,
  `TransactionChannel`, `Account`, `ExchangeRate`, `RateCache`, `CrossCurrencyReceipt`,
  `RateReader` trait, with unit tests inline.
- `src/rates.rs` — SQLite-backed rate store (`SqliteRateStore`), Frankfurter API client,
  migrations, with unit tests inline.
- `src/bin/ledger.rs` — a thin `main()` demo of the ledger API with cross-currency transfer.
- `src/bin/rate_updater.rs` — periodic rate fetcher that writes to SQLite.
- `src/bin/{hello,print,variables}.rs` — small Rust syntax exercises, unrelated to
  the ledger.

### Ledger design (steering decisions — preserve these)

- **Event-sourced**: a `Ledger` is a list of `Transaction`s; each transaction carries
  `entries: Vec<LedgerEntry>` (account + signed `Money`). Balances are always derived
  by summing entries, never stored.
- **One ledger per currency**: `Ledger::new(currency, fee_account)`. All accounts in a
  ledger share its currency. Cross-currency is a separate feature, not a ledger field.
- **Multi-currency accounts**: accounts can have different currencies. `add_account`
  does not enforce a currency match with the ledger. `deposit`/`transfer`/`withdraw`
  validate the amount matches the account's currency. `cross_currency_transfer`
  validates the amount matches the sender's account currency.
- **The ledger owns its configuration**: the fee schedule and the fee account are
  fields on `Ledger`, never per-transaction parameters. `transfer`/`withdraw` read
  `self.*`.
- **Fee account is a full Account**: stored as `Account` (not just `AccountId`) so it's
  always available for display in transaction formatting.
- **Fees are a rate card**: `FeePolicy` supports `Free`, `Flat { amount_cents }`,
  `Percentage { rate_bps }`, and `PercentageWithCap { rate_bps, cap_cents }`.
  `FeeSchedule` maps channels to policies. `fee_for(amount: Money)` takes the
  transaction amount to support percentage-based calculations.
- **Money is integer cents** (`amount_cents: i64`), never floats.
- **Error variants are unit types**: `InvalidAccount(AccountId)`, `InsufficientFunds`,
  `SameSenderAndReceiver`, `InvalidAmount`, `CurrencyMismatch`. No redundant String
  payloads.
- **Account existence validated**: all methods (`deposit`, `transfer`, `withdraw`)
  verify the account exists before proceeding, returning `InvalidAccount` if not.
- **Display traits implemented**: `Currency`, `Money`, `TransactionKind`, `Transaction`
  all have Display impls. `Ledger::format_transaction()` resolves account names for
  human-readable output.
- **Transaction metadata via entries**: `sender()`/`receiver()` methods on `Transaction`
  derive from entry signs, not from `TransactionKind` fields.
- **Cross-currency transfers**: `cross_currency_transfer()` fetches rates from `RateCache`,
  converts the amount, debits sender in their currency, credits receiver in target currency.
  Fees are charged in the sender's currency. Returns `CrossCurrencyReceipt` with both
  sent/received amounts and the rate used.

### Testing

- Unit tests live inline in `src/lib.rs` (`#[cfg(test)] mod tests`, `use super::*`)
  so they can access private fields (e.g. `ledger.accounts`). No test framework.
- Helpers: `with_accounts`, a per-test fee account const, and an `AtomicU64`
  counter for unique account ids (tests run in parallel).
- TDD for bugs: add a failing test first, then the smallest fix.

### Rates

- `src/rates.rs` — `SqliteRateStore` wraps `rusqlite::Connection` with a `Mutex` for
  thread safety. Uses `rusqlite_migration` with `user_version` for schema tracking.
- `RateReader` trait abstracts rate sources — `SqliteRateStore`, `RateCache`, or mocks.
- `RateCache` wraps `ArcSwap<HashMap>` for lock-free reads. Calls `refresh()` to reload
  from the underlying store.
- `fetch_frankfurter()` calls the Frankfurter API v2 (`api.frankfurter.dev/v2/rates`),
  parses the flat array response, and returns rates in both directions.
- `src/bin/rate_updater.rs` — separate binary that fetches rates periodically (default
  1 hour) and writes to SQLite.

### Verification — always run before done

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Format all code before committing: `cargo fmt --all`
