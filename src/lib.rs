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

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::Eur => write!(f, "EUR"),
            Currency::Usd => write!(f, "USD"),
            Currency::Kes => write!(f, "KES"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Money {
    pub amount_cents: i64,
    pub currency: Currency,
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let whole = self.amount_cents / 100;
        let cents = (self.amount_cents % 100).abs();
        write!(f, "{} {}.{:02}", self.currency, whole, cents)
    }
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
    Flat { amount_cents: i64 },
    Percentage { rate_bps: u64 },
    PercentageWithCap { rate_bps: u64, cap_cents: i64 },
}

impl FeePolicy {
    fn fee_for(self, amount: Money) -> Money {
        match self {
            FeePolicy::Free => Money {
                amount_cents: 0,
                currency: amount.currency,
            },
            FeePolicy::Flat { amount_cents } => Money {
                amount_cents,
                currency: amount.currency,
            },
            FeePolicy::Percentage { rate_bps } => {
                let fee = amount.amount_cents * rate_bps as i64 / 10_000;
                Money {
                    amount_cents: fee,
                    currency: amount.currency,
                }
            }
            FeePolicy::PercentageWithCap {
                rate_bps,
                cap_cents,
            } => {
                let fee = (amount.amount_cents * rate_bps as i64 / 10_000).min(cap_cents);
                Money {
                    amount_cents: fee,
                    currency: amount.currency,
                }
            }
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

    fn fee_for(&self, channel: TransactionChannel, amount: Money) -> Money {
        self.policy_for(channel).fee_for(amount)
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
pub enum TransactionKind {
    Deposit { account: AccountId },
    Withdrawal { account: AccountId },
    Transfer { from: AccountId, to: AccountId },
}

impl std::fmt::Display for TransactionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionKind::Deposit { account } => write!(f, "Deposit to {:?}", account),
            TransactionKind::Withdrawal { account } => write!(f, "Withdrawal from {:?}", account),
            TransactionKind::Transfer { from, to } => write!(f, "Transfer {:?} -> {:?}", from, to),
        }
    }
}

#[derive(Debug)]
pub struct Transaction {
    id: TransactionId,
    kind: TransactionKind,
    channel: TransactionChannel,
    entries: Vec<LedgerEntry>,
}

impl Transaction {
    fn sender(&self) -> Option<AccountId> {
        match &self.kind {
            TransactionKind::Deposit { .. } => None,
            TransactionKind::Withdrawal { account } => Some(*account),
            TransactionKind::Transfer { from, .. } => Some(*from),
        }
    }

    fn receiver(&self) -> Option<AccountId> {
        match &self.kind {
            TransactionKind::Deposit { account } => Some(*account),
            TransactionKind::Withdrawal { .. } => None,
            TransactionKind::Transfer { to, .. } => Some(*to),
        }
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{} {} via {:?}", self.id.0, self.kind, self.channel)
    }
}

#[derive(Debug)]
struct LedgerEntry {
    account: AccountId,
    amount: Money,
}

#[derive(Debug)]
pub enum LedgerError {
    InvalidAccount(AccountId),
    InsufficientFunds,
    SameSenderAndReceiver,
    InvalidAmount,
    CurrencyMismatch,
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAccount(id) => write!(f, "invalid account {:?}", id),
            Self::InsufficientFunds => write!(f, "insufficient funds"),
            Self::SameSenderAndReceiver => write!(f, "sender and receiver must differ"),
            Self::InvalidAmount => write!(f, "invalid amount"),
            Self::CurrencyMismatch => write!(f, "currency mismatch"),
        }
    }
}

impl std::error::Error for LedgerError {}

#[derive(Debug)]
pub struct Ledger {
    currency: Currency,
    fee_account: Account,
    external_account: Account,
    accounts: HashMap<AccountId, Account>,
    transactions: Vec<Transaction>,
    fee_schedule: FeeSchedule,
}

