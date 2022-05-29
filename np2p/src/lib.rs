mod governance;
mod oracle;
//mod storage;
mod storage_impl;
//mod token_receiver;
//mod utils;
mod account_deposit;
mod pool;
mod utils;
mod views;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedMap};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, require, AccountId, Balance, BorshStorageKey, Gas,
    PanicOnDefault, Promise, StorageUsage,
};

use account_deposit::AccountDeposit;
use pool::{new_pool_default, Pool};

pub type AssetId = AccountId;

use oracle::{Price, PriceData};
use std::fmt::Debug;

use views::U256;

const BORROW_FEE_DIVISOR: u128 = 10000;
const COLLATERAL_RATIO_DIVISOR: u128 = 10000;
const UTILIZATION_DIVISOR: u128 = 10000;
const INTEREST_RATE_DIVISOR: u128 = 10000;
const ACC_INTEREST_PER_SHARE_MULTIPLIER: u128 = 10u128.pow(8 as u32);
const LOW_POSITION_VALUE_NAI: u128 = 20 * (10u128.pow(18 as u32));
const SECONDS_PER_YEAR: u128 = 365 * 86400;

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    Blacklist,
    SupportedTokens,
    TokenToListLendPools,
    TokenToListCollateralPools,
    CreatedPools,
    DepositedPools,
    BorrowPools,
    UserStorage,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum BlackListStatus {
    // An address might be using
    Allowable,
    // All acts with an address have to be banned
    Banned,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum ContractStatus {
    Working,
    Paused,
}

impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractStatus::Working => write!(f, "working"),
            ContractStatus::Paused => write!(f, "paused"),
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct UserStorageUsage {
    pub near_amount: Balance,
    pub storage_usage: StorageUsage,
}

impl Default for UserStorageUsage {
    fn default() -> UserStorageUsage {
        UserStorageUsage {
            near_amount: 0,
            storage_usage: 0,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenInfo {
    pub token_id: AssetId,
    pub decimals: u8,
}

impl TokenInfo {
    pub fn new(token_id: AssetId, decimals: u8) -> TokenInfo {
        TokenInfo {
            token_id: token_id,
            decimals: decimals,
        }
    }

    fn default() -> TokenInfo {
        TokenInfo {
            token_id: AccountId::new_unchecked("".to_string()),
            decimals: 0,
        }
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    governance: AccountId,
    black_list: LookupMap<AccountId, BlackListStatus>,
    status: ContractStatus,
    supported_tokens: LookupMap<AssetId, TokenInfo>,
    token_list: Vec<AssetId>,
    price_data: PriceData,
    price_feeder: AccountId,
    foundation_id: AccountId,
    pool_creation_fee: Balance,
    pools: Vec<Pool>,
    token_to_list_lend_pools: UnorderedMap<AssetId, Vec<u32>>,
    token_to_list_collateral_pools: UnorderedMap<AssetId, Vec<u32>>,
    created_pools: UnorderedMap<AccountId, Vec<u32>>,
    deposited_pools: UnorderedMap<AccountId, Vec<u32>>,
    borrow_pools: UnorderedMap<AccountId, Vec<u32>>,
    storage_accounts: LookupMap<AccountId, UserStorageUsage>,
    storage_usage_add_pool: StorageUsage,
    storage_usage_join_pool: StorageUsage,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        governance: AccountId,
        foundation: AccountId,
        price_feeder: Option<AccountId>,
    ) -> Self {
        let price_feeder = price_feeder.unwrap_or(governance.clone());
        require!(!env::state_exists(), "Already initialized");

        let mut this = Self {
            governance: governance.clone(),
            black_list: LookupMap::new(StorageKey::Blacklist),
            status: ContractStatus::Working,
            supported_tokens: LookupMap::new(StorageKey::SupportedTokens),
            price_data: PriceData::default(),
            price_feeder: price_feeder,
            token_list: vec![],
            foundation_id: foundation.clone(),
            pool_creation_fee: 10u128.pow(24 as u32) * 10, //10 near to avoid spam
            pools: vec![],
            token_to_list_lend_pools: UnorderedMap::new(StorageKey::TokenToListLendPools),
            token_to_list_collateral_pools: UnorderedMap::new(
                StorageKey::TokenToListCollateralPools,
            ),
            created_pools: UnorderedMap::new(StorageKey::CreatedPools),
            deposited_pools: UnorderedMap::new(StorageKey::DepositedPools),
            borrow_pools: UnorderedMap::new(StorageKey::BorrowPools),
            storage_accounts: LookupMap::new(StorageKey::UserStorage),
            storage_usage_add_pool: 0,
            storage_usage_join_pool: 0,
        };

        this.measure_account_storage_usage();
        this
    }

    pub fn set_foundation_id(&mut self, account_id: AccountId) {
        self.assert_governance();
        self.foundation_id = account_id;
    }

    #[payable]
    pub fn add_new_supported_token(&mut self, token_id: AccountId, decimals: u8) {
        self.assert_governance();
        require!(
            !self.is_token_supported(&token_id),
            "token already supported"
        );
        let prev_storage = env::storage_usage();
        self.supported_tokens.insert(
            &token_id,
            &TokenInfo {
                token_id: token_id.clone(),
                decimals: decimals,
            },
        );
        self.token_list.push(token_id.clone());
        let storage_cost = env::storage_usage()
            .checked_sub(prev_storage)
            .unwrap_or_default() as Balance
            * env::storage_byte_cost();

        let refund = env::attached_deposit().checked_sub(storage_cost).expect(
            format!(
                "ERR_STORAGE_DEPOSIT need {}, attatched {}",
                storage_cost,
                env::attached_deposit()
            )
            .as_str(),
        );
        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
    }

    fn measure_account_storage_usage(&mut self) {
        let mut initial_storage_usage = env::storage_usage();
        let tmp_account_id = AccountId::new_unchecked("a".repeat(64).to_string());
        let tmp_lend_id = AccountId::new_unchecked("b".repeat(64).to_string());
        let tmp_collateral_id = AccountId::new_unchecked("c".repeat(64).to_string());
        let mut tmp_pool = new_pool_default(
            0,
            tmp_account_id.clone(),
            tmp_lend_id.clone(),
            tmp_collateral_id.clone(),
        );
        let account_deposit = AccountDeposit::new(
            0,
            tmp_account_id.clone(),
            tmp_lend_id.clone(),
            tmp_collateral_id.clone(),
        );
        tmp_pool
            .account_deposits
            .insert(&tmp_account_id, &account_deposit);

        self.pools.push(tmp_pool);

        let mut token_to_list_lend_pools = self
            .token_to_list_lend_pools
            .get(&tmp_lend_id)
            .unwrap_or(vec![]);
        token_to_list_lend_pools.push(0);
        self.token_to_list_lend_pools
            .insert(&tmp_lend_id, &token_to_list_lend_pools);

        let mut token_to_list_collateral_pools = self
            .token_to_list_collateral_pools
            .get(&tmp_collateral_id)
            .unwrap_or(vec![]);
        token_to_list_collateral_pools.push(0);
        self.token_to_list_collateral_pools
            .insert(&tmp_collateral_id, &token_to_list_collateral_pools);

        self.add_to_created_pools_list(&tmp_account_id, 0u32);

        self.add_to_deposit_pools_list(&tmp_account_id, 0u32);

        self.storage_usage_add_pool = env::storage_usage() - initial_storage_usage;

        initial_storage_usage = env::storage_usage();
        let tmp_pool = &mut self.pools[0];
        let tmp_account_id2 = AccountId::new_unchecked("e".repeat(64).to_string());
        tmp_pool.register_account(&tmp_account_id2);
        self.add_to_deposit_pools_list(&tmp_account_id2, 0u32);
        self.storage_usage_join_pool = env::storage_usage() - initial_storage_usage;

        //clean out
        self.deposited_pools.remove(&tmp_account_id);
        self.deposited_pools.remove(&tmp_account_id2);
        self.created_pools.remove(&tmp_account_id);
        self.token_to_list_lend_pools.remove(&tmp_lend_id);
        self.token_to_list_collateral_pools
            .remove(&tmp_collateral_id);
        self.pools.remove(0);
    }

    #[payable]
    pub fn borrow(&mut self, pool_id: u32, borrow_amount: U128, to: Option<AccountId>) {
        let prev_storage = env::storage_usage();
        // Select target account.
        let account_id = to.unwrap_or(env::predecessor_account_id());

        self.abort_if_pause();
        self.abort_if_blacklisted(account_id.clone());
        self.abort_if_pool_id_valid(pool_id.clone() as usize);

        let pool = self
            .pools
            .get(pool_id as usize)
            .expect("pool_id out of range");
        let lend_token_info = self.get_token_info(pool.lend_token_id.clone());
        let lend_token_price = self.price_data.price(&pool.lend_token_id.clone());

        let collateral_token_info = self.get_token_info(pool.collateral_token_id.clone());
        let collateral_token_price = self.price_data.price(&pool.collateral_token_id.clone());
        {
            let pool = &mut self.pools[pool_id as usize];
            pool.borrow(
                &account_id,
                &borrow_amount.0,
                &lend_token_info,
                &lend_token_price,
                &collateral_token_info,
                &collateral_token_price,
            );
        }

        self.add_to_borrow_pools_list(&account_id, pool_id);
        self.verify_storage(&account_id, prev_storage, Some(env::attached_deposit()));
    }

    #[payable]
    pub fn withdraw(&mut self, pool_id: u32, token_id: AssetId, amount: U128) -> Promise {
        let account_id = env::predecessor_account_id();
        assert_one_yocto();
        self.abort_if_pause();
        self.abort_if_blacklisted(account_id.clone());
        self.abort_if_pool_id_valid(pool_id.clone() as usize);

        let pool = self
            .pools
            .get(pool_id as usize)
            .expect("pool_id out of range");
        let lend_token_info = self.get_token_info(pool.lend_token_id.clone());
        let lend_token_price = self.price_data.price(&pool.lend_token_id.clone());

        let collateral_token_info = self.get_token_info(pool.collateral_token_id.clone());
        let collateral_token_price = self.price_data.price(&pool.collateral_token_id.clone());
        {
            let pool = &mut self.pools[pool_id as usize];
            pool.withdraw_from_account(
                &account_id,
                &token_id,
                amount.0,
                &lend_token_info,
                &lend_token_price,
                &collateral_token_info,
                &collateral_token_price,
            );
        }

        self.internal_send_tokens(pool_id, &token_id, &account_id, amount.0)
    }

    // #[payable]
    // pub fn liquidate(
    //     &mut self,
    //     account_id: AccountId,
    //     collateral_token_id: AccountId,
    //     nai_amount: U128,
    // ) {
    //     let prev_usage = env::storage_usage();
    //     let maker_id = env::predecessor_account_id();

    //     require!(nai_amount.0 > 0, "nai_amount > 0");
    //     require!(
    //         self.token.ft_balance_of(maker_id.clone()).0 >= nai_amount.0,
    //         "maker insufficient balance"
    //     );

    //     //account must under collateral_ratio
    //     let mut account_deposit = self.get_account_info(account_id.clone());
    //     let mut vault = account_deposit.get_vault(collateral_token_id.clone());
    //     let vault_index = account_deposit.get_vault_index(collateral_token_id.clone());

    //     require!(nai_amount.0 <= vault.borrowed.0, "nai_amount > 0");

    //     let vault_before = vault.clone();
    //     require!(vault.deposited.0 > 0, "no deposited");

    //     let (account_collateral_ratio, collateral_ratio) = self.compute_new_ratio_after_borrow(
    //         account_id.clone(),
    //         collateral_token_id.clone(),
    //         U128(0),
    //         U128(0),
    //     );
    //     require!(
    //         account_collateral_ratio < collateral_ratio,
    //         "account must be under collateral ratio"
    //     );

    //     let mut token_info = self.get_token_info(collateral_token_id.clone());
    //     let price = self.price_data.price(&collateral_token_id);
    //     let multiplier: u128 = price.multiplier.0
    //         * (BORROW_FEE_DIVISOR - (token_info.liquidation_price_fee as u128))
    //         / BORROW_FEE_DIVISOR;
    //     let price_after_liquidation_price_fee = Price {
    //         decimals: price.decimals,
    //         multiplier: U128(multiplier),
    //     };

    //     let liquidate_value = (U256::from(nai_amount.0)
    //         * U256::from(10u128.pow(token_info.decimals as u32)))
    //         / (U256::from(10u128.pow(18 as u32)));
    //     let liquidate_collateral = liquidate_value
    //         * U256::from(10u128.pow(price_after_liquidation_price_fee.decimals as u32))
    //         / U256::from(price_after_liquidation_price_fee.multiplier.0);
    //     let mut liquidate_collateral = liquidate_collateral.as_u128();

    //     //insufficient deposit of account for liquidation should we liquidate all?
    //     //TODO: the system should reward NST token to users who provide liquidation
    //     require!(
    //         liquidate_collateral <= vault.deposited.0,
    //         "insufficient deposit of account for liquidation"
    //     );
    //     vault.deposited = U128(vault.deposited.0 - liquidate_collateral.clone());
    //     vault.borrowed = U128(vault.borrowed.0 - nai_amount.0);

    //     if vault.borrowed.0 == 0 {
    //         //liquidate all if the remaining deposited value <= LOW_POSITION_VALUE_NAI
    //         let remain_collateral_value = self.compute_collateral_value(&vault.deposited.0, &price);
    //         let remain_collateral_value_in_nai = remain_collateral_value * U256::from(10u128.pow(18 as u32))
    //             / U256::from(10u128.pow(token_info.decimals as u32));
    //         require!(
    //             remain_collateral_value_in_nai.as_u128() <= LOW_POSITION_VALUE_NAI,
    //             "remaining collateral must be  below 20$ to liquidate all"
    //         );
    //         liquidate_collateral = liquidate_collateral + vault.deposited.0;
    //         vault.deposited = U128(0);
    //     }
    //     token_info.total_deposit = U128(token_info.total_deposit.0 - liquidate_collateral);
    //     token_info.total_borrowed = U128(token_info.total_borrowed.0 - nai_amount.0);

    //     self.supported_tokens
    //         .insert(&collateral_token_id, &token_info);

    //     self.total_nai_borrowed = U128(self.total_nai_borrowed.0 - nai_amount.0);
    //     //burn nai
    //     self.token.internal_withdraw(&maker_id, nai_amount.0);

    //     //compute liquidated collateral amount to cover NAI burnt by maker
    //     let liquidate_collateral_to_cover_nai_burnt = liquidate_value
    //     * U256::from(10u128.pow(price.decimals as u32))
    //     / U256::from(price.multiplier.0);
    //     let liquidate_collateral_to_cover_nai_burnt = liquidate_collateral_to_cover_nai_burnt.as_u128();
    //     let remain_penalty_in_collateral = liquidate_collateral - liquidate_collateral_to_cover_nai_burnt;

    //     let liquidate_collateral_to_treasury = remain_penalty_in_collateral * 50 / 100;
    //     let liquidate_collateral_to_maker = liquidate_collateral - liquidate_collateral_to_treasury;

    //     //deposit to maker & foundation account
    //     self.deposit_to_vault(
    //         &collateral_token_id,
    //         &liquidate_collateral_to_treasury,
    //         &self.foundation_id.clone(),
    //     );
    //     self.deposit_to_vault(
    //         &collateral_token_id,
    //         &liquidate_collateral_to_maker,
    //         &maker_id,
    //     );

    //     //save vault of account_id
    //     account_deposit.vaults[vault_index] = vault.clone();
    //     self.accounts.insert(&account_id, &account_deposit);

    //     if vault.borrowed.0 > 0 {
    //         //collateral ratio must less than min
    //         let account_collateral_ratio = self.internal_compute_collateral_ratio(
    //             &collateral_token_id,
    //             vault.deposited.0,
    //             vault.borrowed.0,
    //         );

    //         require!(
    //             account_collateral_ratio <= collateral_ratio,
    //             "invalid collateral ratio after liquidation"
    //         );
    //     }

    //     let liquidaion_history = Liquidation {
    //         owner_id: account_id.clone(),
    //         maker_id: maker_id.clone(),
    //         token_id: collateral_token_id.clone(),
    //         collateral_amount_before: vault_before.deposited,
    //         collateral_amount_after: vault.deposited,
    //         borrowed_before: vault_before.borrowed,
    //         borrowed_after: vault.borrowed,
    //         timestamp_sec: env::block_timestamp_ms() / 1000,
    //         nai_burnt: nai_amount,
    //         maker_collateral_amount_received: U128(liquidate_collateral_to_maker),
    //         treasury_collateral_amount_received: U128(liquidate_collateral_to_treasury),
    //         liquidation_price: price_after_liquidation_price_fee, //price with liquidation fee
    //         price: price,
    //     };
    //     self.liquidation_history.push(liquidaion_history);

    //     let storage_cost = self.storage_cost(prev_usage);

    //     let refund = env::attached_deposit().checked_sub(storage_cost).expect(
    //         format!(
    //             "ERR_STORAGE_DEPOSIT need {}, attatched {}",
    //             storage_cost,
    //             env::attached_deposit()
    //         )
    //         .as_str(),
    //     );
    //     if refund > 0 {
    //         Promise::new(env::predecessor_account_id()).transfer(refund);
    //     }
    // }

    pub fn contract_status(&self) -> ContractStatus {
        self.status.clone()
    }

    pub fn version(&self) -> String {
        format!("{}:{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
    }

    /// This is NOOP implementation. KEEP IT if you haven't changed contract state.
    /// Should only be called by this contract on migration.
    /// This method is called from `upgrade()` method.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let contract: Self = env::state_read().expect("Contract is not initialized");
        contract
    }

    fn abort_if_pause(&self) {
        if self.status == ContractStatus::Paused {
            env::panic_str("The contract is under maintenance")
        }
    }

    fn abort_if_pool_id_valid(&self, pool_id: usize) {
        if self.pools.len() <= pool_id {
            env::panic_str("pool_id out of range")
        }
    }

    fn abort_if_unsupported_token(&self, token_id: AccountId) {
        if !self.is_token_supported(&token_id) {
            env::panic_str("The token is not supported")
        }
    }

    fn abort_if_blacklisted(&self, account_id: AccountId) {
        if self.blacklist_status(&account_id) != BlackListStatus::Allowable {
            env::panic_str(&format!("Account '{}' is banned", account_id));
        }
    }
}

impl Contract {
    pub fn pay_loan(
        &mut self,
        pool_id: u32,
        account_id: &AccountId,
        lend_token_id: &AccountId,
        pay_amount: U128,
    ) {
        self.abort_if_pause();
        self.abort_if_blacklisted(account_id.clone());
        self.abort_if_pool_id_valid(pool_id.clone() as usize);

        let pool = self
            .pools
            .get(pool_id as usize)
            .expect("pool_id out of range");
        require!(lend_token_id.clone() == pool.lend_token_id, "invalid token lend");

        let pool = &mut self.pools[pool_id as usize];
        pool.pay_loan(account_id, pay_amount.0);
    }

    pub fn deposit(
        &mut self,
        pool_id: u32,
        account_id: &AccountId,
        token_id: &AssetId,
        amount: Balance,
    ) {
        if pool_id >= self.pools.len() as u32 {
            env::panic_str("pool_id out of bound");
        }

        let prev_storage = env::storage_usage();
        let pool = &mut self.pools[pool_id.clone() as usize];
        pool.deposit(account_id, token_id, amount.clone());
        self.add_to_deposit_pools_list(account_id, pool_id);
        self.verify_storage(account_id, prev_storage, None);
    }

    pub fn verify_storage(
        &mut self,
        account_id: &AccountId,
        prev_storage: StorageUsage,
        attached_deposit: Option<Balance>,
    ) {
        let attached_deposit = attached_deposit.unwrap_or(0);
        let storage_cost = self.storage_cost(prev_storage);
        let mut storage_account = self.get_storage_account_unwrap(account_id);
        storage_account.storage_usage += storage_cost as u64;
        storage_account.near_amount += attached_deposit;
        self.storage_accounts.insert(account_id, &storage_account);
        self.assert_storage_usage(account_id);
    }

    fn get_storage_account_unwrap(&self, account_id: &AccountId) -> UserStorageUsage {
        match self.storage_accounts.get(account_id) {
            Some(storage_account) => storage_account,
            None => env::panic_str("account not registered"),
        }
    }

    pub fn add_to_deposit_pools_list(&mut self, account_id: &AccountId, pool_id: u32) {
        let mut deposited_pools = self.deposited_pools.get(account_id).unwrap_or(vec![]);
        if !deposited_pools.contains(&pool_id) {
            deposited_pools.push(pool_id);
            self.deposited_pools.insert(account_id, &deposited_pools);
        }
    }

    pub fn add_to_created_pools_list(&mut self, account_id: &AccountId, pool_id: u32) {
        let mut created_pools = self.created_pools.get(account_id).unwrap_or(vec![]);
        if !created_pools.contains(&pool_id) {
            created_pools.push(pool_id);
            self.created_pools.insert(account_id, &created_pools);
        }
    }

    pub fn add_to_borrow_pools_list(&mut self, account_id: &AccountId, pool_id: u32) {
        let mut borrow_pools = self.borrow_pools.get(account_id).unwrap_or(vec![]);
        if !borrow_pools.contains(&pool_id) {
            borrow_pools.push(pool_id);
            self.borrow_pools.insert(account_id, &borrow_pools);
        }
    }

    // fn internal_unwrap_account_or_revert(&self, account_id: &AccountId) -> AccountDeposit {
    //     match self.accounts.get(account_id) {
    //         Some(account_deposit) => account_deposit,
    //         None => {
    //             env::panic_str(format!("The account {} is not registered", &account_id).as_str())
    //         }
    //     }
    // }

    // pub fn internal_register_account(
    //     &mut self,
    //     account_id: &AccountId,
    //     amount: &Balance,
    // ) -> Balance {
    //     let init_storage = env::storage_usage();

    //     if !self.token.accounts.contains_key(account_id) {
    //         self.token.accounts.insert(account_id, &0u128);
    //     }

    //     if !self.accounts.contains_key(account_id) {
    //         let deposit_account = AccountDeposit {
    //             vaults: vec![],
    //             near_amount: U128(amount.clone()),
    //             storage_usage: 0,
    //         };
    //         self.accounts.insert(account_id, &deposit_account);
    //     } else {
    //         let mut deposit_account = self.get_account_info(account_id.clone());
    //         deposit_account.near_amount = U128(deposit_account.near_amount.0 + amount);
    //         self.accounts.insert(account_id, &deposit_account);
    //     }

    //     //insert all vaults, even empty
    //     let mut deposit_account = self.get_account_info(account_id.clone());
    //     if deposit_account.vaults.len() < self.token_list.len() {
    //         let mut i = deposit_account.vaults.len();
    //         let token_count = self.token_list.len();
    //         while i < token_count {
    //             let token_id = self.token_list[i].clone();
    //             let vault = Vault::new(&account_id.clone(), &token_id.clone());
    //             deposit_account.add_vault(&vault);
    //             i = i + 1;
    //         }
    //     }

    //     let storage_used = env::storage_usage() - init_storage;
    //     deposit_account.storage_usage += storage_used;
    //     self.accounts.insert(account_id, &deposit_account);
    //     self.assert_storage_usage(account_id);

    //     self.storage_available(account_id.clone()).0
    // }

    // fn deposit_to_vault(
    //     &mut self,
    //     collateral_token_id: &AccountId,
    //     collateral_amount: &Balance,
    //     account_id: &AccountId,
    // ) {
    //     let mut deposit_account = self.internal_unwrap_account_or_revert(account_id);
    //     //find the vault for collateral token
    //     let length = deposit_account.vaults.len();
    //     let i = deposit_account.get_vault_index(collateral_token_id.clone());
    //     if i < length {
    //         let mut vault = deposit_account.vaults[i].clone();
    //         vault.deposited = U128(vault.deposited.0 + collateral_amount.clone());
    //         vault.last_deposit = U128(collateral_amount.clone());
    //         deposit_account.vaults[i] = vault;
    //         self.accounts.insert(&account_id, &deposit_account);
    //     } else {
    //         let mut vault = Vault::new(&account_id, &collateral_token_id);
    //         vault.deposited = U128(vault.deposited.0 + collateral_amount.clone());
    //         vault.last_deposit = U128(collateral_amount.clone());
    //         deposit_account.add_vault(&vault);
    //         self.accounts.insert(&account_id, &deposit_account);
    //     }

    //     let mut token_info = self.supported_tokens.get(collateral_token_id).unwrap();
    //     token_info.total_deposit = U128(token_info.total_deposit.0 + collateral_amount);
    //     self.supported_tokens
    //         .insert(collateral_token_id, &token_info);
    // }
}

#[no_mangle]
pub fn upgrade() {
    // env::setup_panic_hook();

    // let contract: Contract = env::state_read().expect("Contract is not initialized");
    // contract.assert_governance();

    // const _MIGRATE_METHOD_NAME: &[u8; 7] = b"migrate";
    // const _UPDATE_GAS_LEFTOVER: Gas = Gas(5_000_000_000_000);

    // unsafe {
    //     // Load code into register 0 result from the input argument if factory call or from promise if callback.
    //     // sys::input(0);
    //     // // Create a promise batch to update current contract with code from register 0.
    //     // let promise_id = sys::promise_batch_create(
    //     //     env::current_account_id().as_bytes().len() as u64,
    //     //     env::current_account_id().as_bytes().as_ptr() as u64,
    //     // );
    //     // // Deploy the contract code from register 0.
    //     // sys::promise_batch_action_deploy_contract(promise_id, u64::MAX, 0);
    //     // // Call promise to migrate the state.
    //     // // Batched together to fail upgrade if migration fails.
    //     // sys::promise_batch_action_function_call(
    //     //     promise_id,
    //     //     MIGRATE_METHOD_NAME.len() as u64,
    //     //     MIGRATE_METHOD_NAME.as_ptr() as u64,
    //     //     0,
    //     //     0,
    //     //     0,
    //     //     (env::prepaid_gas() - env::used_gas() - UPDATE_GAS_LEFTOVER).0,
    //     // );
    //     // sys::promise_return(promise_id);
    // }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {}
