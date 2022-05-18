use near_sdk::json_types::{U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, AccountId};
use std::convert::{TryFrom};

use crate::*;

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct MetaData {
    pub min_days: u64,
    pub max_days: u64,
    pub max_time: u64,
    pub max_withdrawal_penalty: u64,
    pub precision: u64,
    pub locked_token: AccountId,
    pub penalty_collector: AccountId,
    pub min_locked_amount: U128,
    pub early_withdraw_penalty_rate: u64,
    pub supply: U128
}

#[near_bindgen]
impl Contract {
    /// Return contract basic info
    pub fn metadata(&self) -> MetaData {
        MetaData {
            min_days: MINDAYS,
            max_days: MAXDAYS,
            max_time: MAXTIME,
            max_withdrawal_penalty: MAX_WITHDRAWAL_PENALTY,
            precision: PRECISION,
            locked_token: self.locked_token.clone(),
            penalty_collector: self.penalty_collector.clone(),
            min_locked_amount: U128(self.min_locked_amount.clone()),
            early_withdraw_penalty_rate: self.early_withdraw_penalty_rate.into(),
            supply: self.supply.into()
        }
    }

    pub fn lock_of(&self, account_id: AccountId) -> U128 {
        self.lockeds.get(&account_id).unwrap_or_default().amount.into()
    }

    pub fn lock_end(&self, account_id: AccountId) -> u64 {
        self.lockeds.get(&account_id).unwrap_or_default().end
    }

    pub fn voting_power_unlock_time(&self, value: U128, unlock_time: u64) -> U128 {
        let now = env::block_timestamp();
        if unlock_time <= now {
            return U128(0);
        }
        let locked_seconds = unlock_time - now;
        if locked_seconds >= MAXTIME {
            return value;
        }
        let value_u128: u128 = value.into();
        U128(value_u128 * u128::try_from(locked_seconds).unwrap() / u128::try_from(MAXTIME).unwrap())
    }

    pub fn voting_power_locked_days(&self, value: U128, days: u64) -> U128 {
        if days > MAXDAYS {
            return value;
        }
        let value_u128: u128 = value.into();
        U128(value_u128 * u128::try_from(days).unwrap() / u128::try_from(MAXDAYS).unwrap()) 
    }

    pub fn get_locked_balance(&self, account_id: AccountId) -> LockedBalance {
        self.lockeds.get(&account_id).unwrap_or_default()
    }
}
