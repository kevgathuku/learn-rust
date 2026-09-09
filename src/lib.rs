#![allow(unused)]

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountId(pub u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct TransactionId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Currency {
    Eur,
    Usd,
    Kes,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Money {
    pub amount_cents: i64,
    pub currency: Currency,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub currency: Currency,
}

#[derive(Debug, Copy, Clone)]
pub enum FeePolicy {
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
pub enum TransactionChannel {
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

#[derive(Debug, Copy, Clone)]
pub struct FeeSchedule {
    pub mobile_app: FeePolicy,
    pub web: FeePolicy,
    pub branch: FeePolicy,
    pub agent: FeePolicy,
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

impl Default for FeeSchedule {
    fn default() -> Self {
        FeeSchedule {
            mobile_app: TransactionChannel::MobileApp.fee_policy(),
            web: TransactionChannel::Web.fee_policy(),
            branch: TransactionChannel::Branch.fee_policy(),
            agent: TransactionChannel::Agent.fee_policy(),
        }
    }
}

#[derive(Debug)]
enum TransactionKind {
    Deposit { account: AccountId },
    Withdrawal { account: AccountId },
    Transfer { from: AccountId, to: AccountId },
}

#[derive(Debug)]
struct Transaction {
    id: TransactionId,
    kind: TransactionKind,
    channel: TransactionChannel,
    entries: Vec<LedgerEntry>,
}

#[derive(Debug)]
struct LedgerEntry {
    account: AccountId,
    amount: Money,
}

#[derive(Debug)]
pub enum LedgerError {
    AccountNotFound(AccountId),
    InsufficientFunds,
    InvalidAccount(String),
    InvalidAmount(String),
    CurrencyMismatch,
}

#[derive(Debug)]
pub struct Ledger {
    currency: Currency,
    fee_account: AccountId,
    accounts: HashMap<AccountId, Account>,
    transactions: Vec<Transaction>,
    fee_schedule: FeeSchedule,
}

impl Ledger {
    pub fn new(currency: Currency, fee_account: AccountId) -> Self {
        Self {
            currency,
            fee_account,
            accounts: HashMap::new(),
            transactions: Vec::new(),
            fee_schedule: FeeSchedule::default(),
        }
    }

    pub fn add_account(&mut self, account: Account) -> Result<(), LedgerError> {
        if account.currency != self.currency {
            Err(LedgerError::CurrencyMismatch)
        } else {
            self.accounts.insert(account.id, account);
            Ok(())
        }
    }

    fn record(&mut self, transaction: Transaction) {
        self.transactions.push(transaction);
    }

    fn account(&self, id: AccountId) -> Result<&Account, LedgerError> {
        self.accounts
            .get(&id)
            .ok_or(LedgerError::AccountNotFound(id))
    }

    pub fn balance_for(&self, account: AccountId) -> i64 {
        self.transactions
            .iter()
            .flat_map(|transaction| transaction.entries.iter())
            .filter(|entry| entry.account == account && entry.amount.currency == self.currency)
            .map(|entry| entry.amount.amount_cents)
            .sum()
    }

    pub fn deposit(
        &mut self,
        account: AccountId,
        amount: Money,
        channel: TransactionChannel,
    ) -> Result<(), LedgerError> {
        if amount.amount_cents <= 0 {
            return Err(LedgerError::InvalidAmount(String::from(
                "Amount must be greater than 0",
            )));
        }
        if amount.currency != self.currency {
            return Err(LedgerError::CurrencyMismatch);
        }
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
    ) -> Result<(), LedgerError> {
        if from == to {
            return Err(LedgerError::InvalidAccount(String::from(
                "Sender and receiver cannot be the same",
            )));
        }

        if amount.amount_cents <= 0 {
            return Err(LedgerError::InvalidAmount(String::from(
                "Amount must be greater than 0",
            )));
        }

        if amount.currency != self.currency {
            return Err(LedgerError::CurrencyMismatch);
        }

        self.account(from)?;
        self.account(to)?;

        Ok(())
    }

    pub fn transfer(
        &mut self,
        sender: AccountId,
        receiver: AccountId,
        amount: Money,
        channel: TransactionChannel,
    ) -> Result<(), LedgerError> {
        self.validate_transfer(sender, receiver, amount)?; // short-circuit on error

        let fee = self.fee_schedule.fee_for(channel, self.currency);
        let total_debit = amount.amount_cents + fee.amount_cents;
        if total_debit > self.balance_for(sender) {
            return Err(LedgerError::InsufficientFunds);
        }

        let mut entries = vec![
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
        ];

        // Record fees if applicable
        if fee.amount_cents > 0 {
            entries.push(LedgerEntry {
                account: self.fee_account,
                amount: fee,
            });
        }

        let transaction = Transaction {
            id: TransactionId(self.transactions.len() as u64 + 1),
            kind: TransactionKind::Transfer {
                from: sender,
                to: receiver,
            },
            channel,
            entries,
        };

        self.record(transaction);
        Ok(())
    }

    pub fn withdraw(
        &mut self,
        account_id: AccountId,
        amount: Money,
        channel: TransactionChannel,
    ) -> Result<(), LedgerError> {
        self.account(account_id)?;

        if amount.amount_cents <= 0 {
            return Err(LedgerError::InvalidAmount(String::from(
                "Amount must be greater than 0",
            )));
        }

        if amount.currency != self.currency {
            return Err(LedgerError::CurrencyMismatch);
        }

        let fee = self.fee_schedule.fee_for(channel, self.currency);
        let total_debit = amount.amount_cents + fee.amount_cents;
        let balance = self.balance_for(account_id);

        if balance < total_debit {
            return Err(LedgerError::InsufficientFunds);
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
                account: self.fee_account,
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ACCOUNT_ID: AtomicU64 = AtomicU64::new(1);

    const BANK_FEE_ACCOUNT: AccountId = AccountId(999);
    const CURRENCY: Currency = Currency::Kes;

    fn money(amount_cents: i64) -> Money {
        Money {
            amount_cents,
            currency: CURRENCY,
        }
    }

    fn account(ledger: &mut Ledger, name: &str, balance_cents: i64) -> Account {
        let id = AccountId(NEXT_ACCOUNT_ID.fetch_add(1, Ordering::Relaxed));

        ledger
            .deposit(id, money(balance_cents), TransactionChannel::MobileApp)
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
    ) -> Result<(), LedgerError> {
        ledger.transfer(
            sender,
            receiver,
            money(amount_cents),
            TransactionChannel::MobileApp,
        )
    }

    fn withdraw(
        ledger: &mut Ledger,
        account_id: AccountId,
        amount_cents: i64,
    ) -> Result<(), LedgerError> {
        ledger.withdraw(
            account_id,
            money(amount_cents),
            TransactionChannel::MobileApp,
        )
    }

    impl Ledger {
        fn with_accounts(first: (&str, i64), second: (&str, i64)) -> (Self, Account, Account) {
            let mut ledger = Self::new(CURRENCY, BANK_FEE_ACCOUNT);
            let first_account = account(&mut ledger, first.0, first.1);
            let second_account = account(&mut ledger, second.0, second.1);
            (ledger, first_account, second_account)
        }
    }

    #[test]
    fn rejects_same_sender_and_receiver() {
        let mut ledger = Ledger::new(CURRENCY, BANK_FEE_ACCOUNT);
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

        assert_eq!(ledger.balance_for(sender.id), 70_000);
        assert_eq!(ledger.balance_for(receiver.id), 130_000);
    }

    #[test]
    fn transfer_returns_correct_balances() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        transfer(&mut ledger, sender.id, receiver.id, 30_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id), 70_000);
        assert_eq!(ledger.balance_for(receiver.id), 130_000);
    }

    #[test]
    fn failed_transfer_leaves_balances_unchanged() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        assert!(transfer(&mut ledger, sender.id, receiver.id, 0).is_err());
        assert_eq!(ledger.balance_for(sender.id), 100_000);
        assert_eq!(ledger.balance_for(receiver.id), 100_000);
    }

    #[test]
    fn transfer_allows_same_name_with_different_ids() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Alice", 50_000));

        transfer(&mut ledger, sender.id, receiver.id, 10_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id), 90_000);
        assert_eq!(ledger.balance_for(receiver.id), 60_000);
    }

    #[test]
    fn transfer_fails_for_insufficient_funds() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        assert!(transfer(&mut ledger, sender.id, receiver.id, 900_000).is_err());
        assert_eq!(ledger.balance_for(sender.id), 100_000);
        assert_eq!(ledger.balance_for(receiver.id), 100_000);
    }

    #[test]
    fn transfer_exact_balance() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 50_000));

        transfer(&mut ledger, sender.id, receiver.id, 100_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id), 0);
        assert_eq!(ledger.balance_for(receiver.id), 150_000);
    }

    #[test]
    fn multiple_transfers() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 50_000));

        transfer(&mut ledger, sender.id, receiver.id, 20_000).unwrap();
        transfer(&mut ledger, sender.id, receiver.id, 30_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id), 50_000);
        assert_eq!(ledger.balance_for(receiver.id), 100_000);
    }

    #[test]
    fn free_transfer_skips_fee_entry() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        ledger
            .transfer(
                sender.id,
                receiver.id,
                money(10_000),
                TransactionChannel::MobileApp,
            )
            .unwrap();

        let entries = &ledger.transactions.last().unwrap().entries;
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .all(|entry| entry.account != BANK_FEE_ACCOUNT)
        );
    }

    #[test]
    fn fee_charging_transfer_records_fee_entry() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        ledger
            .transfer(
                sender.id,
                receiver.id,
                money(10_000),
                TransactionChannel::Branch,
            )
            .unwrap();

        let entries = &ledger.transactions.last().unwrap().entries;
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.account == BANK_FEE_ACCOUNT)
                .map(|entry| entry.amount.amount_cents),
            Some(200)
        );
    }

    #[test]
    fn withdraw_reduces_balance() {
        let mut ledger = Ledger::new(CURRENCY, BANK_FEE_ACCOUNT);
        let alice = account(&mut ledger, "Alice", 100_000);

        withdraw(&mut ledger, alice.id, 40_000).unwrap();

        assert_eq!(ledger.balance_for(alice.id), 60_000);
    }

    #[test]
    fn withdraw_fails_for_zero_amount() {
        let mut ledger = Ledger::new(CURRENCY, BANK_FEE_ACCOUNT);
        let alice = account(&mut ledger, "Alice", 100_000);

        assert!(withdraw(&mut ledger, alice.id, 0).is_err());
        assert_eq!(ledger.balance_for(alice.id), 100_000);
    }

    #[test]
    fn withdraw_fails_for_insufficient_funds() {
        let mut ledger = Ledger::new(CURRENCY, BANK_FEE_ACCOUNT);
        let alice = account(&mut ledger, "Alice", 100_000);

        assert!(withdraw(&mut ledger, alice.id, 900_000).is_err());
        assert_eq!(ledger.balance_for(alice.id), 100_000);
    }

    #[test]
    fn withdraw_exact_balance() {
        let mut ledger = Ledger::new(CURRENCY, BANK_FEE_ACCOUNT);
        let alice = account(&mut ledger, "Alice", 100_000);

        withdraw(&mut ledger, alice.id, 100_000).unwrap();

        assert_eq!(ledger.balance_for(alice.id), 0);
    }
}
