use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, near_bindgen, require, AccountId, Balance,
    BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue, StorageUsage,
};

const DATA_IMAGE_SVG_ICON: &str =
    "data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iVVRGLTgiPz4KPHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCAzMiAzMiIgdmVyc2lvbj0iMS4xIiB2aWV3Qm94PSIwIDAgMzIgMzIiIHhtbDpzcGFjZT0icHJlc2VydmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+CjxzdHlsZSB0eXBlPSJ0ZXh0L2NzcyI+Cgkuc3Q2e2ZpbGw6I0ZGRkZGRjt9Cgkuc3Q1OXtmaWxsOiM4NjQ3OUY7fQo8L3N0eWxlPgo8Y2lyY2xlIGNsYXNzPSJzdDU5IiBjeD0iMTYiIGN5PSIxNiIgcj0iMTYiLz4KPHBhdGggY2xhc3M9InN0NiIgZD0ibTIzLjU1IDguNzNjLTAuMDQtMC4wMi0wLjA3LTAuMDQtMC4xMS0wLjA2bC0wLjE2LTAuMDl2MC4wMWMtMS4zOC0wLjY2LTMuMDMtMC4zMi00LjA0IDAuODQtMC4yMiAwLjI1LTAuNDEgMC41Mi0wLjU3IDAuODFsLTAuMDEgMC4wMi0wLjA4IDAuMTQtMC4wMSAwLjAyLTAuODEgMS40OC01LjMxLTMuMDVjLTEuMTYtMC42Ny0yLjU0LTAuNjctMy43LTAuMDMtMS4yMiAwLjY4LTEuOTQgMi0xLjk0IDMuMzl2OC4yYzAgMS4xNyAwLjYzIDIuMjYgMS42NCAyLjg2IDAuMDMgMC4wMiAwLjA3IDAuMDQgMC4xIDAuMDZsMC4xNyAwLjA5YzAuNDcgMC4yMiAwLjk3IDAuMzMgMS40NyAwLjMzIDAuOTYgMCAxLjkxLTAuNDEgMi41Ny0xLjE4IDAuMjItMC4yNSAwLjQxLTAuNTIgMC41Ny0wLjgxbDAuMDEtMC4wMiAwLjA4LTAuMTQgMC4wMS0wLjAxIDAuODEtMS40OCA1LjMxIDMuMDVjMC41OSAwLjM0IDEuMjMgMC41MSAxLjg4IDAuNTEgMC42MiAwIDEuMjUtMC4xNiAxLjgyLTAuNDcgMS4yMi0wLjY4IDEuOTUtMS45OSAxLjk1LTMuMzl2LTguMmMtMC4wMS0xLjE5LTAuNjQtMi4yOS0xLjY1LTIuODh6bS0xMC43NyAxMi4yNi0wLjAzIDAuMDYtMC40NSAwLjgzYy0wLjM0IDAuNjItMC45NyAwLjk5LTEuNjcgMWgtMC4wMWMtMC43MSAwLTEuMzMtMC4zNy0xLjY3LTAuOTgtMC4xLTAuMTgtMC4xNy0wLjM3LTAuMjItMC41OC0wLjA0LTAuMTctMC4wNi0wLjM0LTAuMDYtMC41MXYtMC43OGMwLTAuMTYgMC4wMi0wLjMyIDAuMDYtMC40OSAwLjA4LTAuMzQgMC4yNS0wLjY1IDAuNDctMC45IDAuNDktMC41NSAxLjMyLTAuNjggMS45Ni0wLjMxbDIuMzUgMS4zNS0wLjczIDEuMzF6bS0wLjkyLTQuMzFjLTEuMDEtMC41Ny0yLjI1LTAuNDYtMy4xNCAwLjI3di01LjE0YzAtMC42MSAwLjMyLTEuMTYgMC44NC0xLjQ3IDAuNTMtMC4zMSAxLjE2LTAuMzEgMS43LTAuMDJsNS42NCAzLjE3LTIuNTEgNC42MS0yLjUzLTEuNDJ6bTExLjQyIDMuNWMwIDAuNjEtMC4zMiAxLjE2LTAuODQgMS40Ny0wLjUzIDAuMzEtMS4xNiAwLjMxLTEuNyAwLjAybC01LjY0LTMuMTcgMi41MS00LjYxIDIuNTMgMS40MmMwLjQzIDAuMjQgMC45IDAuMzYgMS4zOCAwLjM2IDAuNjMgMCAxLjI1LTAuMjIgMS43Ny0wLjYzdjUuMTR6bTAuMDYtOC4yMWMwIDAuMTYtMC4wMiAwLjMyLTAuMDYgMC40OS0wLjA4IDAuMzQtMC4yNSAwLjY1LTAuNDcgMC45LTAuNDkgMC41NS0xLjMyIDAuNjgtMS45NiAwLjMxbC0yLjM1LTEuMzUgMS4yLTIuMjFjMC4zNC0wLjYyIDAuOTctMC45OSAxLjY3LTFoMC4wMWMwLjcxIDAgMS4zMyAwLjM3IDEuNjcgMC45OCAwLjEgMC4xOCAwLjE3IDAuMzcgMC4yMiAwLjU4IDAuMDQgMC4xNyAwLjA2IDAuMzQgMC4wNiAwLjUxdjAuNzl6Ii8+Cjwvc3ZnPgo=";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
    Deposits,
}

use crate::errors::*;
use types::*;
use views::current_time_sec;

mod errors;
mod ft_functions;
mod owner;
mod storage_impl;
mod token_receiver;
mod types;
mod views;

