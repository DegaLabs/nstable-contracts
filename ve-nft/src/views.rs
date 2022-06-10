use crate::*;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, AccountId};
use std::convert::TryFrom;

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
    pub total_deposit: Balance,
    pub total_locked: Balance,
    pub voting_power_supply: Balance
}

#[near_bindgen]
impl Contract {
    /// Return contract basic info
    pub fn ve_metadata(&self) -> MetaData {
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
            total_deposit: self.total_deposit.into(),
            total_locked: self.total_locked.into(),
            voting_power_supply: self.voting_power_supply.into()
        }
    }

    pub fn get_deposit(&self, account_id: AccountId) -> U128 {
        U128(self.deposits.get(&account_id).unwrap_or(0))
    }

    pub fn get_token_ve_metadata(&self, token_id: TokenId) -> LockInfo {
        let token = match self.tokens.nft_token(token_id.clone()) {
            Some(t) => t,
            None => env::panic_str("no token found"),
        };

        let metadata = token.metadata.unwrap();
        let lock_info = self.unwrap_metadata(&metadata);
        lock_info
    }

    pub fn get_token_metadata_for_account(&self, account_id: AccountId) -> (Vec<TokenId>, Vec<LockInfo>, U128, U128, U128) {
        let mut ret: Vec<LockInfo> = vec![];
        let mut token_ids: Vec<TokenId> = vec![];
        let tokens = self.tokens.nft_tokens_for_owner(account_id.clone(), None, None);
        let deposited: Balance = self.deposits.get(&account_id).unwrap_or(0);
        let mut locked: Balance = 0;
        let mut voting_power: Balance = 0;

        for token in &tokens {
            let metadata = token.metadata.as_ref().unwrap();
            let lock_info = self.unwrap_metadata(&metadata);
            ret.push(lock_info.clone());
            token_ids.push(token.token_id.clone());
            locked += lock_info.locked_token_amount.0;
            voting_power += lock_info.voting_power.0;
        }
        
        (token_ids, ret, deposited.into(), locked.into(), voting_power.into())
    }

    pub fn get_voting_power_for_account(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> U128 {
        let mut ret = 0u128;
        let tokens = self.tokens.nft_tokens_for_owner(account_id.clone(), from_index, limit);

        for token in &tokens {
            let metadata = token.metadata.as_ref().unwrap();
            let lock_info = self.unwrap_metadata(&metadata);
            ret = ret + lock_info.voting_power.0;
        }
        U128(ret)
    }

    pub fn voting_power_unlock_time(&self, value: U128, unlock_time: u64) -> U128 {
        let now = env::block_timestamp_ms() / 1000;
        if unlock_time <= now {
            return U128(0);
        }
        let locked_seconds = unlock_time - now;
        if locked_seconds >= MAXTIME {
            return value;
        }
        let value_u128: u128 = value.into();
        U128(
            value_u128 * u128::try_from(locked_seconds).unwrap() / u128::try_from(MAXTIME).unwrap(),
        )
    }

    pub fn voting_power_locked_days(&self, value: U128, days: u64) -> U128 {
        if days > MAXDAYS {
            return value;
        }
        let value_u128: u128 = value.into();
        U128(value_u128 * u128::try_from(days).unwrap() / u128::try_from(MAXDAYS).unwrap())
    }
}

pub fn current_time_sec() -> u64 {
    env::block_timestamp_ms() / 1000
}
