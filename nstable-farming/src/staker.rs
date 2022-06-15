//! Staker records a staker's 
//! * all claimed reward tokens, 
//! * all locktokens he staked,
//! * all cd account he add,
//! * user_rps per stakepool,

use std::collections::HashMap;
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, AccountId, Balance};
use near_sdk::serde::{Deserialize, Serialize};
use crate::{LockTokenId, StakePoolId, RPS};
use crate::*;
use crate::errors::*;
use crate::utils::*;
use crate::StorageKeys;

/// If locktoken_amount == 0, this CDAccount is empty and can be occupied.
/// When add/remove locktoken to/from a non-empty CDAccount, 
/// the delta power is calculate based on delta locktoken, current timestamp, begin_sec and end_sec.
/// When remove locktoken before end_sec, a slash on locktoken amount would happen, 
/// based on remove amount, locktoken_slash_rate, current timestamp, begin_sec and end_sec.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct CDAccount {
    pub locktoken_id: LockTokenId,
    /// actual locktoken balance user staked
    pub locktoken_amount: Balance,
    /// implied power_reward_rate when occupied
    pub original_power_reward_rate: u32,
    /// shares used in reward distribution
    pub locktoken_power: Balance,
    /// the begin timestamp
    pub begin_sec: TimestampSec,
    /// promise not unstake before this timestamp
    pub end_sec: TimestampSec,
}

impl Default for CDAccount {
    fn default() -> CDAccount {
        CDAccount {
            locktoken_id: "".to_string(),
            locktoken_amount: 0,
            original_power_reward_rate: 0,
            locktoken_power: 0,
            begin_sec: 0,
            end_sec: 0,
        }
    }
}

impl CDAccount {
    /// return power
    pub(crate) fn occupy(&mut self, locktoken_id: &LockTokenId, amount: Balance, power_reward_rate: u32, lasts_sec: u32) -> Balance {
        assert_eq!(self.locktoken_amount, 0, "{}", ERR65_NON_EMPTY_CD_ACCOUNT);
        assert!(lasts_sec > 0, "{}", ERR68_INVALID_CD_STRATEGY);

        self.locktoken_id = locktoken_id.clone();
        self.locktoken_amount = amount;
        self.original_power_reward_rate = power_reward_rate;
        self.locktoken_power = amount + (U256::from(amount) * U256::from(power_reward_rate) / U256::from(DENOM)).as_u128();
        self.begin_sec = to_sec(env::block_timestamp());
        self.end_sec = self.begin_sec + lasts_sec;
        self.locktoken_power
    }

    /// return power added
    pub(crate) fn append(&mut self, locktoken_id: &LockTokenId, amount: Balance) -> Balance {
        assert!(self.locktoken_amount > 0, "{}", ERR66_EMPTY_CD_ACCOUNT);
        assert_eq!(self.locktoken_id, locktoken_id.clone(), "{}", ERR67_UNMATCHED_LOCKTOKEN_ID);

        self.locktoken_amount += amount;

        let now = to_sec(env::block_timestamp());
        let power_reward = if now < self.end_sec && now > self.begin_sec {
            let full_reward = U256::from(amount) * U256::from(self.original_power_reward_rate) / U256::from(DENOM);
            (full_reward * U256::from(self.end_sec - now) / U256::from(self.end_sec - self.begin_sec)).as_u128()
        } else {
            0
        };
        self.locktoken_power += amount + power_reward;

        amount + power_reward
    }

    /// return power removed and locktoken slashed
    pub(crate) fn remove(&mut self, locktoken_id: &LockTokenId, amount: Balance, slash_rate: u32) -> (Balance, Balance) {
        assert!(self.locktoken_amount > 0, "{}", ERR66_EMPTY_CD_ACCOUNT);
        assert_eq!(self.locktoken_id, locktoken_id.clone(), "{}", ERR67_UNMATCHED_LOCKTOKEN_ID);
        assert!(self.locktoken_amount >= amount, "{}", ERR32_NOT_ENOUGH_LOCKTOKEN);

        let now = to_sec(env::block_timestamp());
        let locktoken_slashed = if now < self.end_sec && now >= self.begin_sec {
            let full_slashed = U256::from(amount) * U256::from(slash_rate) / U256::from(DENOM);
            (full_slashed * U256::from(self.end_sec - now) / U256::from(self.end_sec - self.begin_sec)).as_u128()
        } else {
            0
        };
        
        let power_removed = (U256::from(self.locktoken_power) * U256::from(amount) / U256::from(self.locktoken_amount)).as_u128();

        self.locktoken_amount -= amount;
        self.locktoken_power -= power_removed;

        (power_removed, locktoken_slashed)
    }
}


/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct Staker {
    /// Amounts of various reward tokens the staker claimed.
    pub rewards: HashMap<AccountId, Balance>,
    /// Amounts of various locktoken tokens the staker staked.
    pub locktoken_amounts: HashMap<LockTokenId, Balance>,
    /// Powers of various locktoken tokens the staker staked.
    pub locktoken_powers: HashMap<LockTokenId, Balance>,
    /// Record user_last_rps of stakepools
    pub user_rps: LookupMap<StakePoolId, RPS>,
    pub rps_count: u32,
    /// Staker can create up to 16 CD accounts
    pub cd_accounts: Vector<CDAccount>,
}