const MINDAYS: u64 = 7;
const MAXDAYS: u64 = 3 * 365;
const MAXTIME: u64 = MAXDAYS * 86400;
const MAX_WITHDRAWAL_PENALTY: u64 = 50000; //50%
const PRECISION: u64 = 100000; // 5 decimals

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    /// Account of the owner.
    pub owner_id: AccountId,
    pub locked_token: AccountId,
    pub locked_token_name: String,
    pub locked_token_decimals: u8,
    pub penalty_collector: AccountId,
    pub min_locked_amount: Balance,
    pub early_withdraw_penalty_rate: u64,
    pub deposits: LookupMap<AccountId, Balance>,
    pub total_deposit: Balance,
    pub total_locked: Balance,
    pub voting_power_supply: Balance,
    pub tokens: NonFungibleToken,
    pub metadata: LazyOption<NFTContractMetadata>,
    pub register_storage_usage: StorageUsage,
    pub current_token_id: Balance,
    pub allow_emergency_unlock: bool,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        owner_id: AccountId,
        locked_token: AccountId,
        locked_token_name: String,
        locked_token_decimals: u8,
        penalty_collector: AccountId,
        min_locked_amount: U128,
        early_withdraw_penalty_rate: u64,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");

        let metadata = NFTContractMetadata {
            spec: NFT_METADATA_SPEC.to_string(),
            name: format!("Vesting Escrow NFT for {}", locked_token_name.clone()), // required, ex. "Mosaics"
            symbol: format!("ve{}", locked_token_name.clone()), // required, ex. "MOSIAC"
            icon: Some(DATA_IMAGE_SVG_ICON.to_string()),                                         // Data URL
            base_uri: None,
            reference: None,
            reference_hash: None,
        };

        metadata.assert_valid();
        let mut this = Self {
            owner_id: owner_id.clone(),
            deposits: LookupMap::new(StorageKey::Deposits),
            locked_token: locked_token.clone(),
            penalty_collector: penalty_collector.clone(),
            min_locked_amount: min_locked_amount.into(),
            early_withdraw_penalty_rate: early_withdraw_penalty_rate,
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            locked_token_name: locked_token_name,
            locked_token_decimals: locked_token_decimals,
            total_deposit: 0u128,
            register_storage_usage: 0,
            current_token_id: 0,
            allow_emergency_unlock: false,
            voting_power_supply: 0,
            total_locked: 0,
        };
        this.measure_account_storage_usage();
        this
    }

    fn measure_account_storage_usage(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let tmp_account_id = AccountId::new_unchecked("a".repeat(64).to_string());
        self.deposits.insert(&tmp_account_id, &0u128);
        self.register_storage_usage = env::storage_usage() - initial_storage_usage;
        self.deposits.remove(&tmp_account_id);
    }

    #[private]
    pub fn callback_post_withdraw(
        &mut self,
        _token_id: AccountId,
        _account_id: AccountId,
        _amount: U128,
    ) {
        // assert_eq!(token_id.clone(), self.locked_token.clone(), "invalid token");
        // assert_eq!(env::promise_results_count(), 1, "{}", "withdrawal invalid");
        // let amount: Balance = amount.into();
        // match env::promise_result(0) {
        //     PromiseResult::NotReady => unreachable!(),
        //     PromiseResult::Failed => {
        //         env::log(
        //             format!(
        //                 "{} withdraw {} seed with amount {}, Failed.",
        //                 account_id, token_id, amount,
        //             )
        //             .as_bytes(),
        //         );
        //         // all seed amount go to lostfound
        //         // let seed_amount = self.data().seeds_lostfound.get(&seed_id).unwrap_or(0);
        //         // self.data_mut().seeds_lostfound.insert(&seed_id, &(seed_amount + amount));
        //     }
        //     PromiseResult::Successful(_) => {
        //         env::log(
        //             format!(
        //                 "{} withdraw {} seed with amount {}, Succeed.",
        //                 account_id, token_id, amount,
        //             )
        //             .as_bytes(),
        //         );
        //     }
        // }
    }

    #[payable]
    pub fn create_ve_nft(&mut self, locked_amount: U128, days: u64) -> TokenId {
        assert!(
            locked_amount.0 >= self.min_locked_amount,
            "{}",
            "less than min amount"
        );
        let account_id = env::predecessor_account_id();
        let locked_amount = locked_amount.0;
        self.internal_withdraw_token(&account_id, &locked_amount);
        self.total_locked += locked_amount;
        self.internal_create_ve_nft(&account_id, &locked_amount, days)
    }
    #[payable]
    pub fn increase_unlock_time(&mut self, token_id: TokenId, days: u64) {
        let prev_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        self.assert_token_id_owner(token_id.clone(), account_id.clone());

        self.internal_increase_unlock_time(token_id, days);
        self.check_cost_and_refund(prev_usage);
    }

    #[payable]
    pub fn increase_amount(&mut self, token_id: TokenId, amount: U128) {
        require!(amount.0 > 0, "amount must > 0");
        let prev_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        self.assert_token_id_owner(token_id.clone(), account_id.clone());
        let amount = amount.0;
        self.internal_withdraw_token(&account_id, &amount);
        self.internal_increase_amount(token_id.clone(), &amount);
        self.total_locked += amount;

        self.check_cost_and_refund(prev_usage);
    }

    #[payable]
    pub fn increase_amount_and_unlock_time(&mut self, token_id: TokenId, amount: U128, days: u64) {
        require!(amount.0 > 0, "amount must > 0");

        let prev_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        self.assert_token_id_owner(token_id.clone(), account_id.clone());

        let amount = amount.0;
        self.internal_increase_unlock_time(token_id.clone(), days.clone());

        self.internal_withdraw_token(&account_id, &amount);
        self.internal_increase_amount(token_id.clone(), &amount);
        self.total_locked += amount;

        self.check_cost_and_refund(prev_usage);
    }
    #[payable]
    pub fn unlock(&mut self, token_id: TokenId) -> Promise {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        self.assert_token_id_owner(token_id.clone(), account_id.clone());
        self.internal_unlock(&account_id, token_id, false);
        Promise::new(self.locked_token.clone())
    }
    #[payable]
    pub fn emergency_unlock(&mut self, token_id: TokenId) -> Promise {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        self.assert_token_id_owner(token_id.clone(), account_id.clone());

        self.internal_unlock(&account_id, token_id, true);
        Promise::new(self.locked_token.clone())
    }
}

near_contract_standards::impl_non_fungible_token_core!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_approval!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(Contract, tokens);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

