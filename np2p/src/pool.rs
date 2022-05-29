use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedMap};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, require, AccountId, Balance, BorshStorageKey, Gas,
    PanicOnDefault, Promise, StorageUsage,
};

use crate::*;
//pool can be created by any one
//users can either provide lending assets for others to borrow
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Pool {
    pub pool_id: u32,
    pub owner_id: AccountId,
    pub lend_token_id: AssetId,
    pub collateral_token_id: AssetId, //borrowers collateral assets to borrow lended token
    pub min_cr: u64, //min of (value of collateral)/(value of borrowed), when CR of user belows min_cr, liquidation starts
    pub max_utilization: u64, //max of total borrow / total lend asset liquidity
    pub min_lend_token_deposit: Balance,
    pub min_lend_token_borrow: Balance,
    pub total_lend_asset_deposit: Balance,
    pub total_collateral_deposit: Balance,
    pub total_borrow: Balance,
    pub account_deposits: UnorderedMap<AccountId, AccountDeposit>,
    pub fixed_interest_rate: u64,
    pub acc_interest_per_share: Balance,
    pub last_acc_interest_update_timestamp_sec: u64,

    liquidation_bonus: u64, //or price penalty, it means how much discount liquidators can buy collateral tokens using lend token to pay the debt
}

impl Pool {
    fn new(
        pool_id: u32,
        owner_id: AccountId,
        lend_token_id: AssetId,
        collateral_token_id: AssetId,
        min_cr: u64,
        max_utilization: u64,
        min_lend_token_deposit: Balance,
        min_lend_token_borrow: Balance,
        fixed_interest_rate: u64,
        liquidation_bonus: u64,
    ) -> Pool {
        require!(
            lend_token_id.clone() != collateral_token_id.clone(),
            "lend and collateral tokens must be different"
        );
        Pool {
            pool_id: pool_id.clone(),
            owner_id: owner_id.clone(),
            lend_token_id: lend_token_id.clone(),
            collateral_token_id: collateral_token_id.clone(), //borrowers collateral assets to borrow lended token
            min_cr: min_cr, //min of (value of collateral)/(value of borrowed), when CR of user belows min_cr, liquidation starts
            max_utilization: max_utilization, //max of total borrow / total lend asset liquidity
            min_lend_token_deposit: min_lend_token_deposit,
            min_lend_token_borrow: min_lend_token_borrow,
            total_lend_asset_deposit: 0,
            total_collateral_deposit: 0,
            total_borrow: 0,
            account_deposits: UnorderedMap::new(
                format!("p_account_deposits_{}", pool_id).as_bytes(),
            ),
            fixed_interest_rate: fixed_interest_rate,
            acc_interest_per_share: 0,
            last_acc_interest_update_timestamp_sec: 0,
            liquidation_bonus: liquidation_bonus,
        }
    }

    pub fn deposit(&mut self, account_id: &AccountId, token_id: &AssetId, amount: Balance) {
        let mut account_deposit = self.get_account_deposit(account_id);

        if token_id.clone() == self.lend_token_id {
            if self.total_lend_asset_deposit == 0 {
                self.last_acc_interest_update_timestamp_sec = env::block_timestamp_ms() / 1000;
                account_deposit.deposit_lend_token(&amount, &0u128);
            } else {
                //update acc_interest_per_share
                self.acc_interest_per_share = self.get_current_acc_interest_per_share();
                self.last_acc_interest_update_timestamp_sec = env::block_timestamp_ms() / 1000;
                account_deposit.deposit_lend_token(&amount, &self.acc_interest_per_share);
            }
            self.total_lend_asset_deposit += amount;
        } else if token_id.clone() == self.collateral_token_id {
            account_deposit.deposit_collateral(&amount);
            self.total_collateral_deposit += amount;
        } else {
            env::panic_str("unsupported token for pool")
        }
        self.account_deposits.insert(account_id, &account_deposit);
    }

