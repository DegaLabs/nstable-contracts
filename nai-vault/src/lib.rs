#![deny(warnings)]
mod governance;
mod oracle;
//mod storage;
mod mint;
mod views;
mod token_receiver;
mod storage_impl;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, require, AccountId, Balance,
    BorshStorageKey, Gas, PanicOnDefault, Promise, StorageUsage,
};

use mint::ext_self;
use std::fmt::Debug;

use oracle::{ExchangeRate, PriceData};

const NO_DEPOSIT: Balance = 0;
const GAS_FOR_MINT: Gas = Gas(5_000_000_000_000);
const GAS_FOR_MINT_CALLBACK: Gas = Gas(20_000_000_000_000);

//const USN_DECIMALS: u8 = 18;
//const GAS_FOR_BUY_PROMISE: Gas = Gas(10_000_000_000_000);
//const GAS_FOR_SELL_PROMISE: Gas = Gas(15_000_000_000_000);
//const GAS_FOR_RETURN_VALUE_PROMISE: Gas = Gas(5_000_000_000_000);

//const MAX_SPREAD: Balance = 50_000; // 0.05 = 5%
//const SPREAD_DECIMAL: u8 = 6;
//const SPREAD_MAX_SCALER: f64 = 0.4;

//const COLLATERAL_RATIO_DIVISOR: u64 = 10000;

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    Blacklist,
    SupportedTokens,
    Accounts,
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExpectedRate {
    pub multiplier: U128,
    pub slippage: U128,
    pub decimals: u8,
}

