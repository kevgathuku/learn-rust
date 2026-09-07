#![allow(unused)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AccountId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct TransactionId(u64);

#[derive(Debug, Clone)]
struct Account {
    id: AccountId,
    name: String,
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
    amount: u64,
}

impl Transaction {
    fn balance_change_for(&self, account: AccountId) -> i64 {
        let amount = self.amount as i64;

        match self.kind {
            TransactionKind::Deposit {
                account: transaction_account,
            } if transaction_account == account => amount,
            TransactionKind::Withdrawal {
                account: transaction_account,
            } if transaction_account == account => -amount,
            TransactionKind::Transfer { from, to } if from == account => -amount,
            TransactionKind::Transfer { from, to } if to == account => amount,
            _ => 0,
        }
    }
}

#[derive(Debug, Default)]
struct Ledger {
    transactions: Vec<Transaction>,
}

impl Ledger {
    fn record(&mut self, kind: TransactionKind, amount: u64) {
        self.transactions.push(Transaction {
            id: TransactionId(self.transactions.len() as u64 + 1),
            kind,
            amount,
        });
    }

    fn balance_for(&self, account: AccountId) -> i64 {
        self.transactions
            .iter()
            .map(|transaction| transaction.balance_change_for(account))
            .sum()
    }

    fn deposit(&mut self, account: AccountId, amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Err(String::from("Invalid amount: Must be greater than 0"));
        }

        self.record(TransactionKind::Deposit { account }, amount);
        Ok(())
    }

    fn validate_transfer(
        &self,
        sender: AccountId,
        receiver: AccountId,
        amount: u64,
    ) -> Result<(), String> {
        if sender == receiver {
            return Err(String::from("Sender and receiver cannot be the same"));
        }

        if amount == 0 {
            return Err(String::from("Invalid amount: Must be greater than 0"));
        }

        if amount as i64 > self.balance_for(sender) {
            return Err(String::from("Sender does not have sufficient funds"));
        }

        Ok(())
    }

    fn transfer(
        &mut self,
        sender: AccountId,
        receiver: AccountId,
        amount: u64,
    ) -> Result<(), String> {
        self.validate_transfer(sender, receiver, amount)?; // short-circuit on error

        self.record(
            TransactionKind::Transfer {
                from: sender,
                to: receiver,
            },
            amount,
        );
        Ok(())
    }

    fn withdraw(&mut self, account: AccountId, amount: u64) -> Result<(), String> {
        if amount == 0 {
            return Err(String::from("Invalid amount: Must be greater than 0"));
        }

        if amount as i64 > self.balance_for(account) {
            return Err(String::from("Insufficient funds"));
        }

        self.record(TransactionKind::Withdrawal { account }, amount);
        Ok(())
    }
}

