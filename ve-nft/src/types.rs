use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::U128;

/// Account deposits information and storage cost.
#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LockedBalance {
    /// Native NEAR amount sent to the exchange.
    /// Used for storage right now, but in future can be used for trading as well.
    pub amount: U128,
    pub end: u64,
    pub minted_for_lock: U128
}

impl Default for LockedBalance {
    fn default() -> LockedBalance {
        LockedBalance { amount: U128(0), end: 0, minted_for_lock: U128(0) }
    }
}