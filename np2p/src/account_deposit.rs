use crate::*;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, log, require, AccountId, Balance, PanicOnDefault};
use std::collections::HashMap;
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
        let account_deposit = AccountDeposit {
            pool_id: pool_id.clone(),
            owner_id: owner_id.clone(),
            lend_token_id: lend_token_id.clone(),
            collateral_token_id: collateral_token_id.clone(),
            deposits: UnorderedMap::new(StorageKey::AccountDeposit {
                pool_id: pool_id.clone(),
                account_id: owner_id.clone(),
            }),
            borrow_amount: 0,
            lending_interest_profit_debt: 0,
            unpaid_lending_interest_profit: 0,
            total_lending_interest_profit: 0,
            last_lending_interest_reward_update_timestamp_sec: 0,
            unpaid_borrowing_interest: 0,
            total_borrowing_interest: 0,
            last_borrowing_interest_update_timestamp_sec: 0,
        };
        account_deposit
    }

    pub fn internal_deposit_collateral(&mut self, amount: &Balance) {
        log!("updating collateral for account {}", self.owner_id);
        let mut current_deposit = self
            .deposits
            .get(&self.collateral_token_id.clone())
            .unwrap_or(0);
        current_deposit = current_deposit + amount.clone();
        self.deposits
            .insert(&self.collateral_token_id, &current_deposit);
    }

    //this does not take care of interest when user in a borrowing position
    pub fn internal_deposit_lend_token(&mut self, amount: &Balance) {
        let mut current_deposit = self.get_token_deposit(&self.lend_token_id);
        current_deposit = current_deposit + amount.clone();
        self.deposits.insert(&self.lend_token_id, &current_deposit);
    }

    pub fn update_borrowing_interest(&mut self, interest_rate: u64) -> Balance {
        let mut interest = 0u128;
        if self.borrow_amount > 0 {
            interest = self.compute_unrecorded_interest(interest_rate);
            self.total_borrowing_interest += interest;
            self.unpaid_borrowing_interest += interest;
        }
        self.last_borrowing_interest_update_timestamp_sec = get_next_interest_recal_time_sec();
        interest
    }

    pub fn update_account(&mut self, interest_rate: u64, acc_interest_per_share: &Balance) {
        log!("updating account {}", self.owner_id);
        self.update_lending_profit(interest_rate, acc_interest_per_share);
        self.update_borrowing_interest(interest_rate);
        log!("updating account {} done", self.owner_id);
    }

    fn update_lending_profit(&mut self, _interest_rate: u64, acc_interest_per_share: &Balance) {
        let current_deposit = self.get_token_deposit(&self.lend_token_id);
        self.last_lending_interest_reward_update_timestamp_sec = env::block_timestamp_ms() / 1000;
        let total_interest_reward = (U256::from(current_deposit)
            * U256::from(acc_interest_per_share.clone())
            / U256::from(ACC_INTEREST_PER_SHARE_MULTIPLIER))
        .as_u128();
        self.unpaid_lending_interest_profit +=
            total_interest_reward - self.lending_interest_profit_debt;
        self.total_lending_interest_profit +=
            total_interest_reward - self.lending_interest_profit_debt;
        self.lending_interest_profit_debt = total_interest_reward;
    }

    pub fn update_lending_interest_profit_debt(&mut self, acc_interest_per_share: &Balance) {
        log!("update_lending_interest_profit_debt account {}", self.owner_id);
        let current_deposit = self.get_token_deposit(&self.lend_token_id);
        self.lending_interest_profit_debt = (U256::from(current_deposit)
            * U256::from(acc_interest_per_share.clone())
            / U256::from(ACC_INTEREST_PER_SHARE_MULTIPLIER))
        .as_u128();
        log!("update_lending_interest_profit_debt account {} done", self.owner_id);
    }

    pub fn internal_borrow(
        &mut self,
        amount: &Balance,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        interest_rate: u64,
        min_cr: u64,
    ) -> Balance {
        let deposit_amount = self.get_token_deposit(&self.lend_token_id);
        let mut borrow_amount = amount.clone();
        log!("borrow_amount {}", borrow_amount);
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
        log!("borrow_amount actual {}", borrow_amount);

        let max_borrowable = self.compute_max_borrowable(
            lend_token_info,
            lend_token_price,
            collateral_token_info,
            collateral_token_price,
            None,
            interest_rate,
            min_cr,
        );
        log!("max_borrowable actual {}", max_borrowable);
        require!(
            borrow_amount.clone() <= max_borrowable,
            "exceed max borrowable"
        );

        self.borrow_amount += borrow_amount;
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
        if self.borrow_amount > 0 {
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
    }

    pub fn compute_current_cr(
        &self,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        collateral_amount: Option<Balance>,
        borrow: Option<Balance>,     //borrow more
        pay_amount: Option<Balance>, //pay back
        interest_rate: u64,
    ) -> u64 {
        let collateral_amount = collateral_amount.unwrap_or(0);
        let borrow = borrow.unwrap_or(0);
        let pay_amount = pay_amount.unwrap_or(0);
        let cr = compute_cr(
            self.get_token_deposit(&self.collateral_token_id) + collateral_amount.clone(),
            collateral_token_info.decimals,
            collateral_token_price,
            self.borrow_amount + self.get_interest_owed(interest_rate) + borrow.clone()
                - pay_amount.clone(),
            lend_token_price,
            lend_token_info.decimals,
        );
        cr
    }

    //return the amount withdrawn from the lend deposit token or collateral token, excluding interest if have
    pub fn internal_withdraw_from_account(
        &mut self,
        token_id: &AccountId,
        amount: Balance,
        lend_token_info: &TokenInfo,
        lend_token_price: &Price,
        collateral_token_info: &TokenInfo,
        collateral_token_price: &Price,
        interest_rate: u64,
        acc_interest_per_share: &Balance,
        min_cr: u64,
    ) -> Balance {
        let deposit_amount = self.get_token_deposit(token_id);
        if token_id.clone() == self.lend_token_id {
            let unpaid_lending_interest_profit = self.unpaid_lending_interest_profit;
            require!(
                amount <= deposit_amount + unpaid_lending_interest_profit.clone(),
                format!(
                    "user has insufficient asset {} for withdrawal",
                    token_id.clone()
                )
            );
            let mut remain = amount.clone();
            if remain >= unpaid_lending_interest_profit {
                remain = remain - unpaid_lending_interest_profit.clone();
                self.unpaid_lending_interest_profit = 0;
                log!("withdrawing {}, {}", deposit_amount, remain);
                self.deposits
                    .insert(&self.lend_token_id, &(deposit_amount - remain));
                self.update_lending_profit(interest_rate, acc_interest_per_share);
                return remain;
            } else {
                self.unpaid_lending_interest_profit -= remain;
                return 0;
            }
        } else if token_id.clone() == self.collateral_token_id {
            require!(
                amount <= deposit_amount,
                format!(
                    "user has insufficient asset {} for withdrawal",
                    token_id.clone()
                )
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
            self.update_lending_profit(interest_rate, acc_interest_per_share);
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
        let current_time_sec = get_next_interest_recal_time_sec();
        let interest = self.borrow_amount
            * (((current_time_sec - last_borrowing_interest_update_timestamp_sec) * interest_rate)
                as u128)
            / (INTEREST_RATE_DIVISOR * SECONDS_PER_YEAR);
        interest
    }

    //this does not take care of interest when user in a borrowing position
    // pub fn internal_deposit_lend_token_with_taking_interest(
    //     &mut self,
    //     amount: &Balance,
    //     interest_rate: u64,
    //     acc_interest_per_share: &Balance,
    // ) -> Balance {
    //     if self.borrow_amount > 0 {
    //         self.update_borrowing_interest(interest_rate);
    //         self.internal_deposit_lend_token(amount, interest_rate, acc_interest_per_share);
    //     } else {
    //         self.internal_deposit_lend_token(amount, interest_rate, acc_interest_per_share);
    //     }
    //     amount.clone()
    // }

    pub fn internal_pay_loan(&mut self, pay_amount: Balance) -> (Balance, Balance) {
        log!("updating account");
        let mut actual_borrow_paid = self.borrow_amount.clone();
        let mut remain = pay_amount.clone();
        if self.unpaid_borrowing_interest > 0 {
            if self.unpaid_borrowing_interest > remain {
                self.unpaid_borrowing_interest -= remain;
                remain = 0;
            } else {
                remain -= self.unpaid_borrowing_interest;
                log!(
                    "unpaid_borrowing_interest {}",
                    self.unpaid_borrowing_interest
                );
                self.unpaid_borrowing_interest = 0;
            }
        }
        log!("borrow_amount before {}", self.borrow_amount);
        if self.borrow_amount > 0 {
            if self.borrow_amount > remain {
                self.borrow_amount -= remain;
                remain = 0;
            } else {
                remain -= self.borrow_amount;
                self.borrow_amount = 0;
            }
        }
        log!("borrow_amount after {}", self.borrow_amount);
        actual_borrow_paid -= self.borrow_amount;
        log!("actual_borrow_paid {}", actual_borrow_paid);

        self.reduce_lend_token_deposit(pay_amount.clone());

        if remain > 0 {
            self.internal_deposit_lend_token(&remain);
        }

        (actual_borrow_paid, remain)
    }

    pub fn reduce_collateral(
        &mut self,
        amount: Balance
    ) {
        let collateral_amount = self.get_token_deposit(&self.collateral_token_id);
        require!(amount <= collateral_amount, "!reduce_collateral");
        self.deposits
            .insert(&self.collateral_token_id, &(collateral_amount - amount));
    }

    pub fn reduce_lend_token_deposit(&mut self, amount: Balance) {
        let lend_token_deposit = self.get_token_deposit(&self.lend_token_id);
        require!(amount <= lend_token_deposit, "!reduce_lend_token_deposit");
        self.deposits
            .insert(&self.lend_token_id, &(lend_token_deposit - amount));
    }

    pub fn get_token_deposit(&self, token_id: &AccountId) -> Balance {
        self.deposits.get(token_id).unwrap_or(0u128)
    }

    pub fn get_deposits(&self) -> HashMap<AssetId, U128> {
        let mut ret = HashMap::<AssetId, U128>::new();
        ret.insert(
            self.lend_token_id.clone(),
            U128(self.get_token_deposit(&self.lend_token_id)),
        );
        ret.insert(
            self.collateral_token_id.clone(),
            U128(self.get_token_deposit(&self.collateral_token_id)),
        );
        ret
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
        additional_collateral: Option<Balance>,
        interest_rate: u64,
        cr: u64,
    ) -> Balance {
        let mut collateral_token_deposit = self.get_token_deposit(&self.collateral_token_id);
        let additional_collateral = additional_collateral.unwrap_or(0);
        collateral_token_deposit += additional_collateral.clone();

        let collateral_token_value =
            compute_token_value(collateral_token_deposit.clone(), collateral_token_price);

        let max_borrowable = collateral_token_value
            * U256::from(10u128.pow(lend_token_info.decimals as u32))
            * U256::from(10u128.pow(lend_token_price.decimals as u32))
            / (U256::from(10u128.pow(collateral_token_info.decimals as u32))
                * U256::from(lend_token_price.multiplier.0));

        let mut max_borrowable = max_borrowable.as_u128();
        max_borrowable = max_borrowable * COLLATERAL_RATIO_DIVISOR / (cr as u128);
        let interest_owed = self.get_interest_owed(interest_rate);

        let owed = self.borrow_amount + interest_owed;

        if owed >= max_borrowable {
            return 0u128;
        }
        max_borrowable - owed
    }

    pub fn get_owed_lend_token_amount(&self) -> Balance {
        self.borrow_amount + self.unpaid_borrowing_interest
    }

    pub fn get_pending_unpaid_lending_interest_profit(
        &self,
        acc_interest_per_share: &Balance,
    ) -> Balance {
        let current_deposit = self.get_token_deposit(&self.lend_token_id);

        let total_interest_reward = (U256::from(current_deposit)
            * U256::from(acc_interest_per_share.clone())
            / U256::from(ACC_INTEREST_PER_SHARE_MULTIPLIER))
        .as_u128();
        let unpaid_lending_interest_profit = self.unpaid_lending_interest_profit
            + total_interest_reward
            - self.lending_interest_profit_debt;
        return unpaid_lending_interest_profit;
    }

    pub fn get_total_interest_reward(&self, acc_interest_per_share: &Balance) -> Balance {
        let current_deposit = self.get_token_deposit(&self.lend_token_id);

        let total_interest_reward = (U256::from(current_deposit)
            * U256::from(acc_interest_per_share.clone())
            / U256::from(ACC_INTEREST_PER_SHARE_MULTIPLIER))
        .as_u128();
        total_interest_reward
    }

    pub fn get_pending_total_lending_interest_profit(
        &self,
        acc_interest_per_share: &Balance,
    ) -> Balance {
        let current_deposit = self.get_token_deposit(&self.lend_token_id);

        let total_interest_reward = (U256::from(current_deposit)
            * U256::from(acc_interest_per_share.clone())
            / U256::from(ACC_INTEREST_PER_SHARE_MULTIPLIER))
        .as_u128();
        let total_lending_interest_profit = self.total_lending_interest_profit
            + total_interest_reward
            - self.lending_interest_profit_debt;
        return total_lending_interest_profit;
    }

    pub fn get_pending_unpaid_borrowing_interest(&self, interest_rate: u64) -> Balance {
        let unrecorded = self.compute_unrecorded_interest(interest_rate);
        self.unpaid_borrowing_interest + unrecorded
    }

    pub fn get_pending_total_borrowing_interest(&self, interest_rate: u64) -> Balance {
        let unrecorded = self.compute_unrecorded_interest(interest_rate);
        self.total_borrowing_interest + unrecorded
    }
}
