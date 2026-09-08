#![allow(unused)]

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AccountId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct TransactionId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Currency {
    Eur,
    Usd,
    Kes,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Money {
    amount_cents: i64,
    currency: Currency,
}

#[derive(Debug, Clone)]
struct Account {
    id: AccountId,
    name: String,
    currency: Currency,
}

#[derive(Debug, Copy, Clone)]
enum FeePolicy {
    Free,
    // TODO: Add currency specific fees
    Flat { amount_cents: i64 },
}

impl FeePolicy {
    fn fee_for(self, currency: Currency) -> Money {
        match self {
            FeePolicy::Free => Money {
                amount_cents: 0,
                currency,
            },
            FeePolicy::Flat { amount_cents } => Money {
                amount_cents,
                currency,
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum TransactionChannel {
    MobileApp,
    Web,
    Branch,
    Agent,
}

impl TransactionChannel {
    fn fee_policy(self) -> FeePolicy {
        match self {
            TransactionChannel::MobileApp => FeePolicy::Free,
            TransactionChannel::Web => FeePolicy::Free,
            TransactionChannel::Branch => FeePolicy::Flat { amount_cents: 200 },
            TransactionChannel::Agent => FeePolicy::Flat { amount_cents: 300 },
        }
    }
}

#[derive(Debug)]
enum TransactionKind {
    Deposit { account: AccountId },
    Withdrawal { account: AccountId },
    Transfer { from: AccountId, to: AccountId },
}

#[derive(Debug, Copy, Clone)]
struct FeeSchedule {
    mobile_app: FeePolicy,
    web: FeePolicy,
    branch: FeePolicy,
    agent: FeePolicy,
}

impl FeeSchedule {
    fn policy_for(&self, channel: TransactionChannel) -> FeePolicy {
        match channel {
            TransactionChannel::MobileApp => self.mobile_app,
            TransactionChannel::Web => self.web,
            TransactionChannel::Branch => self.branch,
            TransactionChannel::Agent => self.agent,
        }
    }

    fn fee_for(&self, channel: TransactionChannel, currency: Currency) -> Money {
        match self.policy_for(channel) {
            FeePolicy::Free => Money {
                amount_cents: 0,
                currency,
            },
            FeePolicy::Flat { amount_cents } => Money {
                amount_cents,
                currency,
            },
        }
    }
}

#[derive(Debug)]
struct Transaction {
    id: TransactionId,
    kind: TransactionKind,
    channel: TransactionChannel,
    entries: Vec<LedgerEntry>,
}

#[derive(Debug, Default)]
struct Ledger {
    accounts: HashMap<AccountId, Account>,
    transactions: Vec<Transaction>,
}

impl Ledger {
    fn record(&mut self, transaction: Transaction) {
        self.transactions.push(transaction);
    }

    fn account(&self, id: AccountId) -> Result<&Account, String> {
        self.accounts
            .get(&id)
            .ok_or_else(|| format!("Account {:?} not found", id))
    }

    fn balance_for(&self, account: AccountId, currency: Currency) -> i64 {
        self.transactions
            .iter()
            .flat_map(|transaction| transaction.entries.iter())
            .filter(|entry| entry.account == account && entry.amount.currency == currency)
            .map(|entry| entry.amount.amount_cents)
            .sum()
    }

    fn deposit(
        &mut self,
        account: AccountId,
        amount: Money,
        channel: TransactionChannel,
        fee_schedule: FeeSchedule,
    ) -> Result<(), String> {
        if amount.amount_cents <= 0 {
            return Err(String::from("Invalid amount: Must be greater than 0"));
        }
        let fee_policy = fee_schedule.policy_for(channel);
        let transaction = Transaction {
            id: TransactionId(self.transactions.len() as u64 + 1),
            kind: TransactionKind::Deposit { account },
            channel,
            entries: vec![LedgerEntry { account, amount }],
        };

        self.record(transaction);
        Ok(())
    }

    fn validate_transfer(
        &self,
        from: AccountId,
        to: AccountId,
        amount: Money,
    ) -> Result<(), String> {
        if from == to {
            return Err(String::from("Sender and receiver cannot be the same"));
        }

        if amount.amount_cents <= 0 {
            return Err(String::from("Invalid amount: Must be greater than 0"));
        }

        let sender = self.account(from)?;
        let receiver = self.account(to)?;

        if sender.currency != amount.currency {
            return Err("Transfer currency does not match sender account".into());
        }

        if receiver.currency != amount.currency {
            return Err("Transfer currency does not match receiver account".into());
        }

        Ok(())
    }

    fn transfer(
        &mut self,
        sender: AccountId,
        receiver: AccountId,
        amount: Money,
        channel: TransactionChannel,
        fee_account: AccountId,
        fee_schedule: FeeSchedule,
    ) -> Result<(), String> {
        self.validate_transfer(sender, receiver, amount)?; // short-circuit on error

        let fee = fee_schedule.policy_for(channel).fee_for(amount.currency);
        let total_debit = amount.amount_cents + fee.amount_cents;
        if total_debit > self.balance_for(sender, amount.currency) {
            return Err(String::from("Insufficient funds"));
        }

        let transaction = Transaction {
            id: TransactionId(self.transactions.len() as u64 + 1),
            kind: TransactionKind::Transfer {
                from: sender,
                to: receiver,
            },
            channel,
            entries: vec![
                LedgerEntry {
                    account: sender,
                    amount: Money {
                        amount_cents: -total_debit,
                        currency: amount.currency,
                    },
                },
                LedgerEntry {
                    account: receiver,
                    amount,
                },
                LedgerEntry {
                    account: fee_account,
                    amount: fee,
                },
            ],
        };

        self.record(transaction);
        Ok(())
    }

    fn withdraw(
        &mut self,
        account_id: AccountId,
        amount: Money,
        channel: TransactionChannel,
        fee_schedule: FeeSchedule,
        fee_account: AccountId,
    ) -> Result<(), String> {
        let account = self.account(account_id)?;

        if amount.amount_cents <= 0 {
            return Err(String::from("Invalid amount: Must be greater than 0"));
        }

        if account.currency != amount.currency {
            return Err("Withdrawal currency does not match account currency".into());
        }

        let fee = fee_schedule.fee_for(channel, amount.currency);
        let total_debit = amount.amount_cents + fee.amount_cents;
        let balance = self.balance_for(account_id, amount.currency);

        if balance < total_debit {
            return Err(String::from("Insufficient funds"));
        }

        let mut entries = vec![LedgerEntry {
            account: account_id,

            amount: Money {
                amount_cents: -total_debit,
                currency: amount.currency,
            },
        }];

        // Record fees if applicable
        if fee.amount_cents > 0 {
            entries.push(LedgerEntry {
                account: fee_account,
                amount: fee,
            });
        }

        let transaction = Transaction {
            id: TransactionId(self.transactions.len() as u64 + 1),
            kind: TransactionKind::Withdrawal {
                account: account_id,
            },
            channel,
            entries,
        };

        self.record(transaction);
        Ok(())
    }
}

#[derive(Debug)]
struct LedgerEntry {
    account: AccountId,
    amount: Money,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ACCOUNT_ID: AtomicU64 = AtomicU64::new(1);

    const BANK_FEE_ACCOUNT: AccountId = AccountId(999);
    const CURRENCY: Currency = Currency::Kes;
    const FREE_SCHEDULE: FeeSchedule = FeeSchedule {
        mobile_app: FeePolicy::Free,
        web: FeePolicy::Free,
        branch: FeePolicy::Free,
        agent: FeePolicy::Free,
    };

    fn money(amount_cents: i64) -> Money {
        Money {
            amount_cents,
            currency: CURRENCY,
        }
    }

    fn account(ledger: &mut Ledger, name: &str, balance_cents: i64) -> Account {
        let id = AccountId(NEXT_ACCOUNT_ID.fetch_add(1, Ordering::Relaxed));

        ledger
            .deposit(
                id,
                money(balance_cents),
                TransactionChannel::MobileApp,
                FREE_SCHEDULE,
            )
            .unwrap();

        let account = Account {
            id,
            name: name.into(),
            currency: CURRENCY,
        };
        ledger.accounts.insert(id, account.clone());
        account
    }

    fn transfer(
        ledger: &mut Ledger,
        sender: AccountId,
        receiver: AccountId,
        amount_cents: i64,
    ) -> Result<(), String> {
        ledger.transfer(
            sender,
            receiver,
            money(amount_cents),
            TransactionChannel::MobileApp,
            BANK_FEE_ACCOUNT,
            FREE_SCHEDULE,
        )
    }

    fn withdraw(
        ledger: &mut Ledger,
        account_id: AccountId,
        amount_cents: i64,
    ) -> Result<(), String> {
        ledger.withdraw(
            account_id,
            money(amount_cents),
            TransactionChannel::MobileApp,
            FREE_SCHEDULE,
            BANK_FEE_ACCOUNT,
        )
    }

    impl Ledger {
        fn with_accounts(first: (&str, i64), second: (&str, i64)) -> (Self, Account, Account) {
            let mut ledger = Self::default();
            let first_account = account(&mut ledger, first.0, first.1);
            let second_account = account(&mut ledger, second.0, second.1);
            (ledger, first_account, second_account)
        }
    }

    #[test]
    fn rejects_same_sender_and_receiver() {
        let mut ledger = Ledger::default();
        let sender = account(&mut ledger, "Alice", 1_000);

        assert!(
            ledger
                .validate_transfer(sender.id, sender.id, money(10))
                .is_err()
        );
    }

    #[test]
    fn rejects_zero_amount() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        assert!(
            ledger
                .validate_transfer(sender.id, sender.id, money(0))
                .is_err()
        );
        assert!(transfer(&mut ledger, sender.id, receiver.id, 0).is_err());
    }

    #[test]
    fn rejects_higher_amount_than_balance() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        assert!(transfer(&mut ledger, sender.id, receiver.id, 900_000).is_err());
    }

    #[test]
    fn accepts_different_users_and_valid_amount() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        assert!(
            ledger
                .validate_transfer(sender.id, receiver.id, money(1_000))
                .is_ok()
        );
        assert!(transfer(&mut ledger, sender.id, receiver.id, 1_000).is_ok());
    }

    #[test]
    fn transfer_moves_funds() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        transfer(&mut ledger, sender.id, receiver.id, 30_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id, CURRENCY), 70_000);
        assert_eq!(ledger.balance_for(receiver.id, CURRENCY), 130_000);
    }

    #[test]
    fn transfer_returns_correct_balances() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        transfer(&mut ledger, sender.id, receiver.id, 30_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id, CURRENCY), 70_000);
        assert_eq!(ledger.balance_for(receiver.id, CURRENCY), 130_000);
    }

    #[test]
    fn failed_transfer_leaves_balances_unchanged() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        assert!(transfer(&mut ledger, sender.id, receiver.id, 0).is_err());
        assert_eq!(ledger.balance_for(sender.id, CURRENCY), 100_000);
        assert_eq!(ledger.balance_for(receiver.id, CURRENCY), 100_000);
    }

    #[test]
    fn transfer_allows_same_name_with_different_ids() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Alice", 50_000));

        transfer(&mut ledger, sender.id, receiver.id, 10_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id, CURRENCY), 90_000);
        assert_eq!(ledger.balance_for(receiver.id, CURRENCY), 60_000);
    }

    #[test]
    fn transfer_fails_for_insufficient_funds() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        assert!(transfer(&mut ledger, sender.id, receiver.id, 900_000).is_err());
        assert_eq!(ledger.balance_for(sender.id, CURRENCY), 100_000);
        assert_eq!(ledger.balance_for(receiver.id, CURRENCY), 100_000);
    }

    #[test]
    fn transfer_exact_balance() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 50_000));

        transfer(&mut ledger, sender.id, receiver.id, 100_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id, CURRENCY), 0);
        assert_eq!(ledger.balance_for(receiver.id, CURRENCY), 150_000);
    }

    #[test]
    fn multiple_transfers() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 50_000));

        transfer(&mut ledger, sender.id, receiver.id, 20_000).unwrap();
        transfer(&mut ledger, sender.id, receiver.id, 30_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id, CURRENCY), 50_000);
        assert_eq!(ledger.balance_for(receiver.id, CURRENCY), 100_000);
    }

    #[test]
    fn withdraw_reduces_balance() {
        let mut ledger = Ledger::default();
        let alice = account(&mut ledger, "Alice", 100_000);

        withdraw(&mut ledger, alice.id, 40_000).unwrap();

        assert_eq!(ledger.balance_for(alice.id, CURRENCY), 60_000);
    }

    #[test]
    fn withdraw_fails_for_zero_amount() {
        let mut ledger = Ledger::default();
        let alice = account(&mut ledger, "Alice", 100_000);

        assert!(withdraw(&mut ledger, alice.id, 0).is_err());
        assert_eq!(ledger.balance_for(alice.id, CURRENCY), 100_000);
    }

    #[test]
    fn withdraw_fails_for_insufficient_funds() {
        let mut ledger = Ledger::default();
        let alice = account(&mut ledger, "Alice", 100_000);

        assert!(withdraw(&mut ledger, alice.id, 900_000).is_err());
        assert_eq!(ledger.balance_for(alice.id, CURRENCY), 100_000);
    }

    #[test]
    fn withdraw_exact_balance() {
        let mut ledger = Ledger::default();
        let alice = account(&mut ledger, "Alice", 100_000);

        withdraw(&mut ledger, alice.id, 100_000).unwrap();

        assert_eq!(ledger.balance_for(alice.id, CURRENCY), 0);
    }
}