    pub fn get_account_deposit(&self, account_id: &AccountId) -> AccountDeposit {
        self.account_deposits
            .get(account_id)
            .unwrap_or(AccountDeposit::new(
                self.pool_id.clone(),
                account_id.clone(),
                self.lend_token_id.clone(),
                self.collateral_token_id.clone(),
            ))
    }
    pub fn borrow(
        &mut self,
        account_id: &AccountId,
        amount: &Balance,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
    ) {
        //pub fn borrow(&mut self, account_id: &AccountId, amount: &Balance, contract: &mut Contract) {
        require!(
            self.lend_token_id == lend_token_info.token_id
                && self.collateral_token_id == collateral_token_info.token_id,
            "invalid token info"
        );

        let total_borrow_after = self.total_borrow + amount.clone();
        if self.total_lend_asset_deposit * self.max_utilization as u128 / UTILIZATION_DIVISOR
            < total_borrow_after
        {
            env::panic_str(&format!(
                "borrowing utilization exceed max utilization ratio at {}",
                self.max_utilization
            ));
        }

        self.update_acc_interest_per_share();

        let mut account_deposit = self.get_account_deposit(account_id);
        let actual_borrow_amount = account_deposit.borrow(
            amount,
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            self.fixed_interest_rate,
            self.min_cr,
        );
        self.account_deposits.insert(account_id, &account_deposit);
        if actual_borrow_amount != amount.clone() {
            self.total_lend_asset_deposit -= amount.clone() - actual_borrow_amount;
            self.total_borrow += actual_borrow_amount.clone();
        } else {
            self.total_borrow += amount.clone();
        }
    }

    pub fn register_account(&mut self, account_id: &AccountId) {
        let account_deposit = AccountDeposit::new(
            0,
            account_id.clone(),
            self.lend_token_id.clone(),
            self.collateral_token_id.clone(),
        );
        self.account_deposits.insert(account_id, &account_deposit);
    }

    pub fn withdraw_from_account(
        &mut self,
        account_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
    ) {
        self.update_acc_interest_per_share();
        let mut account_deposit = self.get_account_deposit(account_id);
        account_deposit.update_account(self.fixed_interest_rate, &self.acc_interest_per_share);
        
        let withdrawn_amount_from_deposit = account_deposit.withdraw_from_account(token_id, amount.clone(), lend_token_info, lend_token_price, collateral_token_info, collateral_token_price, self.fixed_interest_rate, self.min_cr);
        self.account_deposits.insert(account_id, &account_deposit);

        if token_id.clone() == self.lend_token_id {
            self.total_lend_asset_deposit -= withdrawn_amount_from_deposit;
        } else if token_id.clone() == self.collateral_token_id {
            self.total_collateral_deposit -= withdrawn_amount_from_deposit;
        }
    }

    fn update_acc_interest_per_share(&mut self) {
        self.acc_interest_per_share = self.get_current_acc_interest_per_share();
        self.last_acc_interest_update_timestamp_sec = env::block_timestamp_ms() / 1000;
    }

    pub fn pay_loan(
        &mut self,
        account_id: &AccountId,
        pay_amount: Balance,
    ) {
        self.update_acc_interest_per_share();
        let mut account_deposit = self.get_account_deposit(account_id);
        account_deposit.update_account(self.fixed_interest_rate, &self.acc_interest_per_share);
        let (paid_borrow, added_liquidity) = account_deposit.pay_loan(pay_amount, self.acc_interest_per_share.clone());
        self.account_deposits.insert(account_id, &account_deposit);

        self.total_borrow -= paid_borrow;
        self.total_lend_asset_deposit += added_liquidity;
    }

    pub fn get_current_acc_interest_per_share(&self) -> Balance {
        let elapsed_time =
            env::block_timestamp_ms() / 1000 - self.last_acc_interest_update_timestamp_sec;
        let elapsed_time = elapsed_time as u128;
        let generated_interest =
            self.total_borrow * elapsed_time * (self.fixed_interest_rate as u128)
                / (SECONDS_PER_YEAR * INTEREST_RATE_DIVISOR);
        return generated_interest * ACC_INTEREST_PER_SHARE_MULTIPLIER
            / self.total_lend_asset_deposit;
    }
}

pub fn new_pool_default(
    pool_id: u32,
    owner_id: AccountId,
    lend_token_id: AssetId,
    collateral_token_id: AssetId,
) -> Pool {
    Pool::new(
        pool_id,
        owner_id,
        lend_token_id,
        collateral_token_id,
        15000,
        9000,
        0,
        0,
        1000,
        1000,
    )
}