use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, log, require, AccountId, Balance, PanicOnDefault};
use std::collections::HashMap;

use crate::*;
use utils::*;

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Liquidation {
    pool_id: u32,
    liquidated_account_id: AccountId,
    liquidator_account_id: AccountId,
    lend_token_id: AssetId,
    collateral_token_id: AssetId,
    liquidated_collateral_amount_before: U128,
    liquidated_collateral_amount_after: U128,
    borrowed_before: U128,
    borrowed_after: U128,
    timestamp_sec: u64,
    liquidator_collateral_amount_received: U128,
    treasury_collateral_amount_received: U128,
    liquidation_price: Price, //price with liquidation fee
    price: Price,             //price before liquidation fee
}

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
    pub liquidation_bonus: u64, //or price penalty, it means how much discount liquidators can buy collateral tokens using lend token to pay the debt
    pub liquidation_history: Vec<Liquidation>,
}

impl Pool {
    pub fn new(
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
            liquidation_history: vec![],
        }
    }

    pub fn internal_deposit(
        &mut self,
        account_id: &AccountId,
        token_id: &AssetId,
        amount: Balance,
    ) {
        if token_id.clone() == self.lend_token_id.clone() {
            require!(
                amount >= self.min_lend_token_deposit,
                "lower than min deposit"
            );
        }
        log!("reading account deposit {} for token {}, lend {}, collateral {}", account_id, token_id, self.lend_token_id, self.collateral_token_id);
        let mut account_deposit = self.get_account_deposit_or_revert(account_id);
        log!("success reading account deposit {} from pool {}", account_id.clone(), self.pool_id);
        if token_id.clone() == self.lend_token_id {
            //update acc_interest_per_share
            log!("update_acc_interest_per_share");
            self.update_acc_interest_per_share();
            account_deposit.internal_deposit_lend_token(
                &amount,
                self.fixed_interest_rate,
                &self.acc_interest_per_share,
            );
            self.total_lend_asset_deposit += amount;
        } else if token_id.clone() == self.collateral_token_id {
            account_deposit.internal_deposit_collateral(
                &amount,
                self.fixed_interest_rate,
                &self.acc_interest_per_share,
            );
            self.total_collateral_deposit += amount;
        } else {
            env::panic_str("unsupported token for pool")
        }
        self.account_deposits.insert(account_id, &account_deposit);
    }

    pub fn get_account_deposit(&self, account_id: &AccountId) -> AccountDeposit {
        if self.account_deposits.get(account_id).is_some() {
            return self.account_deposits.get(account_id).unwrap();
        }
        AccountDeposit::new(
            self.pool_id.clone(),
            account_id.clone(),
            self.lend_token_id.clone(),
            self.collateral_token_id.clone(),
        )
    }

    pub fn get_account_deposit_or_revert(&self, account_id: &AccountId) -> AccountDeposit {
        self.account_deposits
            .get(account_id)
            .expect("account not registered")
    }
    pub fn internal_borrow(
        &mut self,
        account_id: &AccountId,
        amount: &Balance,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
    ) -> Balance {
        self.update_acc_interest_per_share();

        require!(
            amount.clone() >= self.min_lend_token_borrow,
            "lower than min borrow"
        );
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

        let mut account_deposit = self.get_account_deposit_or_revert(account_id);
        let actual_borrow_amount = account_deposit.internal_borrow(
            amount,
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            self.fixed_interest_rate,
            &self.acc_interest_per_share,
            self.min_cr,
        );
        log!("pool actual_borrow_amount {}", actual_borrow_amount);
        self.account_deposits.insert(account_id, &account_deposit);
        if actual_borrow_amount != amount.clone() {
            self.total_lend_asset_deposit -= amount.clone() - actual_borrow_amount;
            self.total_borrow += actual_borrow_amount.clone();
        } else {
            self.total_borrow += amount.clone();
        }
        actual_borrow_amount
    }

    pub fn internal_register_account_if_not(&mut self, account_id: &AccountId) {
        log!(
            "creating default account {}, {}, {}",
            account_id.clone(),
            self.lend_token_id.clone(),
            self.collateral_token_id.clone()
        );
        if self.account_deposits.get(account_id).is_none() {
            let account_deposit = AccountDeposit::new(
                self.pool_id,
                account_id.clone(),
                self.lend_token_id.clone(),
                self.collateral_token_id.clone(),
            );
            log!("done creating default account");
            self.account_deposits.insert(account_id, &account_deposit);
        }
    }

    pub fn internal_withdraw_from_account(
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
        let mut account_deposit = self.get_account_deposit_or_revert(account_id);
        log!("balance before withdraw {}", account_deposit.get_token_deposit(token_id));
        let withdrawn_amount_from_deposit = account_deposit.internal_withdraw_from_account(
            token_id,
            amount.clone(),
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            self.fixed_interest_rate,
            &self.acc_interest_per_share,
            self.min_cr,
        );
        if token_id.clone() == self.lend_token_id.clone() {
            log!("withdraw {} of {}, of which {} is extracted from generated lending interest", amount.clone(), token_id.clone(), amount.clone() - withdrawn_amount_from_deposit.clone());
        }
        self.account_deposits.insert(account_id, &account_deposit);
        log!("balance after withdraw {}", account_deposit.get_token_deposit(token_id));

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

    pub fn internal_pay_loan(&mut self, account_id: &AccountId, pay_amount: Balance) {
        self.update_acc_interest_per_share();
        let mut account_deposit = self.get_account_deposit_or_revert(account_id);
        log!("pool: paying amount {}", pay_amount);
        let (paid_borrow, added_liquidity) = account_deposit.internal_pay_loan(
            pay_amount,
            self.fixed_interest_rate,
            self.acc_interest_per_share.clone(),
        );
        self.account_deposits.insert(account_id, &account_deposit);

        self.total_borrow -= paid_borrow;
        self.total_lend_asset_deposit =
            self.total_lend_asset_deposit + added_liquidity - pay_amount;
    }

    pub fn internal_liquidate(
        &mut self,
        liquidated_account_id: AccountId,
        liquidated_borrow_amount: Balance,
        liquidator_account_id: AccountId,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        foundation_id: AccountId,
        liquidation_marginal: u64,
    ) {
        self.update_acc_interest_per_share();

        let mut liquidator_account_deposit =
            self.get_account_deposit_or_revert(&liquidator_account_id);
        liquidator_account_deposit
            .update_account(self.fixed_interest_rate, &self.acc_interest_per_share);
        let liquidator_account_lend_token_deposit_amount =
            liquidator_account_deposit.get_token_deposit(&self.lend_token_id);
        require!(
            liquidator_account_lend_token_deposit_amount >= liquidated_borrow_amount,
            "liquidation maker insufficient deposit"
        );

        require!(
            liquidator_account_deposit.get_owed_lend_token_amount() == 0,
            "cannot liquidate if liquidator maker owes the pool"
        );

        let mut liquidated_account_deposit =
            self.get_account_deposit_or_revert(&liquidated_account_id);
        let borrowed_before = liquidated_account_deposit.borrow_amount;
        liquidated_account_deposit
            .update_account(self.fixed_interest_rate, &self.acc_interest_per_share);
        let liquidated_account_cr = self.compute_current_cr(
            liquidated_account_id.clone(),
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            None,
            None,
            None,
        );
        require!(
            liquidated_account_cr < self.min_cr,
            "Liquidated account is not under collateral ratio for liquidation"
        );

        let liquidated_borrow_value =
            compute_token_value(liquidated_borrow_amount.clone(), lend_token_price);
        let mut collateral_token_price_after_bonus = collateral_token_price.clone();
        collateral_token_price_after_bonus.multiplier = U128(
            collateral_token_price_after_bonus.multiplier.0
                * (LIQUIDATION_BONUS_DIVISOR - (self.liquidation_bonus as u128))
                / LIQUIDATION_BONUS_DIVISOR,
        );
        let to_liquidate_collateral_amount =
            U256::from(10u128.pow(collateral_token_info.decimals as u32))
                * liquidated_borrow_value
                * U256::from(10u128.pow(collateral_token_price_after_bonus.decimals as u32))
                / (U256::from(collateral_token_price_after_bonus.multiplier.0)
                    * U256::from(10u128.pow(lend_token_info.decimals as u32)));
        let to_liquidate_collateral_amount = to_liquidate_collateral_amount.as_u128();
        let to_liquidate_collateral_amount_to_cover_liquidator =
            U256::from(10u128.pow(collateral_token_info.decimals as u32))
                * liquidated_borrow_value
                * U256::from(10u128.pow(collateral_token_price.decimals as u32))
                / (U256::from(collateral_token_price.multiplier.0)
                    * U256::from(10u128.pow(lend_token_info.decimals as u32)));
        let to_liquidate_collateral_amount_to_cover_liquidator =
            to_liquidate_collateral_amount_to_cover_liquidator.as_u128();

        let (actual_borrow_paid, remain) = liquidated_account_deposit.internal_pay_loan(
            liquidated_borrow_amount,
            self.fixed_interest_rate,
            self.acc_interest_per_share,
        );

        let liquidated_collateral_amount_before =
            liquidated_account_deposit.get_token_deposit(&self.collateral_token_id);
        liquidated_account_deposit.reduce_collateral(to_liquidate_collateral_amount.clone(), self.fixed_interest_rate, &self.acc_interest_per_share);
        liquidator_account_deposit.reduce_lend_token_deposit(liquidated_borrow_amount);

        self.total_lend_asset_deposit =
            self.total_lend_asset_deposit - liquidated_borrow_amount + remain;
        self.total_borrow = self.total_borrow - actual_borrow_paid;

        let liquidated_account_cr = self.compute_current_cr(
            liquidated_account_id.clone(),
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            None,
            None,
            None,
        );
        require!(
            liquidated_account_cr < self.min_cr,
            "invalid collateral ratio after liquidation"
        );

        let collateral_to_foundation = (to_liquidate_collateral_amount
            - to_liquidate_collateral_amount_to_cover_liquidator)
            * (liquidation_marginal as u128)
            / LIQUIDATION_MARGINAL_DIVISOR;
        let collateral_to_liquidator =
            to_liquidate_collateral_amount - collateral_to_foundation.clone();
        liquidator_account_deposit.internal_deposit_collateral(
            &collateral_to_liquidator,
            self.fixed_interest_rate,
            &self.acc_interest_per_share,
        );

        let mut foundation_account_deposit = self.get_account_deposit_or_revert(&foundation_id);
        foundation_account_deposit.internal_deposit_collateral(
            &collateral_to_foundation,
            self.fixed_interest_rate,
            &self.acc_interest_per_share,
        );

        //save accounts
        self.account_deposits
            .insert(&liquidated_account_id, &liquidated_account_deposit);
        self.account_deposits
            .insert(&liquidator_account_id, &liquidator_account_deposit);
        self.account_deposits
            .insert(&foundation_id, &foundation_account_deposit);

        let liquidated_collateral_amount_after =
            liquidated_account_deposit.get_token_deposit(&self.collateral_token_id);
        let borrowed_after = liquidated_account_deposit.borrow_amount;

        let liquidaion_history = Liquidation {
            pool_id: self.pool_id,
            liquidated_account_id: liquidated_account_id,
            liquidator_account_id: liquidator_account_id,
            lend_token_id: self.lend_token_id.clone(),
            collateral_token_id: self.collateral_token_id.clone(),
            liquidated_collateral_amount_before: U128(liquidated_collateral_amount_before),
            liquidated_collateral_amount_after: U128(liquidated_collateral_amount_after),
            borrowed_before: U128(borrowed_before),
            borrowed_after: U128(borrowed_after),
            timestamp_sec: env::block_timestamp_ms() / 1000,
            liquidator_collateral_amount_received: U128(collateral_to_liquidator),
            treasury_collateral_amount_received: U128(collateral_to_foundation),
            liquidation_price: collateral_token_price_after_bonus, //price with liquidation fee
            price: collateral_token_price.clone(),
        };
        self.liquidation_history.push(liquidaion_history);
    }

    pub fn compute_current_cr(
        &self,
        account_id: AccountId,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        collateral_amount: Option<Balance>,
        borrow: Option<Balance>,     //borrow more
        pay_amount: Option<Balance>, //pay back
    ) -> u64 {
        if self.account_deposits.get(&account_id).is_none() {
            return compute_cr(
                0,
                collateral_token_info.decimals,
                collateral_token_price,
                0,
                lend_token_price,
                lend_token_info.decimals,
            );
        }

        let account_deposit = self.get_account_deposit(&account_id);
        account_deposit.compute_current_cr(
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            collateral_amount,
            borrow,
            pay_amount,
            self.fixed_interest_rate,
        )
    }

    pub fn compute_max_borrowable_for_account(
        &self,
        account_id: &AccountId,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        additional_collateral_amount: Option<Balance>,
    ) -> Balance {
        if self.account_deposits.get(account_id).is_none() {
            return 0;
        }
        let account_deposit = self.get_account_deposit(account_id);
        let max_borrowable = account_deposit.compute_max_borrowable(
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            additional_collateral_amount,
            self.fixed_interest_rate,
            self.min_cr,
        );
        let max_of_pool = self.get_max_borrowable_of_pool();
        if max_borrowable > max_of_pool {
            return max_of_pool;
        }
        max_borrowable
    }

    pub fn get_max_borrowable_of_pool(&self) -> Balance {
        let max_of_pool =
            self.total_lend_asset_deposit * self.max_utilization as u128 / UTILIZATION_DIVISOR;
        max_of_pool - self.total_borrow
    }

    pub fn get_deposits(&self, account_id: &AccountId) -> HashMap<AssetId, U128> {
        if self.account_deposits.get(account_id).is_none() {
            return HashMap::new();
        }
        let account_deposit = self.get_account_deposit(account_id);
        account_deposit.get_deposits()
    }

    pub fn get_current_acc_interest_per_share(&self) -> Balance {
        if self.total_lend_asset_deposit == 0 {
            return self.acc_interest_per_share;
        }
        let mut last_acc_interest_update_timestamp_sec =
            self.last_acc_interest_update_timestamp_sec;
        if self.total_borrow == 0 {
            last_acc_interest_update_timestamp_sec = env::block_timestamp_ms() / 1000;
        }
        let elapsed_time =
            env::block_timestamp_ms() / 1000 - last_acc_interest_update_timestamp_sec;
        let elapsed_time = elapsed_time as u128;
        let generated_interest =
            self.total_borrow * elapsed_time * (self.fixed_interest_rate as u128)
                / (SECONDS_PER_YEAR * INTEREST_RATE_DIVISOR);
        return self.acc_interest_per_share
            + generated_interest * ACC_INTEREST_PER_SHARE_MULTIPLIER
                / self.total_lend_asset_deposit;
    }

    pub fn get_pending_unpaid_lending_interest_profit(&self, account_id: &AccountId) -> Balance {
        if self.account_deposits.get(account_id).is_none() {
            return 0;
        }
        let account_deposit = self.get_account_deposit(account_id);
        account_deposit
            .get_pending_unpaid_lending_interest_profit(&self.get_current_acc_interest_per_share())
    }

    pub fn get_pending_total_lending_interest_profit(&self, account_id: &AccountId) -> Balance {
        if self.account_deposits.get(account_id).is_none() {
            return 0;
        }
        let account_deposit = self.get_account_deposit(account_id);
        account_deposit
            .get_pending_total_lending_interest_profit(&self.get_current_acc_interest_per_share())
    }

    pub fn get_total_interest_reward(&self, account_id: &AccountId) -> Balance {
        if self.account_deposits.get(account_id).is_none() {
            return 0;
        }
        let account_deposit = self.get_account_deposit(account_id);
        account_deposit.get_total_interest_reward(&self.get_current_acc_interest_per_share())
    }

    pub fn get_pending_unpaid_borrowing_interest(&self, account_id: &AccountId) -> Balance {
        if self.account_deposits.get(account_id).is_none() {
            return 0;
        }
        let account_deposit = self.get_account_deposit(account_id);
        account_deposit.get_pending_unpaid_borrowing_interest(self.fixed_interest_rate)
    }

    pub fn compute_unrecorded_interest(&self, account_id: &AccountId) -> Balance {
        if self.account_deposits.get(account_id).is_none() {
            return 0;
        }
        let account_deposit = self.get_account_deposit(account_id);
        account_deposit.compute_unrecorded_interest(self.fixed_interest_rate)
    }

    pub fn get_pending_total_borrowing_interest(&self, account_id: &AccountId) -> Balance {
        if self.account_deposits.get(account_id).is_none() {
            return 0;
        }
        let account_deposit = self.get_account_deposit(account_id);
        account_deposit.get_pending_total_borrowing_interest(self.fixed_interest_rate)
    }

    pub fn get_token_deposit(&self, account_id: &AccountId, token_id: &AssetId) -> Balance {
        if self.account_deposits.get(account_id).is_none() {
            return 0;
        }
        let account_deposit = self.get_account_deposit(account_id);
        account_deposit.get_token_deposit(token_id)
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

