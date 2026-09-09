use hello_world::{Account, AccountId, Currency, Ledger, Money, TransactionChannel};

fn main() {
    let alice = Account {
        id: AccountId(1),
        name: String::from("Alice"),
        currency: Currency::Kes,
    };
    let brian = Account {
        id: AccountId(2),
        name: String::from("Brian"),
        currency: Currency::Kes,
    };

    let bank_fee_account = AccountId(999);
    let mut ledger = Ledger::new(Currency::Kes);
    ledger.add_account(alice.clone()).unwrap();
    ledger.add_account(brian.clone()).unwrap();

    let deposit_amount = Money {
        amount_cents: 100_000,
        currency: Currency::Kes,
    };
    ledger
        .deposit(alice.id, deposit_amount, TransactionChannel::MobileApp)
        .unwrap();

    println!("Alice Before: {}", ledger.balance_for(alice.id));
    println!("Brian Before: {}", ledger.balance_for(brian.id));

    let transfer_amount = Money {
        amount_cents: 1_000,
        currency: Currency::Kes,
    };
    ledger
        .transfer(
            alice.id,
            brian.id,
            transfer_amount,
            TransactionChannel::Web,
            bank_fee_account,
        )
        .unwrap();

    println!("Alice: {}", ledger.balance_for(alice.id));
    println!("Brian: {}", ledger.balance_for(brian.id));
}