fn main() {
    let alice = Account {
        id: AccountId(1),
        name: String::from("Alice"),
    };
    let brian = Account {
        id: AccountId(2),
        name: String::from("Brian"),
    };

    let mut ledger = Ledger::default();

    ledger.deposit(alice.id, 1_000).unwrap();

    println!("Alice Before: {}", ledger.balance_for(alice.id));
    println!("Brian Before: {}", ledger.balance_for(brian.id));

    ledger.transfer(alice.id, brian.id, 100).unwrap();

    println!("Alice: {}", ledger.balance_for(alice.id));
    println!("Brian: {}", ledger.balance_for(brian.id));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(ledger: &mut Ledger, name: &str, balance: u64) -> Account {
        let n: u64 = rnd::random::<u64>();

        ledger.deposit(AccountId(n), balance).unwrap();
        Account {
            id: AccountId(n),
            name: name.into(),
        }
    }

    impl Ledger {
        fn with_accounts(
            first: (&str, u64),
            second: (&str, u64),
        ) -> (Self, Account, Account) {
            let mut ledger = Self::default();
            let first_account = account(&mut ledger, first.0, first.1);
            let second_account = account(&mut ledger, second.0, second.1);
            (ledger, first_account, second_account)
        }
    }

    #[test]
    fn rejects_same_sender_and_receiver() {
        let mut ledger = Ledger::default();
        let mut sender = account(&mut ledger, "Alice", 1000);

        assert!(ledger.validate_transfer(sender.id, sender.id, 10).is_err());
    }

    #[test]
    fn rejects_zero_amount() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 1000));

        assert!(ledger.validate_transfer(sender.id, sender.id, 0).is_err());
        assert!(ledger.transfer(sender.id, receiver.id, 0).is_err());
    }

    #[test]
    fn rejects_higher_amount_than_balance() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 1000));

        assert!(
            ledger
                .validate_transfer(sender.id, sender.id, 9_000)
                .is_err()
        );
    }

    #[test]
    fn accepts_different_users_and_valid_amount() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 1000));

        assert!(ledger.validate_transfer(sender.id, receiver.id, 10).is_ok());
        assert!(ledger.transfer(sender.id, receiver.id, 10).is_ok());
    }

    #[test]
    fn transfer_moves_funds() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 1000));

        assert!(ledger.transfer(sender.id, receiver.id, 300).is_ok());
        assert_eq!(ledger.balance_for(sender.id), 700);
        assert_eq!(ledger.balance_for(receiver.id), 1_300);
    }

    #[test]
    fn transfer_returns_correct_balances() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 1000));

        assert!(ledger.transfer(sender.id, receiver.id, 300).is_ok());
        assert_eq!(ledger.balance_for(sender.id), 700);
        assert_eq!(ledger.balance_for(receiver.id), 1300);
    }

    #[test]
    fn failed_transfer_leaves_balances_unchanged() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 1000));

        assert!(ledger.transfer(sender.id, receiver.id, 0).is_err());
        assert_eq!(ledger.balance_for(sender.id), 1000);
        assert_eq!(ledger.balance_for(receiver.id), 1000);
    }

    #[test]
    fn transfer_allows_same_name_with_different_ids() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Alice", 500));

        assert!(ledger.transfer(sender.id, receiver.id, 100).is_ok());
        assert_eq!(ledger.balance_for(sender.id), 900);
        assert_eq!(ledger.balance_for(receiver.id), 600);
    }

    #[test]
    fn transfer_fails_for_insufficient_funds() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 1000));

        assert!(ledger.transfer(sender.id, receiver.id, 9_000).is_err());
        assert_eq!(ledger.balance_for(sender.id), 1000);
        assert_eq!(ledger.balance_for(receiver.id), 1000);
    }

    #[test]
    fn transfer_exact_balance() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 500));

        assert!(ledger.transfer(sender.id, receiver.id, 1_000).is_ok());
        assert_eq!(ledger.balance_for(sender.id), 0);
        assert_eq!(ledger.balance_for(receiver.id), 1500);
    }

    #[test]
    fn multiple_transfers() {
        let (mut ledger, sender, receiver) =
            Ledger::with_accounts(("Alice", 1000), ("Brian", 500));

        assert!(ledger.transfer(sender.id, receiver.id, 200).is_ok());
        assert!(ledger.transfer(sender.id, receiver.id, 300).is_ok());
        assert_eq!(ledger.balance_for(sender.id), 500);
        assert_eq!(ledger.balance_for(receiver.id), 1000);
    }

    #[test]
    fn withdraw_reduces_balance() {
        let mut ledger = Ledger::default();
        let alice = account(&mut ledger, "Alice", 1000);

        assert!(ledger.withdraw(alice.id, 400).is_ok());
        assert_eq!(ledger.balance_for(alice.id), 600);
    }

    #[test]
    fn withdraw_fails_for_zero_amount() {
        let mut ledger = Ledger::default();
        let alice = account(&mut ledger, "Alice", 1000);

        assert!(ledger.withdraw(alice.id, 0).is_err());
        assert_eq!(ledger.balance_for(alice.id), 1000);
    }

    #[test]
    fn withdraw_fails_for_insufficient_funds() {
        let mut ledger = Ledger::default();
        let alice = account(&mut ledger, "Alice", 1000);

        assert!(ledger.withdraw(alice.id, 9000).is_err());
        assert_eq!(ledger.balance_for(alice.id), 1000);
    }

    #[test]
    fn withdraw_exact_balance() {
        let mut ledger = Ledger::default();
        let alice = account(&mut ledger, "Alice", 1000);

        assert!(ledger.withdraw(alice.id, 1000).is_ok());
        assert_eq!(ledger.balance_for(alice.id), 0);
    }
}
