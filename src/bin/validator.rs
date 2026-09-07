#![allow(unused)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AccountId(u64);

#[derive(Debug, Clone)]
struct Account {
    id: AccountId,
    name: String,
    balance: u64,
}

#[derive(Debug)]
struct Transaction {
    sender: AccountId,
    receiver: AccountId,
    amount: u64,
}

fn validate(sender: &Account, receiver: &Account, amount: u64) -> Result<(), String> {
    if sender.id == receiver.id {
        return Err(String::from("Sender and receiver cannot be the same"));
    }

    if amount == 0 {
        return Err(String::from("Invalid amount: Must be greater than 0"));
    }

    if amount > sender.balance {
        return Err(String::from("Sender does not have sufficient funds"));
    }

    Ok(())
}

fn make_transfer(
    sender: &mut Account,
    receiver: &mut Account,
    amount: u64,
) -> Result<Transaction, String> {
    validate(sender, receiver, amount)?; // short-circuit on error
    sender.balance -= amount;
    receiver.balance += amount;
    Ok(Transaction {
        sender: sender.id,
        receiver: receiver.id,
        amount,
    })
}

fn main() {
    todo!();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(name: &str, balance: u64) -> Account {
        let n: u64 = rnd::random::<u64>();
        Account {
            id: AccountId(n),
            name: name.into(),
            balance,
        }
    }

    fn create_accounts() -> (Account, Account) {
        (account("Alice", 1000), account("Brian", 1000))
    }

    #[test]
    fn rejects_same_sender_and_receiver() {
        let mut sender = account("Alice", 1000);

        assert!(validate(&sender, &sender, 10).is_err());
    }

    #[test]
    fn rejects_zero_amount() {
        let (sender, receiver) = create_accounts();
        assert!(validate(&sender, &receiver, 0).is_err());
    }

    #[test]
    fn rejects_higher_amount_than_balance() {
        let (sender, receiver) = create_accounts();

        assert!(validate(&sender, &receiver, 9000).is_err());
    }

    #[test]
    fn accepts_different_users_and_valid_amount() {
        let (sender, receiver) = create_accounts();

        assert!(validate(&sender, &receiver, 10).is_ok());
    }

    #[test]
    fn transfer_moves_funds() {
        let mut sender = account("Alice", 1000);
        let mut receiver = account("Brian", 1000);

        make_transfer(&mut sender, &mut receiver, 300).unwrap();

        assert_eq!(sender.balance, 700);
        assert_eq!(receiver.balance, 1300);
    }

    #[test]
    fn transfer_returns_correct_transaction() {
        let mut sender = account("Alice", 1000);
        let mut receiver = account("Brian", 1000);
        let sender_id = sender.id;
        let receiver_id = receiver.id;

        let tx = make_transfer(&mut sender, &mut receiver, 300).unwrap();

        assert_eq!(tx.amount, 300);
        assert_eq!(sender.balance, 700);
        assert_eq!(receiver.balance, 1300);
    }

    #[test]
    fn failed_transfer_leaves_balances_unchanged() {
        let mut sender = account("Alice", 1000);
        let mut receiver = account("Brian", 1000);

        assert!(make_transfer(&mut sender, &mut receiver, 0).is_err());
        assert_eq!(sender.balance, 1000);
        assert_eq!(receiver.balance, 1000);
    }

    #[test]
    fn transfer_allows_same_name_with_different_ids() {
        let mut alice = account("Alice", 1000);
        let mut dup_alice = account("Alice", 500);

        make_transfer(&mut alice, &mut dup_alice, 100).unwrap();

        assert_eq!(alice.balance, 900);
        assert_eq!(dup_alice.balance, 600);
    }

    #[test]
    fn transfer_fails_for_zero_amount() {
        let mut sender = account("Alice", 1000);
        let mut receiver = account("Brian", 1000);

        assert!(make_transfer(&mut sender, &mut receiver, 0).is_err());
        assert_eq!(sender.balance, 1000);
        assert_eq!(receiver.balance, 1000);
    }

    #[test]
    fn transfer_fails_for_insufficient_funds() {
        let mut sender = account("Alice", 1000);
        let mut receiver = account("Brian", 1000);

        assert!(make_transfer(&mut sender, &mut receiver, 9000).is_err());
        assert_eq!(sender.balance, 1000);
        assert_eq!(receiver.balance, 1000);
    }

    #[test]
    fn transfer_exact_balance() {
        let mut sender = account("Alice", 1000);
        let mut receiver = account("Brian", 500);

        make_transfer(&mut sender, &mut receiver, 1000).unwrap();

        assert_eq!(sender.balance, 0);
        assert_eq!(receiver.balance, 1500);
    }

    #[test]
    fn multiple_transfers() {
        let mut sender = account("Alice", 1000);
        let mut receiver = account("Brian", 500);

        make_transfer(&mut sender, &mut receiver, 200).unwrap();
        make_transfer(&mut sender, &mut receiver, 300).unwrap();

        assert_eq!(sender.balance, 500);
        assert_eq!(receiver.balance, 1000);
    }
}
