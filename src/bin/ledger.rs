use hello_world::rates::SqliteRateStore;
use hello_world::{Account, AccountId, Currency, Ledger, Money, RateCache, TransactionChannel};

fn main() {
    let alice = Account {
        id: AccountId(1),
        name: String::from("Alice"),
        currency: Currency::Eur,
    };
    let brian = Account {
        id: AccountId(2),
        name: String::from("Brian"),
        currency: Currency::Kes,
    };
    let bank_fee_account = Account {
        id: AccountId(999),
        name: String::from("Bank Fee Account"),
        currency: Currency::Eur,
    };
    let external_account = Account {
        id: AccountId(998),
        name: String::from("External Vault"),
        currency: Currency::Eur,
    };

    // Open rate store and create cache
    let store = SqliteRateStore::new("ledger.db").expect("failed to open rate store");
    let cache = RateCache::new(Box::new(store));

    let mut ledger = Ledger::with_rates(Currency::Eur, bank_fee_account, external_account, cache);
    ledger.add_account(alice.clone()).unwrap();
    ledger.add_account(brian.clone()).unwrap();

    // Deposit EUR to Alice
    let deposit_amount = Money {
        amount_cents: 100_000,
        currency: Currency::Eur,
    };
    ledger
        .deposit(
            alice.id,
            deposit_amount,
            TransactionChannel::MobileApp,
            None,
        )
        .unwrap();

    println!("Alice (EUR): {}", ledger.balance_for(alice.id));

    // Cross-currency transfer: Alice sends EUR, Brian receives KES
    let transfer_amount = Money {
        amount_cents: 10_000,
        currency: Currency::Eur,
    };
    match ledger.cross_currency_transfer(
        alice.id,
        brian.id,
        transfer_amount,
        Currency::Kes,
        TransactionChannel::Web,
        None,
    ) {
        Ok(receipt) => {
            println!("\n--- Cross-currency transfer ---");
            println!("Sent: {}", receipt.sent);
            println!("Received: {}", receipt.received);
            println!(
                "Rate: 1 {} = {:.4} {}",
                receipt.rate_used.from, receipt.rate_used.rate, receipt.rate_used.to
            );
            println!("Tx #{}", receipt.transaction_id.0);
        }
        Err(e) => {
            eprintln!("transfer failed: {e}");
            return;
        }
    }

    println!("\nAlice (EUR): {}", ledger.balance_for(alice.id));
    println!("Brian (KES): {}", ledger.balance_for(brian.id));

    println!("\nAll Transactions:");
    for tx in ledger.transactions() {
        println!("  {}", ledger.format_transaction(tx));
    }
}