/// Internal methods implementation.
impl Contract {
    fn assert_token_id_owner(&self, token_id: TokenId, account_id: AccountId) {
        let token_owner_id = self
            .tokens
            .owner_by_id
            .get(&token_id)
            .unwrap_or_else(|| env::panic_str("Token not found"));
        require!(
            account_id.clone() == token_owner_id.clone(),
            "not token owner"
        );
    }

    fn check_cost_and_refund(&self, prev_storage: StorageUsage) {
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

    pub fn internal_increase_amount(&mut self, token_id: TokenId, locked_amount: &Balance) {
        let token = match self.tokens.nft_token(token_id.clone()) {
            Some(t) => t,
            None => env::panic_str("no token found"),
        };

        let mut metadata = token.metadata.unwrap();
        let mut lock_info = self.unwrap_metadata(&metadata);

        let vp = self
            .voting_power_unlock_time(U128(locked_amount.clone()), lock_info.locked_till.clone())
            .0;
        lock_info.locked_token_amount = U128(lock_info.locked_token_amount.0 + locked_amount);

        lock_info.voting_power = U128(lock_info.voting_power.0 + vp);

        metadata.extra = Some(self.wrap_metadata(&lock_info));

        self.voting_power_supply += vp;
        self.tokens
            .token_metadata_by_id
            .as_mut()
            .and_then(|by_id| by_id.insert(&token_id, &metadata));
    }

    pub fn internal_register_account(&mut self, account_id: &AccountId) {
        if self.deposits.insert(&account_id, &0).is_some() {
            env::panic_str("The account is already registered");
        }

        self.deposits.insert(&account_id, &0u128);
    }

    pub fn internal_unwrap_deposit(&self, account_id: &AccountId) -> Balance {
        match self.deposits.get(account_id) {
            Some(d) => d,
            None => env::panic_str("account to registered"),
        }
    }

    fn internal_deposit_token(&mut self, receiver_id: &AccountId, amount: &Balance) {
        let deposited = self.internal_unwrap_deposit(receiver_id);
        if let Some(new_deposited) = deposited.checked_add(amount.clone()) {
            self.deposits.insert(receiver_id, &new_deposited);
            self.total_deposit = self
                .total_deposit
                .checked_add(amount.clone())
                .unwrap_or_else(|| env::panic_str("Total deposit overflow"));
        } else {
            env::panic_str("deposit overflow");
        }
    }

    fn internal_withdraw_token(&mut self, account_id: &AccountId, amount: &Balance) {
        let deposited = self.internal_unwrap_deposit(account_id);
        if let Some(new_deposited) = deposited.checked_sub(amount.clone()) {
            self.deposits.insert(account_id, &new_deposited);
            self.total_deposit = self
                .total_deposit
                .checked_sub(amount.clone())
                .unwrap_or_else(|| env::panic_str("Total deposit underflow"));
        } else {
            env::panic_str("deposit underflow");
        }
    }

    fn internal_create_ve_nft(
        &mut self,
        account_id: &AccountId,
        locked_amount: &Balance,
        days: u64,
    ) -> TokenId {
        self.current_token_id += 1;
        let token_id = format!("{}", self.current_token_id);
        let voting_power = self
            .voting_power_locked_days(U128(locked_amount.clone()), days.clone())
            .0;
        let lock_info = LockInfo {
            creator: account_id.clone(),
            create_time_sec: current_time_sec(),
            locked_token_amount: U128(locked_amount.clone()),
            locked_till: current_time_sec() + days * 86400,
            voting_power: U128(voting_power.clone()),
        };
        let token_metadata = TokenMetadata {
            title: Some(
                format!("Vesting Escrow NFT for token {}", self.locked_token_name).to_string(),
            ),
            description: None,
            media: None,
            media_hash: None,
            copies: None,
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: Some(self.wrap_metadata(&lock_info)),
            reference: None,
            reference_hash: None,
        };
        self.tokens
            .internal_mint(token_id.clone(), account_id.clone(), Some(token_metadata));
        self.voting_power_supply += voting_power;
        token_id
    }

    fn internal_increase_unlock_time(&mut self, token_id: TokenId, days: u64) {
        assert!(days >= MINDAYS, "voting lock must be 7 days mint");
        assert!(days <= MAXDAYS, "voting lock must be 4 years max");
        let token = match self.tokens.nft_token(token_id.clone()) {
            Some(t) => t,
            None => env::panic_str("no token found"),
        };

        let mut metadata = token.metadata.unwrap();
        let mut lock_info = self.unwrap_metadata(&metadata);
        let now = env::block_timestamp_ms() / 1000;
        let vp = self
            .voting_power_locked_days(lock_info.locked_token_amount, days)
            .0;
        lock_info.locked_till = lock_info.locked_till + days * 86400;
        lock_info.voting_power = U128(lock_info.voting_power.0 + vp);
        assert!(
            lock_info.locked_till - now <= MAXTIME,
            "{}",
            "Cannot extend lock to more than 4 years"
        );

        metadata.extra = Some(self.wrap_metadata(&lock_info));
        self.voting_power_supply += vp;
        self.tokens
            .token_metadata_by_id
            .as_mut()
            .and_then(|by_id| by_id.insert(&token_id, &metadata));
    }

    fn unwrap_metadata(&self, metadata: &TokenMetadata) -> LockInfo {
        near_sdk::serde_json::from_str::<LockInfo>(&metadata.extra.clone().unwrap())
            .expect("wrong metadata format")
    }

    fn wrap_metadata(&self, lock_info: &LockInfo) -> String {
        near_sdk::serde_json::to_string(lock_info).unwrap()
    }

    fn internal_penalize(&mut self, _amount: Balance) {
        //just burn locked_token for amount
        //self.token.internal_withdraw(account_id, amount.clone());
    }

    fn internal_unlock(&mut self, _account_id: &AccountId, _token_id: TokenId, _emergency: bool) {
        // let mut lock_info = self.get_token_ve_metadata(token_id.clone());
        // let now = env::block_timestamp_ms() / 1000;
        // assert!(
        //     lock_info.locked_token_amount.0 > 0,
        //     "{}",
        //     "Nothing to withdraw"
        // );
        // if !emergency {
        //     assert!(now >= locked_balance.locked_till, "lock didnt expire yet");
        // }
        // let mut amount = lock_info.locked_token_amount.0;
        // if now < locked_balance.locked_till {
        //     let fee = amount * (self.early_withdraw_penalty_rate as u128) / (PRECISION as u128);
        //     self.internal_penalize(fee.clone()); //burn fee
        //     amount -= fee;
        // }
        // lock_info.locked_token_amount = 0;
        // lock_info.locked_till = 0;

        // //burn ve
        // self.token
        //     .internal_withdraw(account_id, self.internal_unwrap_minted_for_lock(account_id));
        // self.minted_for_lock.insert(account_id, &0);
        // self.supply -= amount;
        // self.lockeds.insert(account_id, &locked_balance);

        // ext_fungible_token::ft_transfer(
        //     account_id.clone(),
        //     amount.into(),
        //     None,
        //     &(self.locked_token),
        //     1,
        //     GAS_FOR_FT_TRANSFER,
        // )
        // .then(ext_self::callback_post_withdraw(
        //     self.locked_token.clone(),
        //     account_id.clone(),
        //     amount.into(),
        //     &env::current_account_id(),
        //     0,
        //     GAS_FOR_RESOLVE_TRANSFER,
        // ));
    }
}

#[cfg(test)]
mod tests {
    // use std::convert::TryFrom;

