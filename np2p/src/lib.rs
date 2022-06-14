mod governance;
mod oracle;
//mod storage;
mod storage_impl;
//mod token_receiver;
//mod utils;
mod account_deposit;
mod pool;
mod token_receiver;
mod utils;
mod views;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, require, AccountId, Balance, BorshStorageKey,
    PanicOnDefault, Promise, StorageUsage,
};

use account_deposit::AccountDeposit;
use pool::{new_pool_default, Pool};

pub type AssetId = AccountId;

use oracle::{Price, PriceData};
use std::fmt::Debug;

use views::U256;

const COLLATERAL_RATIO_DIVISOR: u128 = 10000;
const UTILIZATION_DIVISOR: u128 = 10000;
const INTEREST_RATE_DIVISOR: u128 = 10000;
const ACC_INTEREST_PER_SHARE_MULTIPLIER: u128 = 10u128.pow(8 as u32);
const SECONDS_PER_YEAR: u128 = 365 * 86400;
const LIQUIDATION_BONUS_DIVISOR: u128 = 10000;
const LIQUIDATION_MARGINAL_DIVISOR: u128 = 10000;

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
    AccountDeposit {
        pool_id: u32,
        account_id: AccountId
    },
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
    account_list: Vec<AccountId>,
    liquidation_marginal: u64, //how mujch in terms of % the treasury got
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
            account_list: vec![],
            liquidation_marginal: 5000,
        };

        this.measure_account_storage_usage();
        this
    }

    pub fn set_foundation_id(&mut self, account_id: AccountId) {
        self.assert_governance();
        self.foundation_id = account_id;
    }

    #[payable]
    pub fn add_new_supported_tokens(&mut self, token_ids: Vec<AccountId>, decimals: Vec<u8>) {
        self.assert_governance();
        let prev_storage = env::storage_usage();
        for i in 0..token_ids.len() {
            if self.is_token_supported(&token_ids[i]) {
                log!("token already supported {}", token_ids[i]);
                continue;
            }
            self.supported_tokens.insert(
                &token_ids[i],
                &TokenInfo {
                    token_id: token_ids[i].clone(),
                    decimals: decimals[i],
                },
            );
            self.token_list.push(token_ids[i].clone());
        }
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

        self.storage_usage_add_pool = self.get_storage_usage(initial_storage_usage);
        log!("accessing to pool 0");
        initial_storage_usage = env::storage_usage();
        let tmp_pool = &mut self.pools[0];
        log!("accessing to pool 0 success");
        let tmp_account_id2 = AccountId::new_unchecked("e".repeat(64).to_string());
        log!("start internal_register_account to pool 0");
        tmp_pool.internal_register_account_if_not(&tmp_account_id2);
        log!("internal_register_account to pool 0");
        self.add_to_deposit_pools_list(&tmp_account_id2, 0u32);
        self.account_list.push(tmp_account_id2.clone());

        self.storage_usage_join_pool = self.get_storage_usage(initial_storage_usage);
        log!("storage_usage_join_pool to pool 0");
        //clean out
        self.account_list.pop();
        log!("self.account_list.pop()");
        self.deposited_pools.remove(&tmp_account_id);
        self.deposited_pools.remove(&tmp_account_id2);
        self.created_pools.remove(&tmp_account_id);
        self.token_to_list_lend_pools.remove(&tmp_lend_id);
        self.token_to_list_collateral_pools
            .remove(&tmp_collateral_id);
        self.pools.remove(0);
    }

    #[payable]
    pub fn push_to_account_list(&mut self, account_ids: Vec<AccountId>) {
        self.assert_governance();
        let prev_storage = env::storage_usage();
        for account_id in account_ids {
            self.account_list.push(account_id.clone());
        }
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

    #[payable]
    pub fn borrow(&mut self, pool_id: u32, borrow_amount: U128) -> Promise {
        require!(borrow_amount.0 > 0, "borrow_amount > 0");
        let prev_storage = env::storage_usage();
        // Select target account.
        let account_id = env::predecessor_account_id();

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
            pool.internal_borrow(
                &account_id,
                &borrow_amount.0,
                &lend_token_info,
                &lend_token_price,
                &collateral_token_info,
                &collateral_token_price,
            );
        }

        self.add_to_borrow_pools_list(&account_id, pool_id.clone());
        self.verify_storage(&account_id, prev_storage, Some(env::attached_deposit()));
        self.internal_send_tokens(
            pool_id,
            &lend_token_info.token_id,
            &account_id,
            borrow_amount.0.clone(),
        )
    }

    #[payable]
    pub fn create_new_pool(
        &mut self,
        lend_token_id: AssetId,
        collateral_token_id: AssetId,
        min_cr: Option<u64>,
        max_utilization: Option<u64>,
        min_lend_token_deposit: Option<U128>,
        min_lend_token_borrow: Option<U128>,
        fixed_interest_rate: Option<u64>,
        liquidation_bonus: Option<u64>,
    ) {
        let attached_deposit = env::attached_deposit();
        require!(
            attached_deposit >= self.pool_creation_fee,
            "!pool_creation_fee"
        );

        let min_cr = min_cr.unwrap_or(15000);
        let max_utilization = max_utilization.unwrap_or(9000);
        let min_lend_token_deposit = min_lend_token_deposit.unwrap_or(U128(0));
        let min_lend_token_borrow = min_lend_token_borrow.unwrap_or(U128(0));
        let fixed_interest_rate = fixed_interest_rate.unwrap_or(1000);
        let liquidation_bonus = liquidation_bonus.unwrap_or(1000);

        self.abort_if_unsupported_token(lend_token_id.clone());
        self.abort_if_unsupported_token(collateral_token_id.clone());

        //TODO: add up to total pool creation fee

        let account_id = env::predecessor_account_id();
        let prev_storage = env::storage_usage();
        let pool_id = self.pools.len() as u32;
        log!("creating pool {}", pool_id);
        let mut pool = Pool::new(
            pool_id.clone(),
            account_id.clone(),
            lend_token_id.clone(),
            collateral_token_id.clone(),
            min_cr,
            max_utilization,
            min_lend_token_deposit.0,
            min_lend_token_borrow.0,
            fixed_interest_rate,
            liquidation_bonus,
        );
        pool.internal_register_account_if_not(&account_id);
        pool.internal_register_account_if_not(&self.foundation_id);

        self.pools.push(pool);

        let mut token_to_list_lend_pools = self
            .token_to_list_lend_pools
            .get(&lend_token_id)
            .unwrap_or(vec![]);
        token_to_list_lend_pools.push(pool_id.clone());
        self.token_to_list_lend_pools
            .insert(&lend_token_id, &token_to_list_lend_pools);

        let mut token_to_list_collateral_pools = self
            .token_to_list_collateral_pools
            .get(&collateral_token_id)
            .unwrap_or(vec![]);
        token_to_list_collateral_pools.push(pool_id.clone());
        self.token_to_list_collateral_pools
            .insert(&collateral_token_id, &token_to_list_collateral_pools);

        self.add_to_created_pools_list(&account_id, pool_id.clone());
        self.add_to_deposit_pools_list(&account_id, pool_id.clone());
        self.verify_storage(
            &account_id,
            prev_storage,
            Some(attached_deposit - self.pool_creation_fee),
        );
        log!("Done create pool {}", pool_id);
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
            pool.internal_withdraw_from_account(
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

    #[payable]
    pub fn pay_loan(&mut self, pool_id: u32, amount: U128) {
        let account_id = env::predecessor_account_id();
        assert_one_yocto();
        self.abort_if_pause();
        self.abort_if_blacklisted(account_id.clone());
        self.abort_if_pool_id_valid(pool_id.clone() as usize);
        {
            let pool = &mut self.pools[pool_id as usize];
            pool.internal_pay_loan(&account_id, amount.0);
        }
    }

    #[payable]
    pub fn liquidate(
        &mut self,
        pool_id: u32,
        liquidated_account_id: AccountId,
        liquidated_borrow_amount: U128,
    ) {
        let prev_usage = env::storage_usage();
        let liquidator_account_id = env::predecessor_account_id();
        self.abort_if_pause();
        self.abort_if_blacklisted(liquidator_account_id.clone());
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
            pool.internal_liquidate(
                liquidated_account_id.clone(),
                liquidated_borrow_amount.0,
                liquidator_account_id.clone(),
                &lend_token_info,
                &lend_token_price,
                &collateral_token_info,
                &collateral_token_price,
                self.foundation_id.clone(),
                self.liquidation_marginal,
            );
        }

        self.verify_storage(
            &liquidator_account_id,
            prev_usage,
            Some(env::attached_deposit()),
        );
    }

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
    pub fn internal_pay_loan(
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
        require!(
            lend_token_id.clone() == pool.lend_token_id,
            "invalid token lend"
        );

        let pool = &mut self.pools[pool_id as usize];
        pool.internal_pay_loan(account_id, pay_amount.0);
    }

    pub fn internal_deposit(
        &mut self,
        pool_id: u32,
        account_id: &AccountId,
        token_id: &AssetId,
        amount: Balance,
    ) {
        self.abort_if_pool_id_valid(pool_id.clone() as usize);

        let prev_storage = env::storage_usage();
        let pool = &mut self.pools[pool_id.clone() as usize];
        pool.internal_register_account_if_not(account_id);
        pool.internal_deposit(account_id, token_id, amount.clone());
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
        let mut storage_account = self.get_storage_account_unwrap(account_id);
        storage_account.storage_usage += self.get_storage_usage(prev_storage);
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

#[cfg(test)]
mod tests {
    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::serde_json;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, Balance};
    //use near_sdk_sim::to_yocto;

    use super::*;
    use token_receiver::TokenReceiverMessage;
    const ONE_NEAR: Balance = 10u128.pow(24);

    fn get_account(id: u32) -> AccountId {
        AccountId::new_unchecked(format!("id-{}", id))
    }

    /// Creates contract and a pool with tokens with 0.3% of total fee.
    fn setup_contract(
        governance: AccountId,
        foundation: AccountId,
        price_feeder: AccountId,
    ) -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(get_account(0)).build());
        let contract = Contract::new(governance, foundation, Some(price_feeder));
        (context, contract)
    }

    // fn deposit_tokens(
    //     context: &mut VMContextBuilder,
    //     contract: &mut Contract,
    //     account_id: ValidAccountId,
    //     token_amounts: Vec<(ValidAccountId, Balance)>,
    // ) {
    //     if contract.storage_balance_of(account_id.clone()).is_none() {
    //         testing_env!(context
    //             .predecessor_account_id(account_id.clone())
    //             .attached_deposit(to_yocto("1"))
    //             .build());
    //         contract.storage_deposit(None, None);
    //     }
    //     testing_env!(context
    //         .predecessor_account_id(account_id.clone())
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     let tokens = token_amounts
    //         .iter()
    //         .map(|(token_id, _)| token_id.clone().into())
    //         .collect();
    //     testing_env!(context.attached_deposit(1).build());
    //     contract.register_tokens(tokens);
    //     for (token_id, amount) in token_amounts {
    //         testing_env!(context
    //             .predecessor_account_id(token_id)
    //             .attached_deposit(1)
    //             .build());
    //         contract.ft_on_transfer(account_id.clone(), U128(amount), "".to_string());
    //     }
    // }

    fn add_supported_tokens(context: &mut VMContextBuilder, contract: &mut Contract) {
        let lend_token_id = get_account(3);
        let collateral_token_id = get_account(4);
        testing_env!(context
            .attached_deposit(ONE_NEAR)
            .predecessor_account_id(get_account(0))
            .build());
        contract.add_new_supported_tokens(
            [lend_token_id.clone(), collateral_token_id.clone()].to_vec(),
            [8, 18].to_vec(),
        );
    }

    #[test]
    fn test_basics() {
        let governance = get_account(0);
        let foundation = get_account(1);
        let price_feeder = get_account(2);
        let (mut context, mut contract) =
            setup_contract(governance.clone(), foundation.clone(), price_feeder.clone());
        // add liquidity of (1,2) tokens
        assert_eq!(contract.governance, governance);
    }

    #[test]
    fn create_new_pools() {
        let governance = get_account(0);
        let foundation = get_account(1);
        let price_feeder = get_account(2);
        let lend_token_id = get_account(3);
        let collateral_token_id = get_account(4);
        log!("accounts {}", get_account(0));
        let (mut context, mut contract) =
            setup_contract(governance.clone(), foundation.clone(), price_feeder.clone());
        add_supported_tokens(&mut context, &mut contract);
        for i in 0..2 {
            testing_env!(context
                .attached_deposit(11 * ONE_NEAR)
                .predecessor_account_id(get_account(i))
                .build());
            contract.storage_deposit(None, None);
            contract.create_new_pool(
                lend_token_id.clone(),
                collateral_token_id.clone(),
                None,
                None,
                None,
                None,
                None,
                None,
            );

            for j in 1..3 {
                testing_env!(context
                    .attached_deposit(1 * ONE_NEAR)
                    .predecessor_account_id(get_account(j))
                    .build());
                contract.storage_deposit(None, None);
                //deposit
                testing_env!(context
                    .attached_deposit(1 * ONE_NEAR)
                    .predecessor_account_id(collateral_token_id.clone())
                    .build());
                log!("Depositing to pool {}, token {}", i, collateral_token_id.clone());
                let message = TokenReceiverMessage::Deposit { pool_id: i };
                contract.ft_on_transfer(
                    get_account(j),
                    U128(100000000),
                    serde_json::to_string(&message).unwrap(),
                );
                log!("Depositing to pool {}, token {}", i, lend_token_id.clone());
                testing_env!(context
                    .attached_deposit(1 * ONE_NEAR)
                    .predecessor_account_id(lend_token_id.clone())
                    .build());
                contract.ft_on_transfer(
                    get_account(j),
                    U128(200000000),
                    serde_json::to_string(&message).unwrap(),
                );
                log!("reading deposits from pool {}", i);
                let pool = contract.pools.get(i as usize).unwrap();
                let account_deposit = pool.get_account_deposit(&get_account(j));
                assert_eq!(
                    account_deposit
                        .deposits
                        .get(&collateral_token_id)
                        .unwrap_or(0),
                    100000000
                );
                assert_eq!(
                    account_deposit.deposits.get(&lend_token_id).unwrap_or(0),
                    200000000
                );
            }
        }
    }
}