impl Staker {

    /// Adds amount to the balance of given token
    pub(crate) fn add_reward(&mut self, token: &AccountId, amount: Balance) {
        if let Some(x) = self.rewards.get_mut(token) {
            *x = *x + amount;
        } else {
            self.rewards.insert(token.clone(), amount);
        }
    }

    /// Subtract from `reward` balance.
    /// if amount == 0, subtract all reward balance.
    /// Panics if `amount` is bigger than the current balance.
    /// return actual subtract amount
    pub(crate) fn sub_reward(&mut self, token: &AccountId, amount: Balance) -> Balance {
        let value = *self.rewards.get(token).expect(ERR21_TOKEN_NOT_REG);
        assert!(value >= amount, "{}", ERR22_NOT_ENOUGH_TOKENS);
        if amount == 0 {
            self.rewards.remove(&token.clone());
            value
        } else {
            self.rewards.insert(token.clone(), value - amount);
            amount
        }
    }

    pub fn add_locktoken_amount(&mut self, locktoken_id: &LockTokenId, amount: Balance) {
        if amount > 0 {
            self.locktoken_amounts.insert(
                locktoken_id.clone(), 
                amount + self.locktoken_amounts.get(locktoken_id).unwrap_or(&0_u128)
            );
        }
        
    }

    /// return locktoken remained.
    pub fn sub_locktoken_amount(&mut self, locktoken_id: &LockTokenId, amount: Balance) -> Balance {
        let prev_balance = self.locktoken_amounts.get(locktoken_id).expect(&format!("{}", ERR31_LOCKTOKEN_NOT_EXIST));
        assert!(prev_balance >= &amount, "{}", ERR32_NOT_ENOUGH_LOCKTOKEN);
        let cur_balance = prev_balance - amount;
        if cur_balance > 0 {
            self.locktoken_amounts.insert(locktoken_id.clone(), cur_balance);
        } else {
            self.locktoken_amounts.remove(locktoken_id);
        }
        cur_balance
    }

    pub fn add_locktoken_power(&mut self, locktoken_id: &LockTokenId, amount: Balance) {
        if amount > 0 {
            self.locktoken_powers.insert(
                locktoken_id.clone(), 
                amount + self.locktoken_powers.get(locktoken_id).unwrap_or(&0_u128)
            );
        }
        
    }

    pub fn sub_locktoken_power(&mut self, locktoken_id: &LockTokenId, amount: Balance) -> Balance {
        let prev_balance = self.locktoken_powers.get(locktoken_id).expect(&format!("{}", ERR31_LOCKTOKEN_NOT_EXIST));
        assert!(prev_balance >= &amount, "{}", ERR32_NOT_ENOUGH_LOCKTOKEN);
        let cur_balance = prev_balance - amount;
        if cur_balance > 0 {
            self.locktoken_powers.insert(locktoken_id.clone(), cur_balance);
        } else {
            self.locktoken_powers.remove(locktoken_id);
        }
        cur_balance
    }

    pub fn get_rps(&self, stakepool_id: &StakePoolId) -> RPS {
        self.user_rps.get(stakepool_id).unwrap_or(RPS::default()).clone()
    }

    pub fn set_rps(&mut self, stakepool_id: &StakePoolId, rps: RPS) {
        if !self.user_rps.contains_key(stakepool_id) {
            self.rps_count += 1;
        } 
        self.user_rps.insert(stakepool_id, &rps);
    }

    pub fn remove_rps(&mut self, stakepool_id: &StakePoolId) {
        if self.user_rps.contains_key(stakepool_id) {
            self.user_rps.remove(stakepool_id);
            self.rps_count -= 1;
        }
    }
}


/// Versioned Staker, used for lazy upgrade.
/// Which means this structure would upgrade automatically when used.
/// To achieve that, each time the new version comes in, 
/// each function of this enum should be carefully re-code!
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedStaker {
    V101(Staker),
}

impl VersionedStaker {

    pub fn new(staker_id: AccountId) -> Self {
        VersionedStaker::V101(Staker {
            rewards: HashMap::new(),
            locktoken_amounts: HashMap::new(),
            locktoken_powers: HashMap::new(),
            user_rps: LookupMap::new(StorageKeys::UserRps {
                account_id: staker_id.clone(),
            }),
            rps_count: 0,
            cd_accounts: Vector::new(StorageKeys::CDAccount {
                account_id: staker_id.clone(),
            })
        })
    }

    /// Upgrades from other versions to the currently used version.
    pub fn upgrade(self) -> Self {
        match self {
            VersionedStaker::V101(staker) => VersionedStaker::V101(staker),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn need_upgrade(&self) -> bool {
        match self {
            VersionedStaker::V101(_) => false,
            _ => true,
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref(&self) -> &Staker {
        match self {
            VersionedStaker::V101(staker) => staker,
            _ => unimplemented!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get(self) -> Staker {
        match self {
            VersionedStaker::V101(staker) => staker,
            _ => unimplemented!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref_mut(&mut self) -> &mut Staker {
        match self {
            VersionedStaker::V101(staker) => staker,
            _ => unimplemented!(),
        }
    }
}