    // use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    // use near_sdk::test_utils::{accounts, VMContextBuilder};
    // use near_sdk::{testing_env, Balance, MockedBlockchain};
    // use near_sdk_sim::to_yocto;

    // use super::*;

    // fn setup_contract() -> (VMContextBuilder, Contract) {
    //     let mut context = VMContextBuilder::new();
    //     testing_env!(context.predecessor_account_id(accounts(0)).build());
    //     let contract = Contract::new(accounts(0), 1600, 400);
    //     (context, contract)
    // }

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

    // fn create_pool_with_liquidity(
    //     context: &mut VMContextBuilder,
    //     contract: &mut Contract,
    //     account_id: ValidAccountId,
    //     token_amounts: Vec<(ValidAccountId, Balance)>,
    // ) -> u64 {
    //     let tokens = token_amounts
    //         .iter()
    //         .map(|(x, _)| x.clone())
    //         .collect::<Vec<_>>();
    //     testing_env!(context
    //         .predecessor_account_id(accounts(0))
    //         .attached_deposit(1)
    //         .build());
    //     contract.extend_whitelisted_tokens(tokens.clone());
    //     testing_env!(context
    //         .predecessor_account_id(account_id.clone())
    //         .attached_deposit(env::storage_byte_cost() * 300)
    //         .build());
    //     let pool_id = contract.add_simple_pool(tokens, 25);
    //     testing_env!(context
    //         .predecessor_account_id(account_id.clone())
    //         .attached_deposit(to_yocto("0.03"))
    //         .build());
    //     contract.storage_deposit(None, None);
    //     deposit_tokens(context, contract, accounts(3), token_amounts.clone());
    //     testing_env!(context
    //         .predecessor_account_id(account_id.clone())
    //         .attached_deposit(to_yocto("0.0007"))
    //         .build());
    //     contract.add_liquidity(
    //         pool_id,
    //         token_amounts.into_iter().map(|(_, x)| U128(x)).collect(),
    //         None,
    //     );
    //     pool_id
    // }

    // fn swap(
    //     contract: &mut Contract,
    //     pool_id: u64,
    //     token_in: ValidAccountId,
    //     amount_in: Balance,
    //     token_out: ValidAccountId,
    // ) -> Balance {
    //     contract
    //         .swap(
    //             vec![SwapAction {
    //                 pool_id,
    //                 token_in: token_in.into(),
    //                 amount_in: Some(U128(amount_in)),
    //                 token_out: token_out.into(),
    //                 min_amount_out: U128(1),
    //             }],
    //             None,
    //         )
    //         .0
    // }

    // #[test]
    // fn test_basics() {
    //     let one_near = 10u128.pow(24);
    //     let (mut context, mut contract) = setup_contract();
    //     // add liquidity of (1,2) tokens
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
    //     );
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![
    //             (accounts(1), to_yocto("100")),
    //             (accounts(2), to_yocto("100")),
    //         ],
    //     );
    //     deposit_tokens(&mut context, &mut contract, accounts(1), vec![]);

    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(1)),
    //         to_yocto("100").into()
    //     );
    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(2)),
    //         to_yocto("100").into()
    //     );
    //     assert_eq!(
    //         contract.get_pool_total_shares(0).0,
    //         crate::utils::INIT_SHARES_SUPPLY
    //     );

