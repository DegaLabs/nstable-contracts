//! Wrapper of different types of stakepools 

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, Balance};

use crate::simple_stakepool::{SimpleStakePool, RPS};
use crate::LockTokenId;

pub(crate) type StakePoolId = String;

/// Generic StakePool, providing wrapper around different implementations of stakepools.
/// Allows to add new types of stakepools just by adding extra item in the enum 
/// without needing to migrate the storage.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum StakePool {
    SimpleStakePool(SimpleStakePool),
}

impl StakePool {
    /// Returns stakepool kind.
    pub fn kind(&self) -> String {
        match self {
            StakePool::SimpleStakePool(_) => "SIMPLE_STAKEPOOL".to_string(),
        }
    }

    /// return None if the stakepool can not accept reward anymore
    /// else return amount of undistributed reward 
    pub fn add_reward(&mut self, amount: &Balance) -> Option<Balance> {
        match self {
            StakePool::SimpleStakePool(stakepool) => stakepool.add_reward(amount),
        }
    }

    /// Returns locktoken id this stakepool accepted.
    pub fn get_locktoken_id(&self) -> LockTokenId {
        match self {
            StakePool::SimpleStakePool(stakepool) => stakepool.terms.locktoken_id.clone(),
        }
    }

    /// Returns token contract id this stakepool used for reward.
    pub fn get_reward_token(&self) -> AccountId {
        match self {
            StakePool::SimpleStakePool(stakepool) => stakepool.terms.reward_token.clone(),
        }
    }

    pub fn get_stakepool_id(&self) -> StakePoolId {
        match self {
            StakePool::SimpleStakePool(stakepool) => stakepool.stakepool_id.clone(),
        }
    }

    /// Returns how many reward tokens can given staker claim.
    pub fn view_staker_unclaimed_reward(
        &self,
        user_rps: &RPS,
        user_locktokens: &Balance,
        total_locktokens: &Balance,
    ) -> Balance {
        match self {
            StakePool::SimpleStakePool(stakepool) 
                => stakepool.view_staker_unclaimed_reward(user_rps, user_locktokens, total_locktokens),
        }
    }

    /// return the new user reward per locktoken 
    /// and amount of reward as (user_rps, reward_amount) 
    pub fn claim_user_reward(&mut self, 
        user_rps: &RPS,
        user_locktokens: &Balance, 
        total_locktokens: &Balance, 
        silent: bool,
    ) -> (RPS, Balance) {
        match self {
            StakePool::SimpleStakePool(stakepool) 
                => stakepool.claim_user_reward(user_rps, user_locktokens, total_locktokens, silent),
        }
    }

    pub fn can_be_removed(&self, expire_sec: u32) -> bool {
        match self {
            StakePool::SimpleStakePool(stakepool) => stakepool.can_be_removed(expire_sec),
        }
    }

    pub fn can_be_cancelled(&self) -> bool {
        match self {
            StakePool::SimpleStakePool(stakepool) => stakepool.can_be_cancelled(),
        }
    }

    pub fn move_to_clear(&mut self) {
        match self {
            StakePool::SimpleStakePool(stakepool) => stakepool.move_to_clear(),
        }
    }

}
