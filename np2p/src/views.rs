use crate::*;
use near_sdk::{near_bindgen, AccountId};
use std::collections::HashMap;
use utils::{compute_token_value, compute_token_value_usd};
uint::construct_uint!(
    pub struct U256(4);
);

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenMetaInfo {
    lend_token_price: Price,
    collateral_token_price: Price,
    lend_token_info: TokenInfo,
    collateral_token_info: TokenInfo,
}

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
    pub total_collateral_deposit_value_usd: U128,
    pub total_borrow: U128,
    pub total_borrow_value_usd: U128,
    pub fixed_interest_rate: u64,
    pub acc_interest_per_share: U128,
    pub last_acc_interest_update_timestamp_sec: u64,
    pub liquidation_bonus: u64,
    pub token_meta_info: TokenMetaInfo,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountInfo {
    pool_id: u32,
    owner_id: AccountId,
    lend_token_id: AssetId,
    collateral_token_id: AssetId,
    deposits: HashMap<AssetId, U128>,
    deposits_value_usd: HashMap<AssetId, U128>,
    borrow_amount: U128,
    borrow_value_usd: U128,
    min_cr: u64,
    current_cr: u64,

    //lending interest when account is a lender
    lending_interest_profit_debt: U128, //interest debt when in a lending position, this is similar to rewardDebt in sushi farming
    unpaid_lending_interest_profit: U128,
    total_lending_interest_profit: U128,
    last_lending_interest_reward_update_timestamp_sec: u64,

    //borrowing interest when accc is a borrower
    unpaid_borrowing_interest: U128, //interest unpaid in a borrowing positions
    total_borrowing_interest: U128,  //interest unpaid in a borrowing positions
    last_borrowing_interest_update_timestamp_sec: u64,
    max_borrowable: U128,

    //token info
    token_meta_info: TokenMetaInfo,
    liquidation_price: Price,

    unrecorded_interest: U128,
    acc_interest_per_share: U128,
    total_interest_reward: U128
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

    pub fn get_token_meta_info(&self, pool_id: usize) -> TokenMetaInfo {
        let pool = self.pools.get(pool_id).expect("pool_id out of bound");
        TokenMetaInfo {
            lend_token_info: self.get_token_info(pool.lend_token_id.clone()),
            lend_token_price: self.price_data.price(&pool.lend_token_id.clone()),
            collateral_token_info: self.get_token_info(pool.collateral_token_id.clone()),
            collateral_token_price: self.price_data.price(&pool.collateral_token_id.clone()),
        }
    }

    pub fn get_account_state_in_pool(
        &self,
        pool_id: usize,
        account_id: AccountId,
        collateral_amount: Option<U128>,
        borrow: Option<U128>,
        pay_amount: Option<U128>,
    ) -> AccountInfo {
        let collateral_amount = collateral_amount.unwrap_or(U128(0));
        let borrow = borrow.unwrap_or(U128(0));
        let pay_amount = pay_amount.unwrap_or(U128(0));

        let pool = self.pools.get(pool_id).expect("pool_id out of bound");
        let token_meta_info = self.get_token_meta_info(pool_id);
        let account_deposit = pool.get_account_deposit(&account_id);
        let deposits = pool.get_deposits(&account_id);
        let mut deposits_value_usd = HashMap::<AssetId, U128>::new();
        for (token_id, deposited) in deposits.iter() {
            deposits_value_usd.insert(
                token_id.clone(),
                compute_token_value_usd(
                    deposited.0 + collateral_amount.0,
                    token_meta_info.collateral_token_info.decimals,
                    &token_meta_info.collateral_token_price,
                ),
            );
        }

        AccountInfo {
            pool_id: pool_id.clone() as u32,
            owner_id: account_id.clone(),
            lend_token_id: pool.lend_token_id.clone(),
            collateral_token_id: pool.collateral_token_id.clone(),
            deposits: deposits,
            deposits_value_usd: deposits_value_usd,
            borrow_amount: U128(account_deposit.borrow_amount.clone()),
            borrow_value_usd: compute_token_value_usd(
                account_deposit.borrow_amount.clone(),
                token_meta_info.lend_token_info.decimals,
                &token_meta_info.lend_token_price,
            ),
            min_cr: pool.min_cr,
            current_cr: self.compute_current_cr(
                pool_id.clone() as u32,
                account_id.clone(),
                Some(collateral_amount),
                Some(borrow),
                Some(pay_amount),
            ),
            //lending interest when account is a lender
            lending_interest_profit_debt: U128(account_deposit.lending_interest_profit_debt), //interest debt when in a lending position, this is similar to rewardDebt in sushi farming
            unpaid_lending_interest_profit: U128(
                pool.get_pending_unpaid_lending_interest_profit(&account_id),
            ),
            total_lending_interest_profit: U128(
                pool.get_pending_total_lending_interest_profit(&account_id),
            ),
            last_lending_interest_reward_update_timestamp_sec: account_deposit
                .last_lending_interest_reward_update_timestamp_sec,
            //borrowing interest when accc is a borrower
            unpaid_borrowing_interest: U128(
                pool.get_pending_unpaid_borrowing_interest(&account_id),
            ),
            total_borrowing_interest: U128(pool.get_pending_total_borrowing_interest(&account_id)),
            last_borrowing_interest_update_timestamp_sec: account_deposit
                .last_borrowing_interest_update_timestamp_sec,
            max_borrowable: pool
                .compute_max_borrowable_for_account(
                    &account_id,
                    &token_meta_info.lend_token_info,
                    &token_meta_info.lend_token_price,
                    &token_meta_info.collateral_token_info,
                    &token_meta_info.collateral_token_price,
                    Some(collateral_amount.0),
                )
                .into(),
            token_meta_info: token_meta_info.clone(),
            liquidation_price: self.compute_liquidation_price(
                pool_id.clone(),
                account_id.clone(),
                Some(collateral_amount),
                Some(borrow),
                Some(pay_amount),
            ),
            unrecorded_interest: U128(pool.compute_unrecorded_interest(&account_id)),
            acc_interest_per_share: pool.get_current_acc_interest_per_share().into(),
            total_interest_reward: pool.get_total_interest_reward(&account_id).into()
        }
    }

    pub fn compute_max_borrowable_for_account(
        &self,
        pool_id: usize,
        account_id: AccountId,
    ) -> U128 {
        let token_meta_info = self.get_token_meta_info(pool_id);
        let pool = self.pools.get(pool_id).expect("pool_id out of bound");
        pool.compute_max_borrowable_for_account(
            &account_id,
            &token_meta_info.lend_token_info,
            &token_meta_info.lend_token_price,
            &token_meta_info.collateral_token_info,
            &token_meta_info.collateral_token_price,
            Some(0),
        )
        .into()
    }

    pub fn get_account_state(&self, account_id: AccountId) -> Vec<AccountInfo> {
        self.deposited_pools
            .get(&account_id)
            .unwrap_or(vec![])
            .iter()
            .map(|pool_id| {
                self.get_account_state_in_pool(
                    pool_id.clone() as usize,
                    account_id.clone(),
                    None,
                    None,
                    None,
                )
            })
            .collect::<Vec<_>>()
    }

    pub fn get_pool(&self, pool_id: usize) -> PoolInfo {
        let p = self.pools.get(pool_id).expect("pool_id out of bound");
        let token_meta_info = self.get_token_meta_info(pool_id.clone());
        PoolInfo {
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
            total_collateral_deposit_value_usd: compute_token_value_usd(
                p.total_collateral_deposit.clone(),
                token_meta_info.collateral_token_info.decimals,
                &token_meta_info.collateral_token_price,
            ),
            total_borrow: U128(p.total_borrow),
            total_borrow_value_usd: compute_token_value_usd(
                p.total_borrow.clone(),
                token_meta_info.lend_token_info.decimals,
                &token_meta_info.lend_token_price,
            ),
            fixed_interest_rate: p.fixed_interest_rate,
            acc_interest_per_share: U128(p.acc_interest_per_share),
            last_acc_interest_update_timestamp_sec: p.last_acc_interest_update_timestamp_sec,
            liquidation_bonus: p.liquidation_bonus,
            token_meta_info: token_meta_info,
        }
    }

    pub fn get_pools(&self, from_index: Option<usize>, limit: Option<usize>) -> Vec<PoolInfo> {
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        require!(limit != 0, "Cannot provide limit of 0.");
        let start_index = from_index.unwrap_or(0);
        self.pools
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|p| self.get_pool(p.pool_id.clone() as usize))
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

    pub fn get_deposits(&self, account_id: AccountId) -> (Vec<usize>, Vec<HashMap<AssetId, U128>>) {
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

    pub fn compute_current_cr(
        &self,
        pool_id: u32,
        account_id: AccountId,
        collateral_amount: Option<U128>,
        borrow: Option<U128>,
        pay_amount: Option<U128>,
    ) -> u64 {
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
            Some(collateral_amount.unwrap_or(U128(0)).0),
            Some(borrow.unwrap_or(U128(0)).0),
            Some(pay_amount.unwrap_or(U128(0)).0),
        )
    }

    pub fn compute_liquidation_price(
        &self,
        pool_id: usize,
        account_id: AccountId,
        collateral_amount: Option<U128>,
        borrow_amount: Option<U128>,
        pay_amount: Option<U128>,
    ) -> Price {
        let pool = self.pools.get(pool_id).expect("pool_id out of range");
        let token_meta_info = self.get_token_meta_info(pool_id.clone());

        let collateral_amount = collateral_amount.unwrap_or(U128(0));
        let borrow_amount = borrow_amount.unwrap_or(U128(0));
        let pay_amount = pay_amount.unwrap_or(U128(0));

        let total_collateral_amount =
            collateral_amount.0 + pool.get_token_deposit(&account_id, &pool.collateral_token_id);
        let mut total_borrow =
            borrow_amount.0 + pool.get_token_deposit(&account_id, &pool.lend_token_id);

        if total_borrow > pay_amount.0 {
            total_borrow -= pay_amount.0;
        } else {
            total_borrow = 0;
        }
        let total_borrow_value =
            compute_token_value(total_borrow.clone(), &token_meta_info.lend_token_price);
        let min_required_collateral_value =
            total_borrow_value * pool.min_cr / COLLATERAL_RATIO_DIVISOR;

        //this price is in USD from oracle, we need the price collateral/lend ratio
        let liquidation_price = min_required_collateral_value
            * U256::from(10u128.pow(token_meta_info.lend_token_price.decimals as u32))  //should be 8 for both lend and collateral token price decimal
            * U256::from(10u128.pow(token_meta_info.collateral_token_info.decimals as u32))
            / (U256::from(total_collateral_amount)
                * U256::from(10u128.pow(token_meta_info.lend_token_info.decimals as u32)));
        let liquidation_price = liquidation_price.as_u128();
        let multiplier = liquidation_price
            * 10u128.pow(token_meta_info.lend_token_price.decimals as u32)
            / token_meta_info.lend_token_price.multiplier.0;
        Price {
            multiplier: U128(multiplier),
            decimals: token_meta_info.lend_token_price.decimals,
        }
    }

    pub fn get_account_list_count(&self) -> usize {
        self.account_list.len()
    }

    pub fn get_account_list(
        &self,
        from_index: Option<usize>,
        limit: Option<usize>,
    ) -> Vec<&AccountId> {
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        require!(limit != 0, "Cannot provide limit of 0.");
        let start_index = from_index.unwrap_or(0);
        self.account_list
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .collect::<Vec<_>>()
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