    //     // Get price from pool :0 1 -> 2 tokens.
    //     let expected_out = contract.get_return(0, accounts(1), one_near.into(), accounts(2));
    //     assert_eq!(expected_out.0, 1663192997082117548978741);

    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     let amount_out = swap(&mut contract, 0, accounts(1), one_near, accounts(2));
    //     assert_eq!(amount_out, expected_out.0);
    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(1)).0,
    //         99 * one_near
    //     );
    //     // transfer some of token_id 2 from acc 3 to acc 1.
    //     testing_env!(context.predecessor_account_id(accounts(3)).build());
    //     contract.mft_transfer(accounts(2).to_string(), accounts(1), U128(one_near), None);
    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(2)).0,
    //         99 * one_near + amount_out
    //     );
    //     assert_eq!(contract.get_deposit(accounts(1), accounts(2)).0, one_near);

    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(to_yocto("0.0067"))
    //         .build());
    //     contract.mft_register(":0".to_string(), accounts(1));
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     // transfer 1m shares in pool 0 to acc 1.
    //     contract.mft_transfer(":0".to_string(), accounts(1), U128(1_000_000), None);

    //     testing_env!(context.predecessor_account_id(accounts(3)).build());
    //     contract.remove_liquidity(
    //         0,
    //         contract.get_pool_shares(0, accounts(3)),
    //         vec![1.into(), 2.into()],
    //     );
    //     // Exchange fees left in the pool as liquidity + 1m from transfer.
    //     assert_eq!(
    //         contract.get_pool_total_shares(0).0,
    //         33336806279123620258 + 1_000_000
    //     );

    //     contract.withdraw(
    //         accounts(1),
    //         contract.get_deposit(accounts(3), accounts(1)),
    //         None,
    //     );
    //     assert_eq!(contract.get_deposit(accounts(3), accounts(1)).0, 0);
    // }

    // /// Test liquidity management.
    // #[test]
    // fn test_liquidity() {
    //     let (mut context, mut contract) = setup_contract();
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![
    //             (accounts(1), to_yocto("100")),
    //             (accounts(2), to_yocto("100")),
    //         ],
    //     );
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     let id = contract.add_simple_pool(vec![accounts(1), accounts(2)], 25);
    //     testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
    //     contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("10"))], None);
    //     contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("50"))], None);
    //     testing_env!(context.attached_deposit(1).build());
    //     contract.remove_liquidity(id, U128(to_yocto("1")), vec![U128(1), U128(1)]);

    //     // Check that amounts add up to deposits.
    //     let amounts = contract.get_pool(id).amounts;
    //     let deposit1 = contract.get_deposit(accounts(3), accounts(1)).0;
    //     let deposit2 = contract.get_deposit(accounts(3), accounts(2)).0;
    //     assert_eq!(amounts[0].0 + deposit1, to_yocto("100"));
    //     assert_eq!(amounts[1].0 + deposit2, to_yocto("100"));
    // }

    // /// Should deny creating a pool with duplicate tokens.
    // #[test]
    // #[should_panic(expected = "E92: token duplicated")]
    // fn test_deny_duplicate_tokens_pool() {
    //     let (mut context, mut contract) = setup_contract();
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![(accounts(1), to_yocto("5")), (accounts(1), to_yocto("10"))],
    //     );
    // }

    // /// Deny pool with a single token
    // #[test]
    // #[should_panic(expected = "E89: wrong token count")]
    // fn test_deny_single_token_pool() {
    //     let (mut context, mut contract) = setup_contract();
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![(accounts(1), to_yocto("5"))],
    //     );
    // }

    // /// Deny pool with a single token
    // #[test]
    // #[should_panic(expected = "E89: wrong token count")]
    // fn test_deny_too_many_tokens_pool() {
    //     let (mut context, mut contract) = setup_contract();
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![
    //             (accounts(1), to_yocto("5")),
    //             (accounts(2), to_yocto("10")),
    //             (accounts(3), to_yocto("10")),
    //         ],
    //     );
    // }

    // #[test]
    // #[should_panic(expected = "E12: token not whitelisted")]
    // fn test_deny_send_malicious_token() {
    //     let (mut context, mut contract) = setup_contract();
    //     let acc = ValidAccountId::try_from("test_user").unwrap();
    //     testing_env!(context
    //         .predecessor_account_id(acc.clone())
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     contract.storage_deposit(Some(acc.clone()), None);
    //     testing_env!(context
    //         .predecessor_account_id(ValidAccountId::try_from("malicious").unwrap())
    //         .build());
    //     contract.ft_on_transfer(acc, U128(1_000), "".to_string());
    // }

    // #[test]
    // fn test_send_user_specific_token() {
    //     let (mut context, mut contract) = setup_contract();
    //     let acc = ValidAccountId::try_from("test_user").unwrap();
    //     let custom_token = ValidAccountId::try_from("custom").unwrap();
    //     testing_env!(context
    //         .predecessor_account_id(acc.clone())
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     contract.storage_deposit(None, None);
    //     testing_env!(context.attached_deposit(1).build());
    //     contract.register_tokens(vec![custom_token.clone()]);
    //     testing_env!(context.predecessor_account_id(custom_token.clone()).build());
    //     contract.ft_on_transfer(acc.clone(), U128(1_000), "".to_string());
    //     let prev = contract.storage_balance_of(acc.clone()).unwrap();
    //     testing_env!(context
    //         .predecessor_account_id(acc.clone())
    //         .attached_deposit(1)
    //         .build());
    //     contract.withdraw(custom_token, U128(1_000), Some(true));
    //     let new = contract.storage_balance_of(acc.clone()).unwrap();
    //     // More available storage after withdrawing & unregistering the token.
    //     assert!(new.available.0 > prev.available.0);
    // }

    // #[test]
    // #[should_panic(expected = "E68: slippage error")]
    // fn test_deny_min_amount() {
    //     let (mut context, mut contract) = setup_contract();
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![(accounts(1), to_yocto("1")), (accounts(2), to_yocto("1"))],
    //     );
    //     let acc = ValidAccountId::try_from("test_user").unwrap();

    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         acc.clone(),
    //         vec![(accounts(1), 1_000_000)],
    //     );

    //     testing_env!(context
    //         .predecessor_account_id(acc.clone())
    //         .attached_deposit(1)
    //         .build());
    //     contract.swap(
    //         vec![SwapAction {
    //             pool_id: 0,
    //             token_in: accounts(1).into(),
    //             amount_in: Some(U128(1_000_000)),
    //             token_out: accounts(2).into(),
    //             min_amount_out: U128(1_000_000),
    //         }],
    //         None,
    //     );
    // }

    // #[test]
    // fn test_second_storage_deposit_works() {
    //     let (mut context, mut contract) = setup_contract();
    //     testing_env!(context.attached_deposit(to_yocto("1")).build());
    //     contract.storage_deposit(None, None);
    //     testing_env!(context.attached_deposit(to_yocto("0.001")).build());
    //     contract.storage_deposit(None, None);
    // }

    // #[test]
    // #[should_panic(expected = "E72: at least one swap")]
    // fn test_fail_swap_no_actions() {
    //     let (mut context, mut contract) = setup_contract();
    //     testing_env!(context.attached_deposit(to_yocto("1")).build());
    //     contract.storage_deposit(None, None);
    //     testing_env!(context.attached_deposit(1).build());
    //     contract.swap(vec![], None);
    // }

    // /// Check that can not swap non whitelisted tokens when attaching 0 deposit (access key).
    // #[test]
    // #[should_panic(expected = "E27: attach 1yN to swap tokens not in whitelist")]
    // fn test_fail_swap_not_whitelisted() {
    //     let (mut context, mut contract) = setup_contract();
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         accounts(0),
    //         vec![(accounts(1), 2_000_000), (accounts(2), 1_000_000)],
    //     );
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(0),
    //         vec![(accounts(1), 1_000_000), (accounts(2), 1_000_000)],
    //     );
    //     testing_env!(context.attached_deposit(1).build());
    //     contract.remove_whitelisted_tokens(vec![accounts(2)]);
    //     testing_env!(context.attached_deposit(1).build());
    //     contract.unregister_tokens(vec![accounts(2)]);
    //     testing_env!(context.attached_deposit(0).build());
    //     swap(&mut contract, 0, accounts(1), 10, accounts(2));
    // }

    // #[test]
    // fn test_roundtrip_swap() {
    //     let (mut context, mut contract) = setup_contract();
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
    //     );
    //     let acc = ValidAccountId::try_from("test_user").unwrap();
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         acc.clone(),
    //         vec![(accounts(1), 1_000_000)],
    //     );
    //     testing_env!(context
    //         .predecessor_account_id(acc.clone())
    //         .attached_deposit(1)
    //         .build());
    //     contract.swap(
    //         vec![
    //             SwapAction {
    //                 pool_id: 0,
    //                 token_in: accounts(1).into(),
    //                 amount_in: Some(U128(1_000)),
    //                 token_out: accounts(2).into(),
    //                 min_amount_out: U128(1),
    //             },
    //             SwapAction {
    //                 pool_id: 0,
    //                 token_in: accounts(2).into(),
    //                 amount_in: None,
    //                 token_out: accounts(1).into(),
    //                 min_amount_out: U128(1),
    //             },
    //         ],
    //         None,
    //     );
    //     // Roundtrip returns almost everything except 0.25% fee.
    //     assert_eq!(contract.get_deposit(acc, accounts(1)).0, 1_000_000 - 6);
    // }

    // #[test]
    // #[should_panic(expected = "E14: LP already registered")]
    // fn test_lpt_transfer() {
    //     // account(0) -- swap contract
    //     // account(1) -- token0 contract
    //     // account(2) -- token1 contract
    //     // account(3) -- user account
    //     // account(4) -- another user account
    //     let (mut context, mut contract) = setup_contract();
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![
    //             (accounts(1), to_yocto("100")),
    //             (accounts(2), to_yocto("100")),
    //         ],
    //     );
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     let id = contract.add_simple_pool(vec![accounts(1), accounts(2)], 25);
    //     testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
    //     contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("10"))], None);
    //     assert_eq!(
    //         contract.mft_balance_of(":0".to_string(), accounts(3)).0,
    //         to_yocto("1")
    //     );
    //     assert_eq!(contract.mft_total_supply(":0".to_string()).0, to_yocto("1"));
    //     testing_env!(context.attached_deposit(1).build());
    //     contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("50"))], None);
    //     assert_eq!(
    //         contract.mft_balance_of(":0".to_string(), accounts(3)).0,
    //         to_yocto("2")
    //     );
    //     assert_eq!(contract.mft_total_supply(":0".to_string()).0, to_yocto("2"));

    //     // register another user
    //     testing_env!(context
    //         .predecessor_account_id(accounts(4))
    //         .attached_deposit(to_yocto("0.00071"))
    //         .build());
    //     contract.mft_register(":0".to_string(), accounts(4));
    //     // make transfer to him
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     contract.mft_transfer(":0".to_string(), accounts(4), U128(to_yocto("1")), None);
    //     assert_eq!(
    //         contract.mft_balance_of(":0".to_string(), accounts(3)).0,
    //         to_yocto("1")
    //     );
    //     assert_eq!(
    //         contract.mft_balance_of(":0".to_string(), accounts(4)).0,
    //         to_yocto("1")
    //     );
    //     assert_eq!(contract.mft_total_supply(":0".to_string()).0, to_yocto("2"));
    //     // remove lpt for account 3
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     contract.remove_liquidity(id, U128(to_yocto("0.6")), vec![U128(1), U128(1)]);
    //     assert_eq!(
    //         contract.mft_balance_of(":0".to_string(), accounts(3)).0,
    //         to_yocto("0.4")
    //     );
    //     assert_eq!(
    //         contract.mft_total_supply(":0".to_string()).0,
    //         to_yocto("1.4")
    //     );
    //     // remove lpt for account 4 who got lpt from others
    //     if contract.storage_balance_of(accounts(4)).is_none() {
    //         testing_env!(context
    //             .predecessor_account_id(accounts(4))
    //             .attached_deposit(to_yocto("1"))
    //             .build());
    //         contract.storage_deposit(None, None);
    //     }
    //     testing_env!(context
    //         .predecessor_account_id(accounts(4))
    //         .attached_deposit(1)
    //         .build());
    //     contract.remove_liquidity(id, U128(to_yocto("1")), vec![U128(1), U128(1)]);
    //     assert_eq!(
    //         contract.mft_balance_of(":0".to_string(), accounts(4)).0,
    //         to_yocto("0")
    //     );
    //     assert_eq!(
    //         contract.mft_total_supply(":0".to_string()).0,
    //         to_yocto("0.4")
    //     );

    //     // [AUDIT_13]
    //     // should panic cause accounts(4) not removed by a full remove liquidity
    //     testing_env!(context
    //         .predecessor_account_id(accounts(4))
    //         .attached_deposit(to_yocto("0.00071"))
    //         .build());
    //     contract.mft_register(":0".to_string(), accounts(4));
    // }

    // #[test]
    // #[should_panic(expected = "E33: transfer to self")]
    // fn test_lpt_transfer_self() {
    //     // [AUDIT_07]
    //     // account(0) -- swap contract
    //     // account(1) -- token0 contract
    //     // account(2) -- token1 contract
    //     // account(3) -- user account
    //     let (mut context, mut contract) = setup_contract();
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![
    //             (accounts(1), to_yocto("100")),
    //             (accounts(2), to_yocto("100")),
    //         ],
    //     );
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     let id = contract.add_simple_pool(vec![accounts(1), accounts(2)], 25);
    //     testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
    //     contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("10"))], None);
    //     assert_eq!(
    //         contract.mft_balance_of(":0".to_string(), accounts(3)).0,
    //         to_yocto("1")
    //     );
    //     testing_env!(context.attached_deposit(1).build());
    //     contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("50"))], None);
    //     assert_eq!(
    //         contract.mft_balance_of(":0".to_string(), accounts(3)).0,
    //         to_yocto("2")
    //     );

    //     // make transfer to self
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     contract.mft_transfer(":0".to_string(), accounts(3), U128(to_yocto("1")), None);
    // }

    // #[test]
    // fn test_storage() {
    //     let (mut context, mut contract) = setup_contract();
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     contract.storage_deposit(Some(accounts(1)), None);
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     assert_eq!(contract.storage_withdraw(None).available.0, 0);
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     assert!(contract.storage_unregister(None));
    // }

    // #[test]
    // fn test_storage_registration_only() {
    //     let (mut context, mut contract) = setup_contract();
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     let deposit1 = contract.storage_deposit(Some(accounts(1)), Some(true));
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(to_yocto("1"))
    //         .build());
    //     let deposit2 = contract.storage_deposit(Some(accounts(1)), Some(true));
    //     assert_eq!(deposit1.total, deposit2.total);
    // }

    // #[test]
    // #[should_panic(expected = "E17: deposit less than min storage")]
    // fn test_storage_deposit_less_then_min_storage() {
    //     let (mut context, mut contract) = setup_contract();
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     contract.storage_deposit(Some(accounts(1)), Some(true));
    // }

    // #[test]
    // fn test_instant_swap() {
    //     let (mut context, mut contract) = setup_contract();
    //     // add liquidity of (1,2) tokens
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
    //     );
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![
    //             (accounts(1), to_yocto("100")),
    //             (accounts(2), to_yocto("100")),
    //         ],
    //     );

    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(1)),
    //         to_yocto("100").into()
    //     );
    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(2)),
    //         to_yocto("100").into()
    //     );
    //     assert_eq!(
    //         contract.get_pool_total_shares(0).0,
    //         crate::utils::INIT_SHARES_SUPPLY
    //     );

    //     // Get price from pool :0 1 -> 2 tokens.
    //     let expected_out = contract.get_return(0, accounts(1), to_yocto("1").into(), accounts(2));
    //     assert_eq!(expected_out.0, 1663192997082117548978741);

    //     let actions_str = format!(
    //         "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
    //         0, accounts(1), accounts(2), 1
    //     );

    //     let msg_str = format!("{{\"actions\": [{}]}}", actions_str);
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     contract.ft_on_transfer(accounts(3), to_yocto("1").into(), msg_str);
    // }

    // #[test]
    // fn test_mft_transfer_call() {
    //     let one_near = 10u128.pow(24);
    //     let (mut context, mut contract) = setup_contract();
    //     // add liquidity of (1,2) tokens
    //     create_pool_with_liquidity(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
    //     );
    //     println!("{:?}", contract.get_pools(0, 100));
    //     println!("{:?}", contract.get_pool(0));
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         vec![
    //             (accounts(1), to_yocto("100")),
    //             (accounts(2), to_yocto("100")),
    //         ],
    //     );
    //     deposit_tokens(&mut context, &mut contract, accounts(1), vec![]);

    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(1)),
    //         to_yocto("100").into()
    //     );
    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(2)),
    //         to_yocto("100").into()
    //     );
    //     assert_eq!(
    //         contract.get_pool_total_shares(0).0,
    //         crate::utils::INIT_SHARES_SUPPLY
    //     );

    //     // Get price from pool :0 1 -> 2 tokens.
    //     let expected_out = contract.get_return(0, accounts(1), one_near.into(), accounts(2));
    //     assert_eq!(expected_out.0, 1663192997082117548978741);

    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     let amount_out = swap(&mut contract, 0, accounts(1), one_near, accounts(2));
    //     assert_eq!(amount_out, expected_out.0);
    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(1)).0,
    //         99 * one_near
    //     );
    //     assert_eq!(
    //         "nstable-pool-0".to_string(),
    //         contract.mft_metadata(":0".to_string()).name
    //     );
    //     // transfer some of token_id 2 from acc 3 to acc 1.
    //     testing_env!(context.predecessor_account_id(accounts(3)).build());
    //     contract.mft_transfer_call(
    //         accounts(2).to_string(),
    //         accounts(1),
    //         U128(one_near),
    //         Some("mft".to_string()),
    //         "".to_string(),
    //     );
    //     assert_eq!(
    //         contract.get_deposit(accounts(3), accounts(2)).0,
    //         99 * one_near + amount_out
    //     );
    // }

    // #[test]
    // fn test_stable() {
    //     let (mut context, mut contract) = setup_contract();
    //     let token_amounts = vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("5"))];
    //     let tokens = token_amounts
    //         .iter()
    //         .map(|(x, _)| x.clone())
    //         .collect::<Vec<_>>();
    //     testing_env!(context
    //         .predecessor_account_id(accounts(0))
    //         .attached_deposit(1)
    //         .build());
    //     contract.extend_whitelisted_tokens(tokens.clone());
    //     assert_eq!(
    //         contract.get_whitelisted_tokens(),
    //         vec![accounts(1).to_string(), accounts(2).to_string()]
    //     );
    //     assert_eq!(0, contract.get_user_whitelisted_tokens(accounts(3)).len());
    //     testing_env!(context
    //         .predecessor_account_id(accounts(0))
    //         .attached_deposit(env::storage_byte_cost() * 334)
    //         .build());
    //     let pool_id = contract.add_stable_swap_pool(tokens, vec![18, 18], 25, 240);
    //     println!("{:?}", contract.version());
    //     println!("{:?}", contract.get_stable_pool(pool_id));
    //     println!("{:?}", contract.get_pools(0, 100));
    //     println!("{:?}", contract.get_pool(0));
    //     assert_eq!(1, contract.get_number_of_pools());
    //     assert_eq!(25, contract.get_pool_fee(pool_id));
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(to_yocto("0.03"))
    //         .build());
    //     contract.storage_deposit(None, None);
    //     assert_eq!(
    //         to_yocto("0.03"),
    //         contract
    //             .get_user_storage_state(accounts(3))
    //             .unwrap()
    //             .deposit
    //             .0
    //     );
    //     deposit_tokens(
    //         &mut context,
    //         &mut contract,
    //         accounts(3),
    //         token_amounts.clone(),
    //     );
    //     deposit_tokens(&mut context, &mut contract, accounts(0), vec![]);

    //     let predict = contract.predict_add_stable_liquidity(
    //         pool_id,
    //         &vec![to_yocto("4").into(), to_yocto("4").into()],
    //     );
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(to_yocto("0.0007"))
    //         .build());
    //     let add_liq = contract.add_stable_liquidity(
    //         pool_id,
    //         vec![to_yocto("4").into(), to_yocto("4").into()],
    //         U128(1),
    //     );
    //     assert_eq!(predict.0, add_liq.0);
    //     assert_eq!(100000000, contract.get_pool_share_price(pool_id).0);
    //     assert_eq!(
    //         8000000000000000000000000,
    //         contract.get_pool_shares(pool_id, accounts(3)).0
    //     );
    //     assert_eq!(
    //         8000000000000000000000000,
    //         contract.get_pool_total_shares(pool_id).0
    //     );
    //     let expected_out = contract.get_return(0, accounts(1), to_yocto("1").into(), accounts(2));
    //     assert_eq!(expected_out.0, 996947470156575219215720);

    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     let amount_out = swap(
    //         &mut contract,
    //         0,
    //         accounts(1),
    //         to_yocto("1").into(),
    //         accounts(2),
    //     );
    //     assert_eq!(amount_out, expected_out.0);
    //     assert_eq!(contract.get_deposit(accounts(3), accounts(1)).0, 0);
    //     assert_eq!(
    //         0,
    //         contract
    //             .get_deposits(accounts(3))
    //             .get(&accounts(1).to_string())
    //             .unwrap()
    //             .0
    //     );
    //     assert_eq!(
    //         to_yocto("1") + 996947470156575219215720,
    //         contract
    //             .get_deposits(accounts(3))
    //             .get(&accounts(2).to_string())
    //             .unwrap()
    //             .0
    //     );

    //     let predict = contract.predict_remove_liquidity(pool_id, to_yocto("0.1").into());
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     let remove_liq =
    //         contract.remove_liquidity(pool_id, to_yocto("0.1").into(), vec![1.into(), 1.into()]);
    //     assert_eq!(predict, remove_liq);

    //     let predict = contract.predict_remove_liquidity_by_tokens(
    //         pool_id,
    //         &vec![to_yocto("0.1").into(), to_yocto("0.1").into()],
    //     );
    //     testing_env!(context
    //         .predecessor_account_id(accounts(3))
    //         .attached_deposit(1)
    //         .build());
    //     let remove_liq_by_token = contract.remove_liquidity_by_tokens(
    //         pool_id,
    //         vec![to_yocto("0.1").into(), to_yocto("0.1").into()],
    //         to_yocto("1").into(),
    //     );
    //     assert_eq!(predict.0, remove_liq_by_token.0);

    //     testing_env!(context
    //         .predecessor_account_id(accounts(0))
    //         .attached_deposit(1)
    //         .build());
    //     contract.remove_exchange_fee_liquidity(
    //         0,
    //         to_yocto("0.0001").into(),
    //         vec![1.into(), 1.into()],
    //     );
    //     testing_env!(context
    //         .predecessor_account_id(accounts(0))
    //         .attached_deposit(1)
    //         .build());
    //     contract.withdraw_owner_token(accounts(1), to_yocto("0.00001").into());
    //     testing_env!(context
    //         .predecessor_account_id(accounts(0))
    //         .block_timestamp(2 * 86400 * 1_000_000_000)
    //         .attached_deposit(1)
    //         .build());
    //     contract.stable_swap_ramp_amp(0, 250, (3 * 86400 * 1_000_000_000).into());
    //     testing_env!(context
    //         .predecessor_account_id(accounts(0))
    //         .attached_deposit(1)
    //         .build());
    //     contract.stable_swap_stop_ramp_amp(0);
    // }

    // #[test]
    // fn test_owner() {
    //     let (mut context, mut contract) = setup_contract();
    //     testing_env!(context
    //         .predecessor_account_id(accounts(0))
    //         .attached_deposit(1)
    //         .build());
    //     contract.set_owner(accounts(1));
    //     assert_eq!(accounts(1).to_string(), contract.get_owner());
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     contract.retrieve_unmanaged_token(accounts(2), U128(1));
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     contract.extend_guardians(vec![accounts(2)]);
    //     assert_eq!(vec![accounts(2).to_string()], contract.get_guardians());
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     contract.remove_guardians(vec![accounts(2)]);
    //     assert_eq!(0, contract.get_guardians().len());
    //     assert_eq!(RunningState::Running, contract.metadata().state);
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     contract.change_state(RunningState::Paused);
    //     assert_eq!(RunningState::Paused, contract.metadata().state);
    //     assert_eq!(1600, contract.metadata().exchange_fee);
    //     assert_eq!(400, contract.metadata().referral_fee);
    //     testing_env!(context
    //         .predecessor_account_id(accounts(1))
    //         .attached_deposit(1)
    //         .build());
    //     contract.modify_admin_fee(20, 50);
    //     assert_eq!(20, contract.metadata().exchange_fee);
    //     assert_eq!(50, contract.metadata().referral_fee);
    // }
}
