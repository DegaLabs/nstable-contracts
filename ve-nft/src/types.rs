use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::U128;
use near_sdk:: {env, PanicOnDefault, AccountId, Balance};

/// Account deposits information and storage cost.
#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize, PanicOnDefault)]
#[serde(crate = "near_sdk::serde")]
pub struct LockInfo {
    pub creator: AccountId,
    pub create_time_sec: u64,
    pub locked_token_amount: U128,
    pub locked_till: u64,
    pub voting_power: U128
}

impl LockInfo {
    pub fn new(creator: AccountId, locked_amount: Balance, locked_till: u64, voting_power: Balance) -> LockInfo {
        LockInfo { creator: creator, create_time_sec: env::block_timestamp_ms() / 1000,  locked_token_amount: U128(locked_amount), locked_till: locked_till, voting_power: U128(voting_power)}
    }

    pub fn new_default(creator: AccountId) -> LockInfo {
        LockInfo::new(creator, 0, 0, 0)
    }
}