//!   The SimpleStakePool provide a way to gain stakepooling rewards periodically and 
//! proportionally.
//!   The creator first wrap his reward distribution schema with 
//! `SimpleStakePoolRewardTerms`, and create the stakepool with it, attached enough near 
//! for storage fee.
//!   But to enable stakepooling, the creator or someone else should deposit reward 
//! token to the stakepool, after it was created.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U128, ValidAccountId};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, AccountId, Balance};

use crate::{LockTokenId, StakePoolId};
use crate::errors::*;
use crate::utils::*;
use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

construct_uint! {
	pub struct U512(8);
}

pub type RPS = [u8; 32];

// to ensure precision, all reward_per_locktoken would be multiplied by this DENOM
// this value should be carefully choosen, now is 10**24.
pub const DENOM: u128 = 1_000_000_000_000_000_000_000_000;

///   The terms defines how the stakepool works.
///   In this version, we distribute reward token with a start height, a reward 
/// session interval, and reward amount per session.  
///   In this way, the stakepool will take the amount from undistributed reward to  
/// unclaimed reward each session. And all stakers would got reward token pro  
/// rata of their locktokens.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SimpleStakePoolTerms {
    pub locktoken_id: LockTokenId,
    pub reward_token: AccountId,
    pub start_at: TimestampSec,
    pub reward_per_session: Balance,
    pub session_interval: TimestampSec,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct HRSimpleStakePoolTerms {
    pub locktoken_id: LockTokenId,
    pub reward_token: ValidAccountId,
    pub start_at: u32,
    pub reward_per_session: U128,
    pub session_interval: u32, 
}

