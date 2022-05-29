use crate::oracle::Price;
use crate::*;
use near_sdk::{near_bindgen, AccountId};
uint::construct_uint!(
    pub struct U256(4);
);

#[near_bindgen]
impl Contract {
    pub fn is_token_supported(&self, token_id: &AccountId) -> bool {
        self.supported_tokens.contains_key(token_id)
    }

    pub fn get_token_info(&self, token_id: AssetId) -> TokenInfo {
        if !self.is_token_supported(&token_id) {
            return TokenInfo::new(token_id.clone(), 18);
        }
        self.supported_tokens.get(&token_id).unwrap()
    }

    pub fn get_all_token_info(&self) -> Vec<TokenInfo> {
        let mut ret = Vec::with_capacity(self.token_list.len());
        for token_id in &self.token_list {
            ret.push(self.get_token_info(token_id.clone()));
        }
        ret
    }

    pub fn blacklist_status(&self, account_id: &AccountId) -> BlackListStatus {
        return match self.black_list.get(account_id) {
            Some(x) => x.clone(),
            None => BlackListStatus::Allowable,
        };
    }

    pub fn get_token_count(&self) -> usize {
        self.token_list.len()
    }

    pub fn get_token_list(&self) -> &[AccountId] {
        &self.token_list
    }

    pub fn get_storage_account(&self, account_id: AccountId) -> UserStorageUsage {
        let storage_account = self.storage_accounts.get(&account_id).unwrap_or_default();
        storage_account
    }

    pub fn storage_available(&self, account_id: AccountId) -> U128 {
        let storage_account = self.get_storage_account(account_id.clone());
        let locked = self.compute_storage_usage_near(account_id.clone());
        if storage_account.near_amount > locked.0 {
            U128(storage_account.near_amount - locked.0)
        } else {
            U128(0)
        }
    }

    pub fn compute_storage_usage_near(&self, account_id: AccountId) -> U128 {
        let storage_account = self.get_storage_account(account_id.clone());
        U128(storage_account.storage_usage as u128 * env::storage_byte_cost())
    }
}

impl Contract {
    pub fn storage_cost(&self, prev_storage: StorageUsage) -> Balance {
        let storage_cost = env::storage_usage()
            .checked_sub(prev_storage)
            .unwrap_or_default() as Balance
            * env::storage_byte_cost();
        storage_cost
    }
}