impl Ledger {
    pub fn new(currency: Currency, fee_account: Account, external_account: Account) -> Self {
        Self {
            currency,
            fee_account,
            external_account,
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
            .ok_or(LedgerError::InvalidAccount(id))
    }

    pub fn balance_for(&self, account: AccountId) -> i64 {
        self.transactions
            .iter()
            .flat_map(|transaction| transaction.entries.iter())
            .filter(|entry| entry.account == account && entry.amount.currency == self.currency)
            .map(|entry| entry.amount.amount_cents)
            .sum()
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn format_transaction(&self, tx: &Transaction) -> String {
        let sender = tx
            .sender()
            .and_then(|id| self.accounts.get(&id))
            .map(|a| a.name.as_str())
            .unwrap_or("external");
        let receiver = tx
            .receiver()
            .and_then(|id| self.accounts.get(&id))
            .map(|a| a.name.as_str())
            .unwrap_or("external");
        match &tx.kind {
            TransactionKind::Deposit { .. } => {
                let money = tx.entries.first().map(|e| e.amount).unwrap_or(Money {
                    amount_cents: 0,
                    currency: self.currency,
                });
                format!(
                    "#{} Deposit {} {} via {:?}",
                    tx.id.0, receiver, money, tx.channel
                )
            }
            TransactionKind::Withdrawal { .. } => {
                let money = tx.entries.first().map(|e| e.amount).unwrap_or(Money {
                    amount_cents: 0,
                    currency: self.currency,
                });
                let abs_money = Money {
                    amount_cents: money.amount_cents.abs(),
                    currency: money.currency,
                };
                format!(
                    "#{} Withdrawal {} {} via {:?}",
                    tx.id.0, sender, abs_money, tx.channel
                )
            }
            TransactionKind::Transfer { .. } => {
                let money = tx.entries.get(1).map(|e| e.amount).unwrap_or(Money {
                    amount_cents: 0,
                    currency: self.currency,
                });
                format!(
                    "#{} {} -> {} {} via {:?}",
                    tx.id.0, sender, receiver, money, tx.channel
                )
            }
        }
    }

    fn validate_amount(&self, amount: Money) -> Result<(), LedgerError> {
        if amount.amount_cents <= 0 {
            return Err(LedgerError::InvalidAmount);
        }
        if amount.currency != self.currency {
            return Err(LedgerError::CurrencyMismatch);
        }
        Ok(())
    }

    pub fn deposit(
        &mut self,
        account: AccountId,
        amount: Money,
        channel: TransactionChannel,
    ) -> Result<(), LedgerError> {
        self.account(account)?;
        self.validate_amount(amount)?;
        let transaction = Transaction {
            id: TransactionId(self.transactions.len() as u64 + 1),
            kind: TransactionKind::Deposit { account },
            channel,
            entries: vec![
                LedgerEntry { account, amount },
                LedgerEntry {
                    account: self.external_account.id,
                    amount: Money {
                        amount_cents: -amount.amount_cents,
                        currency: amount.currency,
                    },
                },
            ],
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
            return Err(LedgerError::SameSenderAndReceiver);
        }

        self.validate_amount(amount)?;

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

        let fee = self.fee_schedule.fee_for(channel, amount);
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
                account: self.fee_account.id,
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

        self.validate_amount(amount)?;

        let fee = self.fee_schedule.fee_for(channel, amount);
        let total_debit = amount.amount_cents + fee.amount_cents;
        let balance = self.balance_for(account_id);

        if balance < total_debit {
            return Err(LedgerError::InsufficientFunds);
        }

        let mut entries = vec![
            LedgerEntry {
                account: account_id,
                amount: Money {
                    amount_cents: -total_debit,
                    currency: amount.currency,
                },
            },
            LedgerEntry {
                account: self.external_account.id,
                amount,
            },
        ];

        // Record fees if applicable
        if fee.amount_cents > 0 {
            entries.push(LedgerEntry {
                account: self.fee_account.id,
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

    const CURRENCY: Currency = Currency::Kes;

    fn bank_fee_account() -> Account {
        Account {
            id: AccountId(999),
            name: "Bank KES Fee Account".into(),
            currency: CURRENCY,
        }
    }

    fn external_account() -> Account {
        Account {
            id: AccountId(998),
            name: "External Vault".into(),
            currency: CURRENCY,
        }
    }

    fn money(amount_cents: i64) -> Money {
        Money {
            amount_cents,
            currency: CURRENCY,
        }
    }

    fn account(ledger: &mut Ledger, name: &str, balance_cents: i64) -> Account {
        let id = AccountId(NEXT_ACCOUNT_ID.fetch_add(1, Ordering::Relaxed));

        let account = Account {
            id,
            name: name.into(),
            currency: CURRENCY,
        };
        ledger.accounts.insert(id, account.clone());

        ledger
            .deposit(id, money(balance_cents), TransactionChannel::MobileApp)
            .unwrap();

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
            let mut ledger = Self::new(CURRENCY, bank_fee_account(), external_account());
            let first_account = account(&mut ledger, first.0, first.1);
            let second_account = account(&mut ledger, second.0, second.1);
            (ledger, first_account, second_account)
        }
    }

    #[test]
    fn rejects_same_sender_and_receiver() {
        let mut ledger = Ledger::new(CURRENCY, bank_fee_account(), external_account());
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
    fn rejects_deposit_to_invalid_account() {
        let mut ledger = Ledger::new(CURRENCY, bank_fee_account(), external_account());
        let invalid_id = AccountId(999_999);

        let result = ledger.deposit(invalid_id, money(1_000), TransactionChannel::MobileApp);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LedgerError::InvalidAccount(id) if id == invalid_id));
    }

    #[test]
    fn rejects_transfer_from_invalid_account() {
        let (mut ledger, _, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));
        let invalid_id = AccountId(999_999);

        let result = ledger.transfer(
            invalid_id,
            receiver.id,
            money(1_000),
            TransactionChannel::MobileApp,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LedgerError::InvalidAccount(id) if id == invalid_id));
    }

    #[test]
    fn rejects_transfer_to_invalid_account() {
        let (mut ledger, sender, _) = Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));
        let invalid_id = AccountId(999_999);

        let result = ledger.transfer(
            sender.id,
            invalid_id,
            money(1_000),
            TransactionChannel::MobileApp,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LedgerError::InvalidAccount(id) if id == invalid_id));
    }

    #[test]
    fn rejects_withdraw_from_invalid_account() {
        let mut ledger = Ledger::new(CURRENCY, bank_fee_account(), external_account());
        let invalid_id = AccountId(999_999);

        let result = ledger.withdraw(invalid_id, money(1_000), TransactionChannel::MobileApp);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LedgerError::InvalidAccount(id) if id == invalid_id));
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

        let tx = ledger.transactions.last().unwrap();
        assert_eq!(tx.sender(), Some(sender.id));
        assert_eq!(tx.receiver(), Some(receiver.id));
    }

    #[test]
    fn transfer_returns_correct_balances() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 100_000), ("Brian", 100_000));

        transfer(&mut ledger, sender.id, receiver.id, 30_000).unwrap();

        assert_eq!(ledger.balance_for(sender.id), 70_000);
        assert_eq!(ledger.balance_for(receiver.id), 130_000);

        let tx = ledger.transactions.last().unwrap();
        assert_eq!(tx.sender(), Some(sender.id));
        assert_eq!(tx.receiver(), Some(receiver.id));
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

        let tx = ledger.transactions.last().unwrap();
        assert_eq!(tx.sender(), Some(sender.id));
        assert_eq!(tx.receiver(), Some(receiver.id));

        let entries = &tx.entries;
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .all(|entry| entry.account != bank_fee_account().id)
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

        let tx = ledger.transactions.last().unwrap();
        assert_eq!(tx.sender(), Some(sender.id));
        assert_eq!(tx.receiver(), Some(receiver.id));

        let entries = &tx.entries;
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.account == bank_fee_account().id)
                .map(|entry| entry.amount.amount_cents),
            Some(200)
        );
    }

    #[test]
    fn percentage_fee_calculates_correctly() {
        let fee_account = bank_fee_account();
        let mut ledger = Ledger {
            currency: CURRENCY,
            fee_account: fee_account.clone(),
            external_account: external_account(),
            accounts: HashMap::new(),
            transactions: Vec::new(),
            fee_schedule: FeeSchedule {
                mobile_app: FeePolicy::Percentage { rate_bps: 150 }, // 1.5%
                web: FeePolicy::Free,
                branch: FeePolicy::Free,
                agent: FeePolicy::Free,
            },
        };
        let alice = account(&mut ledger, "Alice", 100_000);
        let bob = account(&mut ledger, "Bob", 100_000);

        // 1.5% of 10_000 = 150
        ledger
            .transfer(
                alice.id,
                bob.id,
                money(10_000),
                TransactionChannel::MobileApp,
            )
            .unwrap();

        let tx = ledger.transactions.last().unwrap();
        let fee_entry = tx
            .entries
            .iter()
            .find(|e| e.account == fee_account.id)
            .unwrap();
        assert_eq!(fee_entry.amount.amount_cents, 150);
    }

    #[test]
    fn percentage_fee_with_cap() {
        let fee_account = bank_fee_account();
        let mut ledger = Ledger {
            currency: CURRENCY,
            fee_account: fee_account.clone(),
            external_account: external_account(),
            accounts: HashMap::new(),
            transactions: Vec::new(),
            fee_schedule: FeeSchedule {
                mobile_app: FeePolicy::PercentageWithCap {
                    rate_bps: 500,    // 5%
                    cap_cents: 1_000, // max 10.00
                },
                web: FeePolicy::Free,
                branch: FeePolicy::Free,
                agent: FeePolicy::Free,
            },
        };
        let alice = account(&mut ledger, "Alice", 1_000_000);
        let bob = account(&mut ledger, "Bob", 1_000_000);

        // 5% of 50_000 = 2_500, but capped at 1_000
        ledger
            .transfer(
                alice.id,
                bob.id,
                money(50_000),
                TransactionChannel::MobileApp,
            )
            .unwrap();

        let tx = ledger.transactions.last().unwrap();
        let fee_entry = tx
            .entries
            .iter()
            .find(|e| e.account == fee_account.id)
            .unwrap();
        assert_eq!(fee_entry.amount.amount_cents, 1_000);
    }

    #[test]
    fn percentage_fee_below_cap() {
        let fee_account = bank_fee_account();
        let mut ledger = Ledger {
            currency: CURRENCY,
            fee_account: fee_account.clone(),
            external_account: external_account(),
            accounts: HashMap::new(),
            transactions: Vec::new(),
            fee_schedule: FeeSchedule {
                mobile_app: FeePolicy::PercentageWithCap {
                    rate_bps: 500,    // 5%
                    cap_cents: 5_000, // max 50.00
                },
                web: FeePolicy::Free,
                branch: FeePolicy::Free,
                agent: FeePolicy::Free,
            },
        };
        let alice = account(&mut ledger, "Alice", 100_000);
        let bob = account(&mut ledger, "Bob", 100_000);

        // 5% of 10_000 = 500, below cap
        ledger
            .transfer(
                alice.id,
                bob.id,
                money(10_000),
                TransactionChannel::MobileApp,
            )
            .unwrap();

        let tx = ledger.transactions.last().unwrap();
        let fee_entry = tx
            .entries
            .iter()
            .find(|e| e.account == fee_account.id)
            .unwrap();
        assert_eq!(fee_entry.amount.amount_cents, 500);
    }

    #[test]
    fn withdraw_reduces_balance() {
        let mut ledger = Ledger::new(CURRENCY, bank_fee_account(), external_account());
        let alice = account(&mut ledger, "Alice", 100_000);

        withdraw(&mut ledger, alice.id, 40_000).unwrap();

        assert_eq!(ledger.balance_for(alice.id), 60_000);

        let tx = ledger.transactions.last().unwrap();
        assert_eq!(tx.sender(), Some(alice.id));
        assert_eq!(tx.receiver(), None);
    }

    #[test]
    fn withdraw_fails_for_zero_amount() {
        let mut ledger = Ledger::new(CURRENCY, bank_fee_account(), external_account());
        let alice = account(&mut ledger, "Alice", 100_000);

        assert!(withdraw(&mut ledger, alice.id, 0).is_err());
        assert_eq!(ledger.balance_for(alice.id), 100_000);
    }

    #[test]
    fn withdraw_fails_for_insufficient_funds() {
        let mut ledger = Ledger::new(CURRENCY, bank_fee_account(), external_account());
        let alice = account(&mut ledger, "Alice", 100_000);

        assert!(withdraw(&mut ledger, alice.id, 900_000).is_err());
        assert_eq!(ledger.balance_for(alice.id), 100_000);
    }

    #[test]
    fn withdraw_exact_balance() {
        let mut ledger = Ledger::new(CURRENCY, bank_fee_account(), external_account());
        let alice = account(&mut ledger, "Alice", 100_000);

        withdraw(&mut ledger, alice.id, 100_000).unwrap();

        assert_eq!(ledger.balance_for(alice.id), 0);
    }

    #[test]
    fn deposit_entries_sum_to_zero() {
        let (mut ledger, alice, _) = Ledger::with_accounts(("Alice", 100_000), ("Bob", 100_000));
        ledger
            .deposit(alice.id, money(10_000), TransactionChannel::MobileApp)
            .unwrap();
        let tx = ledger.transactions.last().unwrap();
        let sum: i64 = tx.entries.iter().map(|e| e.amount.amount_cents).sum();
        assert_eq!(sum, 0);
    }

    #[test]
    fn withdrawal_entries_sum_to_zero() {
        let (mut ledger, alice, _) = Ledger::with_accounts(("Alice", 100_000), ("Bob", 100_000));
        ledger
            .withdraw(alice.id, money(40_000), TransactionChannel::MobileApp)
            .unwrap();
        let tx = ledger.transactions.last().unwrap();
        let sum: i64 = tx.entries.iter().map(|e| e.amount.amount_cents).sum();
        assert_eq!(sum, 0);
    }

    #[test]
    fn all_transactions_entries_sum_to_zero() {
        let (mut ledger, alice, bob) = Ledger::with_accounts(("Alice", 100_000), ("Bob", 50_000));
        ledger
            .deposit(alice.id, money(10_000), TransactionChannel::MobileApp)
            .unwrap();
        ledger
            .transfer(alice.id, bob.id, money(30_000), TransactionChannel::Web)
            .unwrap();
        ledger
            .withdraw(bob.id, money(20_000), TransactionChannel::MobileApp)
            .unwrap();
        for tx in ledger.transactions() {
            let sum: i64 = tx.entries.iter().map(|e| e.amount.amount_cents).sum();
            assert_eq!(
                sum, 0,
                "Transaction {:?} entries don't sum to zero",
                tx.id.0
            );
        }
    }
}
