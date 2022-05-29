use crate::*;
use near_sdk::{near_bindgen, AccountId, StorageUsage};


#[near_bindgen]
impl Contract {
    pub fn blacklist_status(&self, account_id: &AccountId) -> BlackListStatus {
        return match self.black_list.get(account_id) {
            Some(x) => x.clone(),
            None => BlackListStatus::Allowable,
        };
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
