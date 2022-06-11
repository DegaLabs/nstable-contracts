use crate::*;
use near_sdk::{near_bindgen, AccountId};
use std::collections::HashMap;

uint::construct_uint!(
    pub struct U256(4);
);

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PoolInfo {
    pub pool_id: u32,
    pub owner_id: AccountId,
    pub lend_token_id: AssetId,
    pub collateral_token_id: AssetId, //borrowers collateral assets to borrow lended token
    pub min_cr: u64, //min of (value of collateral)/(value of borrowed), when CR of user belows min_cr, liquidation starts
    pub max_utilization: u64, //max of total borrow / total lend asset liquidity
    pub min_lend_token_deposit: U128,
    pub min_lend_token_borrow: U128,
    pub total_lend_asset_deposit: U128,
    pub total_collateral_deposit: U128,
    pub total_borrow: U128,
    pub fixed_interest_rate: u64,
    pub acc_interest_per_share: U128,
    pub last_acc_interest_update_timestamp_sec: u64,
    pub liquidation_bonus: u64
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountInfo {
    pool_id: u32,
    owner_id: AccountId,
    lend_token_id: AssetId,
    collateral_token_id: AssetId,
    deposits: HashMap<AssetId, U128>,
    borrow_amount: U128,
    min_cr: u64,

    //lending interest when account is a lender
    lending_interest_profit_debt: U128, //interest debt when in a lending position, this is similar to rewardDebt in sushi farming
    unpaid_lending_interest_profit: U128,
    total_lending_interest_profit: U128,
    last_lending_interest_reward_update_timestamp_sec: u64,

    //borrowing interest when accc is a borrower
    unpaid_borrowing_interest: U128, //interest unpaid in a borrowing positions
    total_borrowing_interest: U128, //interest unpaid in a borrowing positions
    last_borrowing_interest_update_timestamp_sec: u64,
    max_borrowable: U128
}

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

    pub fn get_account_state_in_pool(&self, pool_id: usize, account_id: AccountId) -> AccountInfo {
        let pool = self.pools.get(pool_id).expect("pool_id out of bound");
        let lend_token_info = self.get_token_info(pool.lend_token_id.clone());
        let lend_token_price = self.price_data.price(&pool.lend_token_id.clone());

        let collateral_token_info = self.get_token_info(pool.collateral_token_id.clone());
        let collateral_token_price = self.price_data.price(&pool.collateral_token_id.clone());
        let account_deposit = pool.get_account_deposit(&account_id);
        AccountInfo {
            pool_id: pool_id.clone() as u32,
            owner_id: account_id.clone(),
            lend_token_id: pool.lend_token_id.clone(),
            collateral_token_id: pool.collateral_token_id.clone(),
            deposits: pool.get_deposits(&account_id),
            borrow_amount: U128(account_deposit.borrow_amount),
            min_cr: pool.min_cr,
        
            //lending interest when account is a lender
            lending_interest_profit_debt: U128(account_deposit.lending_interest_profit_debt), //interest debt when in a lending position, this is similar to rewardDebt in sushi farming
            unpaid_lending_interest_profit: U128(account_deposit.unpaid_lending_interest_profit),
            total_lending_interest_profit: U128(account_deposit.total_lending_interest_profit),
            last_lending_interest_reward_update_timestamp_sec: account_deposit.last_lending_interest_reward_update_timestamp_sec,
        
            //borrowing interest when accc is a borrower
            unpaid_borrowing_interest: U128(account_deposit.unpaid_borrowing_interest), 
            total_borrowing_interest: U128(account_deposit.total_borrowing_interest), 
            last_borrowing_interest_update_timestamp_sec: account_deposit.last_borrowing_interest_update_timestamp_sec,
            max_borrowable: pool.compute_max_borrowable_for_account(&account_id, &lend_token_info, &lend_token_price, &collateral_token_info, &collateral_token_price, None).into()
        }
    }

    pub fn get_account_state(&self, account_id: AccountId) -> Vec<AccountInfo> {
        self.deposited_pools
            .get(&account_id)
            .unwrap_or(vec![])
            .iter()
            .map(|pool_id| self.get_account_state_in_pool(pool_id.clone() as usize, account_id.clone()))
            .collect::<Vec<_>>()
    }

    pub fn get_pools(&self, from_index: Option<usize>, limit: Option<usize>) -> Vec<PoolInfo> {
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        require!(limit != 0, "Cannot provide limit of 0.");
        let start_index = from_index.unwrap_or(0);
        self.pools
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|p| PoolInfo {
                pool_id: p.pool_id,
                owner_id: p.owner_id.clone(),
                lend_token_id: p.lend_token_id.clone(),
                collateral_token_id: p.collateral_token_id.clone(),
                min_cr: p.min_cr,
                max_utilization: p.max_utilization,
                min_lend_token_deposit: U128(p.min_lend_token_deposit),
                min_lend_token_borrow: U128(p.min_lend_token_borrow),
                total_lend_asset_deposit: U128(p.total_lend_asset_deposit),
                total_collateral_deposit: U128(p.total_collateral_deposit),
                total_borrow: U128(p.total_borrow),
                fixed_interest_rate: p.fixed_interest_rate,
                acc_interest_per_share: U128(p.acc_interest_per_share),
                last_acc_interest_update_timestamp_sec: p.last_acc_interest_update_timestamp_sec,
                liquidation_bonus: p.liquidation_bonus,
            })
            .collect::<Vec<_>>()
    }

    pub fn get_storage_account(&self, account_id: AccountId) -> UserStorageUsage {
        let storage_account = self.storage_accounts.get(&account_id).unwrap_or_default();
        storage_account
    }

    pub fn get_account_deposit_exist(&self, pool_id: usize, account_id: AccountId) -> bool {
        let pool = self.pools.get(pool_id).expect("pool_id out of bound");
        pool.account_deposits.get(&account_id).is_some()
    }

    pub fn get_pool_account_list(&self, pool_id: usize) -> Vec<AccountId> {
        let pool = self.pools.get(pool_id).expect("pool_id out of bound");
        pool.account_deposits.keys_as_vector().to_vec()
    }

    pub fn get_account_deposits_for_pool(
        &self,
        pool_id: usize,
        account_id: AccountId,
    ) -> HashMap<AssetId, U128> {
        let pool = self.pools.get(pool_id).expect("pool_id out of bound");
        pool.get_deposits(&account_id)
    }

    pub fn get_account_deposits(
        &self,
        account_id: AccountId,
    ) -> (Vec<usize>, Vec<HashMap<AssetId, U128>>) {
        let iter = self.deposited_pools.get(&account_id).unwrap_or(vec![]);
        let pool_ids = iter
            .iter()
            .map(|pool_id| pool_id.clone() as usize)
            .collect::<Vec<_>>();
        let deposits = iter
            .iter()
            .map(|pool_id| {
                self.get_account_deposits_for_pool(pool_id.clone() as usize, account_id.clone())
            })
            .collect::<Vec<_>>();
        (pool_ids, deposits)
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
        U128((storage_account.storage_usage as u128) * env::storage_byte_cost())
    }

    pub fn compute_current_cr(&self, pool_id: u32, account_id: AccountId) -> u64 {
        let pool = self
            .pools
            .get(pool_id as usize)
            .expect("pool_id out of range");
        let lend_token_info = self.get_token_info(pool.lend_token_id.clone());
        let lend_token_price = self.price_data.price(&pool.lend_token_id.clone());

        let collateral_token_info = self.get_token_info(pool.collateral_token_id.clone());
        let collateral_token_price = self.price_data.price(&pool.collateral_token_id.clone());
        pool.compute_current_cr(
            account_id,
            &lend_token_info,
            &lend_token_price,
            &collateral_token_info,
            &collateral_token_price,
        )
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
