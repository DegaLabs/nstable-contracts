use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedMap};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, require, AccountId, Balance, BorshStorageKey, Gas,
    PanicOnDefault, Promise, StorageUsage,
};

use crate::*;
use utils::*;

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AccountDeposit {
    pub pool_id: u32,
    pub owner_id: AccountId,
    pub lend_token_id: AssetId,
    pub collateral_token_id: AssetId,
    pub deposits: UnorderedMap<AssetId, Balance>,
    pub borrow_amount: Balance,

    //lending interest when account is a lender
    pub lending_interest_profit_debt: Balance, //interest debt when in a lending position, this is similar to rewardDebt in sushi farming
    pub unpaid_lending_interest_profit: Balance,
    pub total_lending_interest_profit: Balance,
    pub last_lending_interest_reward_update_timestamp_sec: u64,

    //borrowing interest when accc is a borrower
    pub unpaid_borrowing_interest: Balance, //interest unpaid in a borrowing positions
    pub total_borrowing_interest: Balance,  //interest unpaid in a borrowing positions
    pub last_borrowing_interest_update_timestamp_sec: u64,
}

impl AccountDeposit {
    pub fn new(
        pool_id: u32,
        owner_id: AccountId,
        lend_token_id: AssetId,
        collateral_token_id: AssetId,
    ) -> AccountDeposit {
        let mut account_deposit = AccountDeposit {
            pool_id: pool_id.clone(),
            owner_id: owner_id.clone(),
            lend_token_id: lend_token_id.clone(),
            collateral_token_id: collateral_token_id.clone(),
            deposits: UnorderedMap::new(format!("d{}", pool_id.clone()).as_bytes()),
            borrow_amount: 0,
            lending_interest_profit_debt: 0,
            unpaid_lending_interest_profit: 0,
            total_lending_interest_profit: 0,
            last_lending_interest_reward_update_timestamp_sec: 0,
            unpaid_borrowing_interest: 0,
            total_borrowing_interest: 0,
            last_borrowing_interest_update_timestamp_sec: 0,
        };
        account_deposit.deposits.insert(&lend_token_id, &0u128);
        account_deposit
            .deposits
            .insert(&collateral_token_id, &0u128);
        account_deposit
    }

    pub fn deposit_collateral(&mut self, amount: &Balance) {
        let mut current_deposit = self
            .deposits
            .get(&self.collateral_token_id.clone())
            .unwrap_or(0);
        current_deposit = current_deposit + amount.clone();
        self.deposits
            .insert(&self.collateral_token_id, &current_deposit);
    }

    //this does not take care of interest when user in a borrowing position
    pub fn deposit_lend_token(&mut self, amount: &Balance, acc_interest_per_share: &Balance) {
        let mut current_deposit = self.get_token_deposit(&self.lend_token_id);

        self.last_lending_interest_reward_update_timestamp_sec = env::block_timestamp_ms() / 1000;
        let total_interest_reward = (U256::from(current_deposit)
            * U256::from(acc_interest_per_share.clone())
            / U256::from(ACC_INTEREST_PER_SHARE_MULTIPLIER))
        .as_u128();
        self.unpaid_lending_interest_profit +=
            total_interest_reward - self.lending_interest_profit_debt;
        self.total_lending_interest_profit += total_interest_reward - self.lending_interest_profit_debt;

        current_deposit = current_deposit + amount.clone();
        self.deposits.insert(&self.lend_token_id, &current_deposit);

        let total_interest_reward = (U256::from(current_deposit)
            * U256::from(acc_interest_per_share.clone())
            / U256::from(ACC_INTEREST_PER_SHARE_MULTIPLIER))
        .as_u128();
        self.lending_interest_profit_debt = total_interest_reward;
    }

    pub fn update_borrowing_interest(&mut self, interest_rate: u64) -> Balance {
        let mut interest = 0u128;
        if self.borrow_amount > 0 {
            interest = self.compute_unrecorded_interest(interest_rate);
            self.total_borrowing_interest += interest;
            self.unpaid_borrowing_interest += interest;
            self.last_borrowing_interest_update_timestamp_sec = env::block_timestamp_ms() / 1000;
        }
        interest
    }

    pub fn update_account(&mut self, interest_rate: u64, acc_interest_per_share: &Balance) {
        self.deposit_lend_token(&0u128, acc_interest_per_share);
        self.update_borrowing_interest(interest_rate);
    }

    pub fn borrow(
        &mut self,
        amount: &Balance,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        interest_rate: u64,
        min_cr: u64,
    ) -> Balance {
        self.update_borrowing_interest(interest_rate);

        let deposit_amount = self.get_token_deposit(&self.lend_token_id);
        let mut borrow_amount = amount.clone();
        if deposit_amount > 0 {
            if deposit_amount > amount.clone() {
                borrow_amount = 0;
                self.deposits
                    .insert(&self.lend_token_id, &(deposit_amount - amount.clone()));
            } else {
                borrow_amount = amount.clone() - deposit_amount;
                self.deposits.insert(&self.lend_token_id, &0u128);
            }
        }

        let max_borrowable = self.compute_max_borrowable(
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            interest_rate,
            min_cr,
        );
        if borrow_amount.clone() <= max_borrowable {
            self.borrow_amount += borrow_amount;
        }

        self.assert_collateral_ratio_valid_after_borrow(
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            interest_rate,
            min_cr,
        );
        borrow_amount
    }

    /// Asserts there is sufficient amount of $NEAR to cover storage usage.
    pub fn assert_collateral_ratio_valid_after_borrow(
        &self,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        interest_rate: u64,
        min_cr: u64,
    ) {
        let cr = compute_cr(
            self.get_token_deposit(&self.collateral_token_id),
            collateral_token_info.decimals,
            collateral_token_price,
            self.borrow_amount + self.get_interest_owed(interest_rate),
            lend_token_price,
            lend_token_info.decimals,
        );
        assert!(min_cr <= cr, "{}", "collateral ratio after borrow too low");
    }

