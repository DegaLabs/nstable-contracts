// mod governance;
// //mod storage;
// mod storage_impl;
// mod token_receiver;
// mod utils;
mod views;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap};
use near_sdk::json_types::{U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey,
    PanicOnDefault, Balance
};


const FEE_DIVISOR: u64 = 10000;

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    Blacklist,
    VerifiedPoolIdsByAccount,
    UnverifiedPoolIdsByAccount,
    VerifiedPoolIdsByToken,
    UnverifiedPoolIdsByToken
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

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Pool {
    pool_id: u32,
    owner_id: AccountId,
    token_id: AccountId,
    amount_for_sale: Balance,
    rate: Balance,     //how many token per one NEAR 
    soft_cap: Balance,
    hard_cap: Balance,
    buyers: LookupMap<AccountId, Balance>,
    buyer_list: Vec<AccountId>,
    token_claimed: LookupMap<AccountId, Balance>,
    sale_start_sec: u64,
    sale_end_sec: u64,
    sold_amount: Balance,
    verified_track: bool,
    is_token_deposited: bool
}

impl Pool {
    fn new(pool_id: u32, owner_id: AccountId , token_id: AccountId, amount_for_sale: Balance, rate: Balance, soft_cap: Balance, hard_cap: Balance, sale_start_sec: u64, sale_end_sec: u64, verified_track: bool) -> Self {
        Self {
            pool_id: pool_id.clone(), 
            owner_id: owner_id,
            token_id: token_id,
            amount_for_sale: amount_for_sale, 
            rate: rate, 
            soft_cap: soft_cap,
            hard_cap: hard_cap,
            buyers: LookupMap::new(format!("buyers{}{}", pool_id, verified_track).as_bytes()),
            buyer_list: vec![],
            token_claimed: LookupMap::new(format!("token_claimed{}{}", pool_id, verified_track).as_bytes()),
            sale_start_sec: sale_start_sec,
            sale_end_sec: sale_end_sec,
            sold_amount: 0u128,
            verified_track: verified_track,
            is_token_deposited: false
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct UserAccount {
    owner_id: AccountId,
    joint_verified_pool_ids: Vec<u32>,
    joint_unverified_pool_ids: Vec<u32>,
    total_near_amount: Balance
}

impl UserAccount {
    fn new (owner_id: AccountId) -> Self {
        Self {
            owner_id: owner_id,
            joint_verified_pool_ids: vec![],
            joint_unverified_pool_ids: vec![],
            total_near_amount: 0
        }
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    governance: AccountId,
    black_list: LookupMap<AccountId, BlackListStatus>,
    status: ContractStatus,
    token_list: Vec<AccountId>,
    fee: u64,
    verified_pools: Vec<Pool>,
    unverified_pools: Vec<Pool>,
    verified_pool_ids_by_account: LookupMap<AccountId, Vec<u32>>,
    unverified_pool_ids_by_account: LookupMap<AccountId, Vec<u32>>,
    verified_pool_ids_by_token: LookupMap<AccountId, Vec<u32>>,
    unverified_pool_ids_by_token: LookupMap<AccountId, Vec<u32>>,
    new_pool_creation_fee: Balance
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(governance: AccountId, fee: Option<u64>, new_pool_creation_fee: Option<U128>) -> Self {
        require!(!env::state_exists(), "Already initialized");
        let fee = fee.unwrap_or(1000);  //10% by default
        let new_pool_creation_fee = new_pool_creation_fee.unwrap_or(U128(10u128.pow(24 as u32)));
        let mut this = Self {
            governance: governance.clone(),
            black_list: LookupMap::new(StorageKey::Blacklist),
            status: ContractStatus::Working,
            token_list: vec![],
            fee: fee,
            verified_pools: vec![],
            unverified_pools: vec![],
            verified_pool_ids_by_account: LookupMap::new(StorageKey::VerifiedPoolIdsByAccount),
            unverified_pool_ids_by_account: LookupMap::new(StorageKey::UnverifiedPoolIdsByAccount),
            verified_pool_ids_by_token: LookupMap::new(StorageKey::VerifiedPoolIdsByToken),
            unverified_pool_ids_by_token: LookupMap::new(StorageKey::UnverifiedPoolIdsByToken),
            new_pool_creation_fee: new_pool_creation_fee.0
        };

        this
    }

    #[payable]
    pub fn create_new_unverified_pool(&mut self, token_id: AccountId, amount_for_sale: U128, rate: U128, soft_cap: U128, sale_start_sec: u64, sale_end_sec: u64) {
        let prev_storage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        let pool_len = self.unverified_pools.len() as u32;
        let hard_cap = 0 as Balance;
        let pool = Pool::new(pool_len.clone(), account_id.clone(), token_id.clone(), amount_for_sale.0, rate.0, soft_cap.0, hard_cap.clone(), sale_start_sec.clone(), sale_end_sec.clone(), false);

        self.unverified_pools.push(pool);
        let mut unverified_pools_by_token = self.unverified_pool_ids_by_token.get(&token_id).unwrap_or(vec![]);
        unverified_pools_by_token.push(pool_len.clone());
        self.unverified_pool_ids_by_token.insert(&token_id, &unverified_pools_by_token);

        let mut unverified_pools_by_account = self.unverified_pool_ids_by_account.get(&account_id).unwrap_or(vec![]);
        unverified_pools_by_account.push(pool_len.clone());
        self.unverified_pool_ids_by_account.insert(&account_id, &unverified_pools_by_account);
    }

    // fn measure_account_storage_usage(&mut self) {
    //     let mut initial_storage_usage = env::storage_usage();
    //     let tmp_account_id = AccountId::new_unchecked("a".repeat(64).to_string());
    //     let temp_account_deposit = AccountDeposit::default();
    //     self.accounts.insert(&tmp_account_id, &temp_account_deposit);
    //     self.base_storage_usage = env::storage_usage() - initial_storage_usage;
    //     self.token.accounts.insert(&tmp_account_id, &0u128);

    //     initial_storage_usage = env::storage_usage();
    //     let mut tmp_acc = self.accounts.get(&tmp_account_id).unwrap();
    //     let tmp_token_id = AccountId::new_unchecked("a".repeat(64).to_string());
    //     let vault = Vault {
    //         owner_id: tmp_account_id.clone(),
    //         token_id: tmp_token_id.clone(),
    //         deposited: U128(0),
    //         borrowed: U128(0),
    //         last_borrowed: U128(0),
    //         last_deposit: U128(0),
    //     };
    //     tmp_acc.vaults.push(vault);
    //     self.accounts.insert(&tmp_account_id, &tmp_acc);

    //     self.storage_usage_per_vault = env::storage_usage() - initial_storage_usage;

    //     self.token.accounts.remove(&tmp_account_id);
    //     self.accounts.remove(&tmp_account_id);
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

    fn abort_if_blacklisted(&self, account_id: AccountId) {
        if self.blacklist_status(&account_id) != BlackListStatus::Allowable {
            env::panic_str(&format!("Account '{}' is banned", account_id));
        }
    }
}

impl Contract {
    
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