impl From<&HRSimpleStakePoolTerms> for SimpleStakePoolTerms {
    fn from(terms: &HRSimpleStakePoolTerms) -> Self {
        SimpleStakePoolTerms {
            locktoken_id: terms.locktoken_id.clone(),
            reward_token: terms.reward_token.clone().into(),
            start_at: terms.start_at,
            reward_per_session: terms.reward_per_session.into(),
            session_interval: terms.session_interval,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum SimpleStakePoolStatus {
    Created, Running, Ended, Cleared
}

impl From<&SimpleStakePoolStatus> for String {
    fn from(status: &SimpleStakePoolStatus) -> Self {
        match *status {
            SimpleStakePoolStatus::Created => { String::from("Created") },
            SimpleStakePoolStatus::Running => { String::from("Running") },
            SimpleStakePoolStatus::Ended => { String::from("Ended") },
            SimpleStakePoolStatus::Cleared => { String::from("Cleared") },
        }
    }
}

/// Reward Distribution Record
#[derive(BorshSerialize, BorshDeserialize, Clone, Default)]
pub struct SimpleStakePoolRewardDistribution {
    /// unreleased reward
    pub undistributed: Balance,
    /// the total rewards distributed but not yet claimed by stakers.
    pub unclaimed: Balance,
    /// Reward_Per_LockToken
    /// rps(cur) = rps(prev) + distributing_reward / total_locktoken_staked
    pub rps: RPS,
    /// Reward_Round
    /// rr = (cur_block_timestamp in sec - start_at) / session_interval
    pub rr: u32,
}

///   Implementation of simple stakepool, Similar to the design of "berry stakepool".
///   Staker stake their locktoken to stakepooling on multiple stakepool accept that locktoken.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct SimpleStakePool {

    pub stakepool_id: StakePoolId,
    
    pub terms: SimpleStakePoolTerms,

    pub status: SimpleStakePoolStatus,

    pub last_distribution: SimpleStakePoolRewardDistribution,

    /// total reward send into this stakepool by far, 
    /// every time reward deposited in, add to this field
    pub amount_of_reward: Balance,
    /// reward token has been claimed by staker by far
    pub amount_of_claimed: Balance,
    /// when there is no locktoken token staked, reward goes to beneficiary
    pub amount_of_beneficiary: Balance,

}

impl SimpleStakePool {
    pub(crate) fn new(
        id: StakePoolId,
        terms: SimpleStakePoolTerms,
    ) -> Self {
        Self {
            stakepool_id: id.clone(),
            amount_of_reward: 0,
            amount_of_claimed: 0,
            amount_of_beneficiary: 0,

            status: SimpleStakePoolStatus::Created,
            last_distribution: SimpleStakePoolRewardDistribution::default(),
            terms,
        }
    }

    /// return None if the stakepool can not accept reward anymore
    /// else return amount of undistributed reward 
    pub(crate) fn add_reward(&mut self, amount: &Balance) -> Option<Balance> {

        match self.status {
            SimpleStakePoolStatus::Created => {
                // When a stakepool gots first deposit of reward, it turns to Running state,
                // but stakepooling or not depends on `start_at` 
                self.status = SimpleStakePoolStatus::Running;
                if self.terms.start_at == 0 {
                    // for a stakepool without start time, the first deposit of reward 
                    // would trigger the stakepooling
                    self.terms.start_at = to_sec(env::block_timestamp());
                }
                self.amount_of_reward += amount;
                self.last_distribution.undistributed += amount;
                Some(self.last_distribution.undistributed)
            },
            SimpleStakePoolStatus::Running => {
                if let Some(dis) = self.try_distribute(&DENOM) {
                    if dis.undistributed == 0 {
                        // stakepool has ended actually
                        return None;
                    }
                }
                // For a running stakepool, can add reward to extend duration
                self.amount_of_reward += amount;
                self.last_distribution.undistributed += amount;
                Some(self.last_distribution.undistributed)
            },
            _ => {None},
        }
        
    }


    /// Try to distribute reward according to current timestamp
    /// return None if stakepool is not in Running state or haven't start stakepooling yet;
    /// return new dis :SimpleStakePoolRewardDistribution 
    /// Note, if total_locktoken is 0, the rps in new dis would be reset to 0 too.
    pub(crate) fn try_distribute(&self, total_locktokens: &Balance) -> Option<SimpleStakePoolRewardDistribution> {

        if let SimpleStakePoolStatus::Running = self.status {
            if env::block_timestamp() < to_nano(self.terms.start_at) {
                // a stakepool haven't start yet
                return None;
            }
            let mut dis = self.last_distribution.clone();
            // calculate rr according to cur_timestamp
            dis.rr = (to_sec(env::block_timestamp()) - self.terms.start_at) / self.terms.session_interval;
            let mut reward_added = (dis.rr - self.last_distribution.rr) as u128 
                * self.terms.reward_per_session;
            if self.last_distribution.undistributed < reward_added {
                // all undistribution would be distributed this time
                reward_added = self.last_distribution.undistributed;
                // recalculate rr according to undistributed
                let increased_rr = (reward_added / self.terms.reward_per_session) as u32;
                dis.rr = self.last_distribution.rr + increased_rr;
                let reward_caculated = increased_rr as u128 * self.terms.reward_per_session;
                if reward_caculated < reward_added {
                    // add the tail round
                    dis.rr += 1;

                }
                // env::log(
                //     format!(
                //         "StakePool ends at Round #{}, unclaimed reward: {}.",
                //         dis.rr, reward_added + dis.unclaimed
                //     )
                //     .as_bytes(),
                // );
            }
            dis.unclaimed += reward_added;
            dis.undistributed -= reward_added;

            // calculate rps
            if total_locktokens == &0 {
                U256::from(0).to_little_endian(&mut dis.rps);
            } else {
                (
                    U256::from_little_endian(&self.last_distribution.rps) + 
                    U256::from(reward_added) 
                    * U256::from(DENOM) 
                    / U256::from(*total_locktokens)
                ).to_little_endian(&mut dis.rps);
            }
            Some(dis)
        } else {
            None
        }

    }

    /// Return how many reward token that the user hasn't claimed yet.
    /// return (cur_rps - last_user_rps) * user_locktokens / DENOM
    pub(crate) fn view_staker_unclaimed_reward(
        &self,
        user_rps: &RPS,
        user_locktokens: &Balance,
        total_locktokens: &Balance,
    ) -> Balance {
        if total_locktokens == &0 {
            return 0;
        }
        if user_locktokens == &0 {
            return 0;
        }
        if let Some(dis) = self.try_distribute(total_locktokens) {
            (U256::from(*user_locktokens) 
            * (U256::from_little_endian(&dis.rps) - U256::from_little_endian(user_rps))
            / U256::from(DENOM)).as_u128()
        } else {
            (U256::from(*user_locktokens) 
            * (U256::from_little_endian(&self.last_distribution.rps) - U256::from_little_endian(user_rps))
            / U256::from(DENOM)).as_u128()
        }
    }

    /// Distribute reward generated from previous distribution to now,
    /// only works for stakepool in Running state and has reward deposited in,
    /// Note 1, if undistribute equals 0, the stakepool goes to Ended state;
    /// Note 2, if total_locktoken is 0, reward is claimed directly by beneficiary
    pub(crate) fn distribute(&mut self, total_locktokens: &Balance, silent: bool) {
        if let Some(dis) = self.try_distribute(total_locktokens) {
            if self.last_distribution.rr != dis.rr {
                self.last_distribution = dis.clone();
                if total_locktokens == &0 {
                    // if total_locktokens == &0, reward goes to beneficiary,
                    self.amount_of_claimed += self.last_distribution.unclaimed;
                    self.amount_of_beneficiary += self.last_distribution.unclaimed;
                    self.last_distribution.unclaimed = 0;
                }   
                if !silent {
                    env::log(
                        format!(
                            "{} RPS increased to {} and RR update to #{}",
                            self.stakepool_id, U256::from_little_endian(&dis.rps), dis.rr,
                        )
                        .as_bytes(),
                    );
                }
                
            }
            if self.last_distribution.undistributed == 0 {
                self.status = SimpleStakePoolStatus::Ended;
            }
        } 
    }

    /// Claim user's unclaimed reward in this stakepool,
    /// return the new user RPS (reward per locktoken),  
    /// and amount of reward 
    pub(crate) fn claim_user_reward(
        &mut self, 
        user_rps: &RPS,
        user_locktokens: &Balance, 
        total_locktokens: &Balance, 
        silent: bool,
    ) -> (RPS, Balance) {

        self.distribute(total_locktokens, silent);
        // if user_locktokens == &0 {
        //     return (self.last_distribution.rps, 0);
        // }

        let claimed = (
            U512::from(*user_locktokens) 
            * (U512::from_little_endian(&self.last_distribution.rps) - U512::from_little_endian(user_rps))
            / U512::from(DENOM)
        ).as_u128();

        if claimed > 0 {
            assert!(
                self.last_distribution.unclaimed >= claimed, 
                "{} unclaimed:{}, cur_claim:{}", 
                ERR500, self.last_distribution.unclaimed, claimed
            );
            self.last_distribution.unclaimed -= claimed;
            self.amount_of_claimed += claimed;
        }

        (self.last_distribution.rps, claimed)
    }

    /// Move an Ended stakepool to Cleared, if any unclaimed reward exists, go to beneficiary
    pub(crate) fn move_to_clear(&mut self) {
        self.last_distribution.unclaimed += self.last_distribution.undistributed;
        if self.last_distribution.unclaimed > 0 {
            self.amount_of_claimed += self.last_distribution.unclaimed;
            self.amount_of_beneficiary += self.last_distribution.unclaimed;
            self.last_distribution.unclaimed = 0;
        }
        self.status = SimpleStakePoolStatus::Cleared;
    }

    /// stakepool_expire_sec after end_time
    pub fn can_be_removed(&self, expire_sec: u32) -> bool {
        let mut ret = false;
        if self.amount_of_reward > 0 {
            let duration = self.terms.session_interval * (1 + self.amount_of_reward / self.terms.reward_per_session) as u32;
            let end_secs = self.terms.start_at + duration;
            if to_sec(env::block_timestamp()) > end_secs + expire_sec {
                ret = true;
            }
        }
        return ret;
    }

    pub fn can_be_cancelled(&self) -> bool {
        self.amount_of_reward == 0 
    }

}