    //return the amount withdrawn from the lend deposit token or collateral token, excluding interest if have
    pub fn withdraw_from_account(
        &mut self,
        token_id: &AccountId,
        amount: Balance,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        interest_rate: u64,
        min_cr: u64,
    ) -> Balance {
        let deposit_amount = self.get_token_deposit(token_id);
        if token_id.clone() == self.lend_token_id {
            let unpaid_lending_interest_profit = self.unpaid_lending_interest_profit;
            require!(
                amount <= deposit_amount + unpaid_lending_interest_profit.clone(),
                "insufficient amount for withdrawal"
            );
            let mut remain = amount.clone();
            if remain >= unpaid_lending_interest_profit {
                self.unpaid_lending_interest_profit = 0;
                remain = remain - amount.clone();

                self.deposits
                    .insert(&self.lend_token_id, &(deposit_amount - remain));
                return remain;
            } else {
                self.unpaid_lending_interest_profit -= remain;
                return 0;
            }
        } else if token_id.clone() == self.collateral_token_id {
            require!(
                amount <= deposit_amount,
                "insufficient amount for withdrawal"
            );
            let collateral_after_withdrawal = deposit_amount - amount;
            if self.borrow_amount + self.get_interest_owed(interest_rate) > 0 {
                let cr = compute_cr(
                    collateral_after_withdrawal,
                    collateral_token_info.decimals,
                    collateral_token_price,
                    self.borrow_amount + self.get_interest_owed(interest_rate),
                    lend_token_price,
                    lend_token_info.decimals,
                );
                assert!(
                    min_cr <= cr,
                    "{}",
                    "collateral ratio after withdrawal too low"
                );
            }

            self.deposits.insert(token_id, &collateral_after_withdrawal);
            return amount;
        } else {
            env::panic_str("invalid token");
        }
    }

    pub fn compute_unrecorded_interest(&self, interest_rate: u64) -> Balance {
        if self.borrow_amount == 0 {
            return 0u128;
        }
        let last_borrowing_interest_update_timestamp_sec =
            self.last_borrowing_interest_update_timestamp_sec.clone();
        let current_time_sec = env::block_timestamp_ms() / 1000;
        let interest = self.borrow_amount
            * (((current_time_sec - last_borrowing_interest_update_timestamp_sec) * interest_rate)
                as u128)
            / (INTEREST_RATE_DIVISOR * SECONDS_PER_YEAR);
        interest
    }

    //this does not take care of interest when user in a borrowing position
    pub fn deposit_lend_token_with_taking_interest(
        &mut self,
        amount: &Balance,
        interest_rate: u64,
        acc_interest_per_share: &Balance,
    ) -> Balance {
        if self.borrow_amount > 0 {
            self.update_borrowing_interest(interest_rate);
            self.deposit_lend_token(amount, acc_interest_per_share);
        } else {
            self.deposit_lend_token(amount, acc_interest_per_share);
        }
        amount.clone()
    }

    pub fn pay_loan(&mut self, pay_amount: Balance, acc_interest_per_share: Balance) -> (Balance, Balance) {
        let mut actual_borrow_paid = self.borrow_amount.clone();
        let mut remain = pay_amount.clone();
        if self.unpaid_borrowing_interest > 0 {
            if self.unpaid_borrowing_interest > remain {
                self.unpaid_borrowing_interest -= remain;
            } else {
                self.unpaid_borrowing_interest = 0;
                remain -= self.unpaid_borrowing_interest;
            }
        }

        if self.borrow_amount > 0 {
            if self.borrow_amount > remain {
                self.borrow_amount -= remain;
            } else {
                self.borrow_amount = 0;
                remain -= self.borrow_amount;
            }
        }

        actual_borrow_paid -= self.borrow_amount;

        if remain > 0 {
            self.deposit_lend_token(&remain, &acc_interest_per_share);
        }

        (actual_borrow_paid, remain)
    }

    pub fn get_token_deposit(&self, token_id: &AccountId) -> Balance {
        self.deposits.get(token_id).unwrap_or(0u128)
    }

    pub fn get_interest_owed(&self, interest_rate: u64) -> Balance {
        let mut unpaid_interest = self.unpaid_borrowing_interest;
        if self.borrow_amount > 0 {
            unpaid_interest += self.compute_unrecorded_interest(interest_rate);
        }
        unpaid_interest
    }

    pub fn compute_max_borrowable(
        &self,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        interest_rate: u64,
        cr: u64,
    ) -> Balance {
        let lend_token_deposit = self.get_token_deposit(&self.lend_token_id);
        let collateral_token_deposit = self.get_token_deposit(&self.collateral_token_id);

        let lend_token_value = compute_token_value(lend_token_deposit.clone(), lend_token_price);
        let collateral_token_value =
            compute_token_value(collateral_token_deposit.clone(), collateral_token_price);

        let max_borrowable = collateral_token_value
            * U256::from(10u128.pow(lend_token_info.decimals as u32))
            * U256::from(10u128.pow(lend_token_info.decimals as u32))
            / (U256::from(10u128.pow(collateral_token_info.decimals as u32)) * lend_token_value);
        let mut max_borrowable = max_borrowable.as_u128();
        max_borrowable = max_borrowable * COLLATERAL_RATIO_DIVISOR / (cr as u128);
        let interest_owed = self.get_interest_owed(interest_rate);

        let owed = self.borrow_amount + interest_owed;

        if owed >= max_borrowable {
            return 0u128;
        }
        max_borrowable - owed
    }
}