impl Default for ExpectedRate {
    fn default() -> ExpectedRate {
        ExpectedRate {
            multiplier: U128(0),
            slippage: U128(0),
            decimals: 0,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Vault {
    owner_id: AccountId,
    token_id: AccountId,
    deposited: Balance,
    borrowed: Balance,
    last_deposit: Balance,
    last_borrowed: Balance
}

impl Default for Vault {
    fn default() -> Vault {
        Vault {
            owner_id: AccountId::new_unchecked("".to_string()),
            token_id: AccountId::new_unchecked("".to_string()),
            deposited: 0,
            borrowed: 0,
            last_borrowed: 0,
            last_deposit: 0
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountDeposit {
    pub vaults: Vec<Vault>,
    pub near_amount: Balance,
    pub storage_usage: StorageUsage,
}

impl Default for AccountDeposit {
    fn default() -> AccountDeposit {
        AccountDeposit {
            vaults: vec![],
            near_amount: 0,
            storage_usage: 0,
        }
    }
}

impl AccountDeposit {
    pub fn get_vault_index(&self, collateral_token_id: AccountId) -> usize {
        let length = self.vaults.len();
        let mut i = 0;
        while i < length {
            let vault = &self.vaults[i];
            if vault.token_id == collateral_token_id.clone() {
                break;
            }
            i = i + 1;
        }
        i
    }

    pub fn get_vault(&self, collateral_token_id: AccountId) -> Vault {
        let length = self.vaults.len();
        let i = self.get_vault_index(collateral_token_id.clone());

        if i < length {
            return self.vaults[i].clone();
        }
        return Vault::default();
    }

    pub fn add_vault(&mut self, vault: &Vault) {
        self.vaults.push(vault.clone());
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenInfo {
    pub token_id: AccountId,
    pub collateral_ratio: u64,
    pub total_deposit: Balance,
    pub total_borrowed: Balance, //NAI balance
    pub decimals: u8,
    pub generated_fees: Balance,
}

impl Default for TokenInfo {
    fn default() -> TokenInfo {
        TokenInfo {
            token_id: AccountId::new_unchecked("".to_string()),
            collateral_ratio: 0,
            total_deposit: 0,
            total_borrowed: 0,
            decimals: 0,
            generated_fees: 0,
        }
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    governance: AccountId,
    black_list: LookupMap<AccountId, BlackListStatus>,
    status: ContractStatus,
    supported_tokens: LookupMap<AccountId, TokenInfo>,
    token_list: Vec<AccountId>,
    accounts: LookupMap<AccountId, AccountDeposit>,
    total_nai_borrowed: Balance,
    total_generated_fees: Balance,
    price_data: PriceData,
    price_feeder: AccountId,
    nai_token_id: AccountId,
    base_storage_usage: StorageUsage,
    storage_usage_per_vault: StorageUsage,
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract owned by the given `owner_id` with default metadata.
    #[init]
    pub fn new(
        governance: AccountId,
        price_feeder: Option<AccountId>,
        nai_token_id: Option<AccountId>,
    ) -> Self {
        let price_feeder = price_feeder.unwrap_or(AccountId::new_unchecked("".to_string()));
        let nai_token_id = nai_token_id.unwrap_or(AccountId::new_unchecked("".to_string()));
        let mut this = Self {
            governance: governance.clone(),
            black_list: LookupMap::new(StorageKey::Blacklist),
            status: ContractStatus::Working,
            supported_tokens: LookupMap::new(StorageKey::SupportedTokens),
            accounts: LookupMap::new(StorageKey::Accounts),
            total_nai_borrowed: 0,
            total_generated_fees: 0,
            price_data: PriceData::default(),
            price_feeder: price_feeder,
            nai_token_id: nai_token_id,
            base_storage_usage: 0,
            storage_usage_per_vault: 0,
            token_list: vec![]
        };
        this.measure_account_storage_usage();
        this
    }

    fn measure_account_storage_usage(&mut self) {
        let mut initial_storage_usage = env::storage_usage();
        let tmp_account_id = AccountId::new_unchecked("a".repeat(64).to_string());
        let temp_account_deposit = AccountDeposit::default();
        self.accounts.insert(&tmp_account_id, &temp_account_deposit);
        self.base_storage_usage = env::storage_usage() - initial_storage_usage;

        initial_storage_usage = env::storage_usage();
        let mut tmp_acc = self.accounts.get(&tmp_account_id).unwrap();
        let tmp_token_id = AccountId::new_unchecked("a".repeat(64).to_string());
        let vault = Vault {
            owner_id: tmp_account_id.clone(),
            token_id: tmp_token_id.clone(),
            deposited: 0,
            borrowed: 0,
            last_borrowed: 0,
            last_deposit: 0
        };
        tmp_acc.vaults.push(vault);
        self.accounts.insert(&tmp_account_id, &tmp_acc);

        self.storage_usage_per_vault = env::storage_usage() - initial_storage_usage;

        self.accounts.remove(&tmp_account_id);
    }

    pub fn add_new_collateral_token(
        &mut self,
        token_id: AccountId,
        decimals: u8,
        collateral_ratio: u64,
    ) {
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
                collateral_ratio: collateral_ratio,
                decimals: decimals,
                total_deposit: 0,
                total_borrowed: 0,
                generated_fees: 0,
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

    pub fn destroy_black_funds(&mut self, _account_id: &AccountId) {
        // self.assert_owner();
        // assert_eq!(self.blacklist_status(&account_id), BlackListStatus::Banned);
        // let black_balance = self.ft_balance_of(account_id.clone());
        // if black_balance.0 <= 0 {
        //     env::panic_str("The account doesn't have enough balance");
        // }
        // self.token.accounts.insert(account_id, &0u128);
        // self.token.total_supply = self
        //     .token
        //     .total_supply
        //     .checked_sub(u128::from(black_balance))
        //     .expect("Failed to decrease total supply");
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
    fn _assert_exchange_rate(&self, actual: &ExchangeRate, expected: &ExpectedRate) {
        let slippage = u128::from(expected.slippage);
        let multiplier = u128::from(expected.multiplier);
        let start = multiplier.saturating_sub(slippage);
        let end = multiplier.saturating_add(slippage);
        assert_eq!(
            actual.decimals(),
            expected.decimals,
            "Slippage error: different decimals"
        );

        if !(start..=end).contains(&actual.multiplier()) {
            env::panic_str(&format!(
                "Slippage error: fresh exchange rate {} is out of expected range {} +/- {}",
                actual.multiplier(),
                multiplier,
                slippage
            ));
        }
    }

    fn internal_unwrap_account_or_revert(&self, account_id: &AccountId) -> AccountDeposit {
        match self.accounts.get(account_id) {
            Some(account_deposit) => account_deposit,
            None => {
                env::panic_str(format!("The account {} is not registered", &account_id).as_str())
            }
        }
    }

    pub fn internal_register_account(&mut self, account_id: &AccountId, amount: &Balance) -> Balance {
        let init_storage = env::storage_usage();
        if !self.accounts.contains_key(account_id) {
            let deposit_account = AccountDeposit {
                vaults: vec![],
                near_amount: amount.clone(),
                storage_usage: 0,
            };
            self.accounts.insert(account_id, &deposit_account);
        } else {
            let mut deposit_account = self.get_account_info(account_id.clone());
            deposit_account.near_amount += amount;
            self.accounts.insert(account_id, &deposit_account);
        }

        //insert all vaults, even empty
        let mut deposit_account = self.get_account_info(account_id.clone());
        if deposit_account.vaults.len() < self.token_list.len() {
            let mut i = deposit_account.vaults.len();
            let token_count = self.token_list.len();
            while i < token_count {
                let token_id = self.token_list[i].clone();
                let mut vault = Vault::default();
                 
                vault.owner_id = account_id.clone();
                vault.token_id = token_id.clone();
                deposit_account.add_vault(&vault);
                i = i + 1;
            }
        }

        let storage_used = env::storage_usage() - init_storage;
        deposit_account.storage_usage += storage_used;
        self.accounts.insert(account_id, &deposit_account);
        
        self.assert_storage_usage(account_id);

        self.storage_available(account_id.clone())
    }

    fn deposit_to_vault(
        &mut self,
        collateral_token_id: &AccountId,
        collateral_amount: &Balance,
        account_id: &AccountId,
    ) {
        let mut deposit_account = self.internal_unwrap_account_or_revert(account_id);
        //find the vault for collateral token
        let length = deposit_account.vaults.len();
        let i = deposit_account.get_vault_index(collateral_token_id.clone());
        if i < length {
            let mut vault = deposit_account.vaults[i].clone();
            vault.deposited += collateral_amount.clone();
            vault.last_deposit = collateral_amount.clone();
            deposit_account.vaults[i] = vault;
            self.accounts.insert(&account_id, &deposit_account);
        } else {
            let mut vault = Vault::default();
            vault.deposited += collateral_amount.clone();
            vault.last_deposit = collateral_amount.clone();
            deposit_account.add_vault(&vault);
            self.accounts.insert(&account_id, &deposit_account);
        }

        let mut token_info = self.supported_tokens.get(collateral_token_id).unwrap_or_default();
        token_info.total_deposit += collateral_amount;
        self.supported_tokens.insert(collateral_token_id, &token_info);
    }

    /// Asserts there is sufficient amount of $NEAR to cover storage usage.
    pub fn assert_storage_usage(&self, account_id: &AccountId) {
        let account_deposit = self.get_account_info(account_id.clone());
        assert!(
            self.compute_storage_usage_near(account_id.clone()) <= account_deposit.near_amount,
            "{}",
            "insufficient near deposit"
        );
    }

    pub fn borrow(
        &mut self,
        collateral_token_id: &AccountId,
        collateral_amount: Balance,
        borrow_amount: Balance,
        to: AccountId,
    ) {
        // Select target account.
        let account = to.clone();

        self.abort_if_pause();
        self.abort_if_blacklisted(account.clone());
        require!(
            self.is_token_supported(collateral_token_id),
            "unsupported token"
        );

        let near = env::attached_deposit();

        let prev_usage = env::storage_usage();


        //let exchange_rate = self.get_exchange_rate(collateral_token_id);

        //self.assert_exchange_rate(&exchange_rate, &expected);

        let mut borrowable = self.internal_compute_max_borrowable_amount(
            collateral_token_id.clone(),
            collateral_amount.clone(),
        );

        require!(borrow_amount <= borrowable, format!("cannot borrow more than {}", borrowable));

        borrowable = borrow_amount;

        self.deposit_to_vault(collateral_token_id, &collateral_amount, &account);

        let storage_used = env::storage_usage() - prev_usage;
        let mut account_deposit = self.get_account_info(account.clone());
        account_deposit.storage_usage += storage_used;
        account_deposit.near_amount += near;
        self.accounts.insert(&account, &account_deposit);

        self.assert_storage_usage(&account);

        self.call_mint(account.clone(), borrowable.clone())
            .then(ext_self::mint_callback(
                collateral_token_id.clone(),
                collateral_amount.clone(),
                account.clone(),
                borrowable.clone(),
                env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_MINT_CALLBACK,
            ));
    }

    pub fn finish_borrow(
        &mut self,
        collateral_token_id: AccountId, _collateral_amount: Balance, account_id: AccountId, borrowed: Balance, actual_received: Balance
    ) -> Balance {
        if actual_received == 0 {
            //do nothing
        } else {
            //save actual received to vault
            let mut deposit_account = self.internal_unwrap_account_or_revert(&account_id);
            let mut vault = deposit_account.get_vault(collateral_token_id.clone());
            let i = deposit_account.get_vault_index(collateral_token_id.clone());
            vault.borrowed += actual_received;
            vault.last_borrowed = actual_received;
            deposit_account.vaults[i] = vault;
            self.accounts.insert(&account_id, &deposit_account);

            let mut token_info = self.supported_tokens.get(&collateral_token_id).unwrap_or_default();
            token_info.total_borrowed += actual_received;
            let fee = borrowed - actual_received;
            token_info.generated_fees += fee.clone();
            self.total_generated_fees += fee.clone();
            self.total_nai_borrowed += actual_received;
            self.supported_tokens.insert(&collateral_token_id, &token_info);
        }

        actual_received
    }

    /// Sells USN tokens getting NEAR tokens.
    /// Return amount of purchased NEAR tokens.
    // pub fn liquidate(&mut self, amount: U128, expected: Option<ExpectedRate>) -> Promise {
    //     // assert_one_yocto();
    //     // self.abort_if_pause();
    //     // self.abort_if_blacklisted();

    //     // let amount = Balance::from(amount);

    //     // if amount == 0 {
    //     //     env::panic_str("Not allowed to sell 0 tokens");
    //     // }

    //     // let account = env::predecessor_account_id();

    //     // Oracle::get_exchange_rate_promise().then(ext_self::sell_with_price_callback(
    //     //     account,
    //     //     amount.into(),
    //     //     expected,
    //     //     env::current_account_id(),
    //     //     NO_DEPOSIT,
    //     //     GAS_FOR_SELL_PROMISE,
    //     // ))
    // }

    fn _finish_liquidate(
        &mut self,
        _account: AccountId,
        _amount: Balance,
        _expected: Option<ExpectedRate>,
        _rate: ExchangeRate,
    ) -> Balance {
        0
        // if let Some(expected) = expected {
        //     Self::assert_exchange_rate(&rate, &expected);
        // }

        // let mut sell_amount = U256::from(amount);

        // if account != self.owner_id {
        //     // Commission.
        //     let spread_denominator = 10u128.pow(SPREAD_DECIMAL as u32);
        //     let commission_usn =
        //         U256::from(amount) * U256::from(self.spread_u128(amount)) / spread_denominator;
        //     let commission_near = commission_usn
        //         * U256::from(10u128.pow(u32::from(rate.decimals() - USN_DECIMALS)))
        //         / rate.multiplier();
        //     self.commission.usn += commission_usn.as_u128();
        //     self.commission.near += commission_near.as_u128();

        //     sell_amount -= commission_usn;
        // }

        // // Make exchange: USN -> NEAR.
        // let deposit = sell_amount
        //     * U256::from(10u128.pow(u32::from(rate.decimals() - USN_DECIMALS)))
        //     / rate.multiplier();

        // // Here we don't expect too big deposit. Otherwise, panic.
        // let deposit = deposit.as_u128();

        // self.token.internal_withdraw(&account, amount);

        // event::emit::ft_burn(&account, amount, None);

        // deposit
    }
}

#[no_mangle]
pub fn upgrade() {
    env::setup_panic_hook();

    let contract: Contract = env::state_read().expect("Contract is not initialized");
    contract.assert_governance();

    const _MIGRATE_METHOD_NAME: &[u8; 7] = b"migrate";
    const _UPDATE_GAS_LEFTOVER: Gas = Gas(5_000_000_000_000);

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
