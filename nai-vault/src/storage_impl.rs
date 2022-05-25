use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::json_types::U128;
use near_sdk::{assert_one_yocto, env, AccountId, Balance, Promise};

use crate::*;

impl Contract {
    /// Internal method that returns the Account ID and the balance in case the account was
    /// unregistered.
    pub fn internal_storage_unregister(
        &mut self,
        _force: Option<bool>,
    ) -> Option<(AccountId, Balance)> {
        assert_one_yocto();
        //TODO

        // let account_id = env::predecessor_account_id();
        // let force = force.unwrap_or(false);
        // if let Some(balance) = self.token.accounts.get(&account_id) {
        //     if balance == 0 || force {
        //         self.token.accounts.remove(&account_id);
        //         self.token.total_supply -= balance;
        //         if let Some(minted_for) = self.minted_for_lock.get(&account_id) {
        //             if minted_for == 0 || force {
        //                 self.lockeds.remove(&account_id);
        //                 self.minted_for_lock.remove(&account_id);
        //                 self.supply -= minted_for;
        //             }
        //         }
        //         Promise::new(account_id.clone()).transfer(self.storage_balance_bounds().min.0 + 1);
        //         Some((account_id, balance))
        //     } else {
        //         env::panic(
        //             "Can't unregister the account with the positive balance without force"
        //                 .as_bytes(),
        //         )
        //     }
        // } else {
        //     log!("The account {} is not registered", &account_id);
        //     None
        // }
        None
    }

    fn internal_storage_balance_of(&self, account_id: &AccountId) -> Option<StorageBalance> {
        if self.accounts.contains_key(account_id) {
            let account_deposit = self.get_account_info(account_id.clone());
            Some(StorageBalance {
                total: U128(account_deposit.near_amount),
                available: U128(self.storage_available(account_id.clone())),
            })
        } else {
            None
        }
    }
}

#[near_bindgen]
impl StorageManagement for Contract {
    // `registration_only` doesn't affect the implementation for vanilla fungible token.
    #[allow(unused_variables)]
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        let amount: Balance = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());

        let bounds_for_account = self.storage_balance_bounds_for_account(account_id.clone());
        let min = bounds_for_account.min.0;

        if amount < min {
            env::panic_str("The attached deposit is less than the minimum storage balance");
        }

        let refund = self.internal_register_account(&account_id, &amount);

        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }

        self.internal_storage_balance_of(&account_id).unwrap()
    }

    /// While storage_withdraw normally allows the caller to retrieve `available` balance, the basic
    /// Fungible Token implementation sets storage_balance_bounds.min == storage_balance_bounds.max,
    /// which means available balance will always be 0. So this implementation:
    /// * panics if `amount > 0`
    /// * never transfers Ⓝ to caller
    /// * returns a `storage_balance` struct if `amount` is 0
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();
        let predecessor_account_id = env::predecessor_account_id();
        if let Some(storage_balance) = self.internal_storage_balance_of(&predecessor_account_id) {
            match amount {
                Some(amount) if amount.0 > 0 => {
                    env::panic_str("The amount is greater than the available storage balance");
                }
                _ => storage_balance,
            }
        } else {
            env::panic_str(format!(
                "The account {} is not registered",
                &predecessor_account_id
            ).as_str());
        }
    }

    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.internal_storage_unregister(force).is_some()
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        let required_storage_balance = Balance::from(
            self.base_storage_usage
                + self.storage_usage_per_vault * (self.get_token_count() as u64),
        ) * env::storage_byte_cost();
        StorageBalanceBounds {
            min: required_storage_balance.into(),
            max: Some(required_storage_balance.into()),
        }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_storage_balance_of(&account_id)
    }
}

#[near_bindgen]
impl Contract {
    pub fn storage_balance_bounds_for_account(
        &self,
        account_id: AccountId,
    ) -> StorageBalanceBounds {
        let required_storage_balance = Balance::from(
            self.base_storage_usage
                + self.storage_usage_per_vault * (self.get_token_count() as u64),
        ) * env::storage_byte_cost();
        let deposit_account = self.get_account_info(account_id.clone());

        let near_deposited = deposit_account.near_amount;

        let mut min: Balance = 0;
        if near_deposited < required_storage_balance {
            min = required_storage_balance - near_deposited;
        }
        StorageBalanceBounds {
            min: min.into(),
            max: Some(min.into()),
        }
    }
}
