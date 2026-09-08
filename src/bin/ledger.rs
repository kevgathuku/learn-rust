use hello_world::{
    Account, AccountId, Currency, FeePolicy, FeeSchedule, Ledger, Money, TransactionChannel,
};

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
    let mut ledger = Ledger::default();
    ledger.add_account(alice.clone());
    ledger.add_account(brian.clone());

    let schedule = FeeSchedule {
        mobile_app: FeePolicy::Free,
        web: FeePolicy::Free,
        branch: FeePolicy::Flat { amount_cents: 200 },
        agent: FeePolicy::Flat { amount_cents: 300 },
    };

    let deposit_amount = Money {
        amount_cents: 100_000,
        currency: Currency::Kes,
    };
    ledger
        .deposit(
            alice.id,
            deposit_amount,
            TransactionChannel::MobileApp,
            schedule,
        )
        .unwrap();

    println!(
        "Alice Before: {}",
        ledger.balance_for(alice.id, alice.currency)
    );
    println!(
        "Brian Before: {}",
        ledger.balance_for(brian.id, brian.currency)
    );

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
            schedule,
        )
        .unwrap();

    println!("Alice: {}", ledger.balance_for(alice.id, alice.currency));
    println!("Brian: {}", ledger.balance_for(brian.id, brian.currency));
}
