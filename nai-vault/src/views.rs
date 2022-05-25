use near_sdk::{near_bindgen, AccountId};

use crate::*;

#[near_bindgen]
impl Contract {
    pub fn is_token_supported(&self, token_id: &AccountId) -> bool {
        let token_info = self.supported_tokens.get(token_id).unwrap_or_default();
        return token_info.token_id == AccountId::new_unchecked("".to_string());
    }

    pub fn blacklist_status(&self, account_id: &AccountId) -> BlackListStatus {
        return match self.black_list.get(account_id) {
            Some(x) => x.clone(),
            None => BlackListStatus::Allowable,
        };
    }

    pub fn compute_borrowable_amount(&self, collateral_token_id: AccountId, collateral_amount: U128) -> U128 {
        let borrowable = self.internal_compute_max_borrowable_amount(collateral_token_id.clone(), collateral_amount.0);
        U128(borrowable)
    }

    pub fn get_account_info(&self, account_id: AccountId) -> AccountDeposit {
        self.accounts.get(&account_id).unwrap_or_default()
    }

    pub fn get_account_vault(&self, account_id: AccountId, collateral_token_id: AccountId) -> Vault {
        let account_deposit = self.get_account_info(account_id.clone());
        account_deposit.get_vault(collateral_token_id.clone())
    }

    pub fn compute_storage_usage_near(&self, account_id: AccountId) -> Balance {
        let account_deposit = self.get_account_info(account_id.clone());
        account_deposit.storage_usage as u128
            * env::storage_byte_cost()
    }

    /// Returns how much NEAR is available for storage.
    pub fn storage_available(&self, account_id: AccountId) -> Balance {
        let account_deposit = self.get_account_info(account_id.clone());
        let locked = self.compute_storage_usage_near(account_id.clone());
        if account_deposit.near_amount > locked {
            account_deposit.near_amount - locked
        } else {
            0
        }
    }

    pub fn get_token_count(&self) -> usize {
        self.token_list.len()
    }

    pub fn compute_max_borrowable(&self, collateral_token_id: AccountId, collateral_amount: U128) -> U128 {
        U128(self.internal_compute_max_borrowable_amount(collateral_token_id.clone(), collateral_amount.0))
    }
}

impl Contract {
    pub fn internal_compute_max_borrowable_amount(&self, _collateral_token_id: AccountId, _collateral_amount: Balance) -> Balance {
        return (300 as u128) * (10u128.pow(18));
    }
}
