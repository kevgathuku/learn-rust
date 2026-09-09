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

Rust learning project — **stdlib only, no dependencies** (Cargo.toml is effectively
empty). Write idiomatic stdlib Rust; do not reach for crates to avoid a few lines.

### Structure

- `src/lib.rs` — the ledger domain: `Ledger`, `Money`, `FeePolicy`, `FeeSchedule`,
  `TransactionChannel`, `Account`, etc., with unit tests inline.
- `src/bin/ledger.rs` — a thin `main()` demo of the ledger API.
- `src/bin/{hello,print,variables}.rs` — small Rust syntax exercises, unrelated to
  the ledger.

### Ledger design (steering decisions — preserve these)

- **Event-sourced**: a `Ledger` is a list of `Transaction`s; each transaction carries
  `entries: Vec<LedgerEntry>` (account + signed `Money`). Balances are always derived
  by summing entries, never stored.
- **One ledger per currency**: `Ledger::new(currency, fee_account)`. All accounts in a
  ledger share its currency. Cross-currency is a separate feature, not a ledger field.
- **The ledger owns its configuration**: the fee schedule and the fee account are
  fields on `Ledger`, never per-transaction parameters. `transfer`/`withdraw` read
  `self.*`.
- **Fees are a rate card**: `FeePolicy` (`Free | Flat { … }`) holds the default per
  channel; `FeeSchedule` is the per-bank card (a second bank = a second schedule).
  A zero or free fee produces **no fee entry** — skip it, don't record a 0.
- **Money is integer cents** (`amount_cents: i64`), never floats.

### Testing

- Unit tests live inline in `src/lib.rs` (`#[cfg(test)] mod tests`, `use super::*`)
  so they can access private fields (e.g. `ledger.accounts`). No test framework.
- Helpers: `with_accounts`, a per-test fee account const, and an `AtomicU64`
  counter for unique account ids (tests run in parallel).
- TDD for bugs: add a failing test first, then the smallest fix.

### Verification — always run before done

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```
