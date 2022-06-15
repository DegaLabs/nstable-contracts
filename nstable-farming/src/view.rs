//! View functions for the contract.

use std::collections::HashMap;

use near_sdk::json_types::{ValidAccountId, U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, AccountId};

use crate::stakepool_locktoken::LockTokenInfo;
use crate::staker::CDAccount;
use crate::utils::parse_stakepool_id;
use crate::simple_stakepool::DENOM;
use crate::*;

use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Metadata {
    pub version: String,
    pub owner_id: AccountId,
    pub operators: Vec<AccountId>,
    pub staker_count: U64,
    pub stakepool_count: U64,
    pub locktoken_count: U64,
    pub reward_count: U64,
    pub stakepool_expire_sec: u32,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageReport {
    pub storage: U64,
    pub locking_near: U128,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StakePoolInfo {
    pub stakepool_id: StakePoolId,
    pub stakepool_kind: String,
    pub stakepool_status: String,
    pub locktoken_id: LockTokenId,
    pub reward_token: AccountId,
    pub start_at: u32,
    pub reward_per_session: U128,
    pub session_interval: u32,

    pub total_reward: U128,
    pub cur_round: u32,
    pub last_round: u32,
    pub claimed_reward: U128,
    pub unclaimed_reward: U128,
    pub beneficiary_reward: U128,
}

impl From<&StakePool> for StakePoolInfo {
    fn from(stakepool: &StakePool) -> Self {
        let stakepool_kind = stakepool.kind();
        match stakepool {
            StakePool::SimpleStakePool(stakepool) => {
                if let Some(dis) = stakepool.try_distribute(&DENOM) {
                    let mut stakepool_status: String = (&stakepool.status).into();
                    if stakepool_status == "Running".to_string()
                        && dis.undistributed == 0
                    {
                        stakepool_status = "Ended".to_string();
                    }
                    Self {
                        stakepool_id: stakepool.stakepool_id.clone(),
                        stakepool_kind,
                        stakepool_status,
                        locktoken_id: stakepool.terms.locktoken_id.clone(),
                        reward_token: stakepool.terms.reward_token.clone(),
                        start_at: stakepool.terms.start_at,
                        reward_per_session: stakepool.terms.reward_per_session.into(),
                        session_interval: stakepool.terms.session_interval,

                        total_reward: stakepool.amount_of_reward.into(),
                        cur_round: dis.rr.into(),
                        last_round: stakepool.last_distribution.rr.into(),
                        claimed_reward: stakepool.amount_of_claimed.into(),
                        unclaimed_reward: dis.unclaimed.into(),
                        beneficiary_reward: stakepool.amount_of_beneficiary.into(),
                    }
                } else {
                    Self {
                        stakepool_id: stakepool.stakepool_id.clone(),
                        stakepool_kind,
                        stakepool_status: (&stakepool.status).into(),
                        locktoken_id: stakepool.terms.locktoken_id.clone(),
                        reward_token: stakepool.terms.reward_token.clone(),
                        start_at: stakepool.terms.start_at.into(),
                        reward_per_session: stakepool.terms.reward_per_session.into(),
                        session_interval: stakepool.terms.session_interval.into(),
    
                        total_reward: stakepool.amount_of_reward.into(),
                        cur_round: stakepool.last_distribution.rr.into(),
                        last_round: stakepool.last_distribution.rr.into(),
                        claimed_reward: stakepool.amount_of_claimed.into(),
                        // unclaimed_reward: (stakepool.amount_of_reward - stakepool.amount_of_claimed).into(),
                        unclaimed_reward: stakepool.last_distribution.unclaimed.into(),
                        beneficiary_reward: stakepool.amount_of_beneficiary.into(),
                    }
                }                
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct UserLockTokenInfo {
    pub locktoken_id: LockTokenId,
    pub amount: U128,
    pub power: U128,
    pub cds: Vec<CDAccountInfo>
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct CDAccountInfo {
    pub cd_account_id: u32,
    pub locktoken_id: LockTokenId,
    pub locktoken_amount: U128,
    pub locktoken_power: U128,
    pub begin_sec: u32,
    pub end_sec: u32
}

impl From<CDAccount> for CDAccountInfo {
    fn from(cd_account: CDAccount) -> Self {
        CDAccountInfo{
            cd_account_id: 0,
            locktoken_id: cd_account.locktoken_id.clone(),
            locktoken_amount: cd_account.locktoken_amount.into(),
            locktoken_power: cd_account.locktoken_power.into(),
            begin_sec: cd_account.begin_sec,
            end_sec: cd_account.end_sec,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct CDStakeItemInfo{
    pub enable: bool,
    pub lock_sec: u32,
    pub power_reward_rate: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct CDStrategyInfo {
    pub stake_strategy: Vec<CDStakeItemInfo>,
    pub locktoken_slash_rate: u32,
}

impl From<&CDStrategy> for CDStrategyInfo {
    fn from(cd_strategy: &CDStrategy) -> Self {
        CDStrategyInfo{
            stake_strategy: cd_strategy.stake_strategy.iter().map(|item|
                CDStakeItemInfo {
                    lock_sec: item.lock_sec,
                    power_reward_rate: item.power_reward_rate,
                    enable: item.enable,
                }).collect(),
            locktoken_slash_rate: cd_strategy.locktoken_slash_rate,
        }
    }
}

impl Contract {
    fn user_locktoken_info(&self, staker: &VersionedStaker, locktoken_id: &LockTokenId) -> UserLockTokenInfo {
        UserLockTokenInfo{
            locktoken_id: locktoken_id.clone(),
            amount: staker.get_ref().locktoken_amounts.get(locktoken_id).map_or(U128(0), |&v| {
                let mut cd_amount_total = 0;
                for f in staker.get_ref().cd_accounts.iter(){
                    cd_amount_total += f.locktoken_amount;
                }
                U128(v + cd_amount_total)
            }),
            power: staker.get_ref().locktoken_powers.get(locktoken_id).map_or(U128(0), |&v| U128(v)),
            cds: staker.get_ref().cd_accounts.iter().enumerate().map(|(index, cd_account)| {
                let mut cd_account_info: CDAccountInfo = cd_account.into();
                cd_account_info.cd_account_id = index as u32;
                cd_account_info
            }).collect()
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_metadata(&self) -> Metadata {
        Metadata {
            owner_id: self.data().owner_id.clone(),
            operators: self.data().operators.to_vec(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            staker_count: self.data().staker_count.into(),
            stakepool_count: self.data().stakepools.len().into(),
            locktoken_count: self.data().locktokens.len().into(),
            reward_count: self.data().reward_info.len().into(),
            stakepool_expire_sec: self.data().stakepool_expire_sec,
        }
    }

    pub fn get_contract_storage_report(&self) -> StorageReport {
        let su = env::storage_usage();
        StorageReport {
            storage: U64(su),
            locking_near: U128(su as Balance * env::storage_byte_cost()),
        }
    }

    /// Returns number of stakepools.
    pub fn get_number_of_stakepools(&self) -> u64 {
        self.data().stakepools.len()
    }

    pub fn get_number_of_outdated_stakepools(&self) -> u64 {
        self.data().outdated_stakepools.len()
    }

    /// Returns list of stakepools of given length from given start index.
    pub fn list_stakepools(&self, from_index: u64, limit: u64) -> Vec<StakePoolInfo> {
        let keys = self.data().stakepools.keys_as_vector();

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| 
                (&self.data().stakepools.get(&keys.get(index).unwrap()).unwrap()).into()
            )
            .collect()
    }

    pub fn list_outdated_stakepools(&self, from_index: u64, limit: u64) -> Vec<StakePoolInfo> {
        let keys = self.data().outdated_stakepools.keys_as_vector();

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| 
                (&self.data().outdated_stakepools.get(&keys.get(index).unwrap()).unwrap()).into()
            )
            .collect()
    }

    pub fn list_stakepools_by_locktoken(&self, locktoken_id: LockTokenId) -> Vec<StakePoolInfo> {
        self.get_locktoken(&locktoken_id)
            .get_ref()
            .stakepools
            .iter()
            .map(|stakepool_id| 
                (&self.data().stakepools.get(&stakepool_id).unwrap()).into()
            )
            .collect()
    }

    /// Returns information about specified stakepool.
    pub fn get_stakepool(&self, stakepool_id: StakePoolId) -> Option<StakePoolInfo> {
        if let Some(stakepool) = self.data().stakepools.get(&stakepool_id) {
            Some((&stakepool).into())
        } else {
            None
        }
    }

    pub fn get_outdated_stakepool(&self, stakepool_id: StakePoolId) -> Option<StakePoolInfo> {
        if let Some(stakepool) = self.data().outdated_stakepools.get(&stakepool_id) {
            Some((&stakepool).into())
        } else {
            None
        }
    }

    pub fn list_rewards_info(&self, from_index: u64, limit: u64) -> HashMap<AccountId, U128> {
        let keys = self.data().reward_info.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.data()
                        .reward_info
                        .get(&keys.get(index).unwrap())
                        .unwrap_or(0)
                        .into(),
                )
            })
            .collect()
    }

    /// Returns reward token claimed for given user outside of any stakepools.
    /// Returns empty list if no rewards claimed.
    pub fn list_rewards(&self, account_id: ValidAccountId) -> HashMap<AccountId, U128> {
        self.get_staker_default(account_id.as_ref())
            .get()
            .rewards
            .into_iter()
            .map(|(acc, bal)| (acc, U128(bal)))
            .collect()
    }

    /// Returns balance of amount of given reward token that ready to withdraw.
    pub fn get_reward(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128 {
        self.internal_get_reward(account_id.as_ref(), token_id.as_ref())
            .into()
    }

    pub fn get_unclaimed_reward(&self, account_id: ValidAccountId, stakepool_id: StakePoolId) -> U128 {
        let (locktoken_id, _) = parse_stakepool_id(&stakepool_id);

        if let (Some(staker), Some(stakepool_locktoken)) = (
            self.get_staker_wrapped(account_id.as_ref()),
            self.get_locktoken_wrapped(&locktoken_id),
        ) {
            if let Some(stakepool) = self.data().stakepools.get(&stakepool_id) {
                let reward_amount = stakepool.view_staker_unclaimed_reward(
                    &staker.get_ref().get_rps(&stakepool.get_stakepool_id()),
                    staker.get_ref().locktoken_powers.get(&locktoken_id).unwrap_or(&0_u128),
                    &stakepool_locktoken.get_ref().total_locktoken_power,
                );
                reward_amount.into()
            } else {
                0.into()
            }
        } else {
            0.into()
        }
    }

    /// return all locktoken and its amount staked in this contract in a hashmap
    pub fn list_locktokens(&self, from_index: u64, limit: u64) -> HashMap<LockTokenId, U128> {
        let keys = self.data().locktokens.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.get_locktoken(&keys.get(index).unwrap())
                        .get_ref()
                        .total_locktoken_amount
                        .into(),
                )
            })
            .collect()
    }

    /// return user staked locktokens and its amount in a hashmap
    pub fn list_user_locktoken_amounts(&self, account_id: ValidAccountId) -> HashMap<LockTokenId, U128> {
        if let Some(staker) = self.get_staker_wrapped(account_id.as_ref()) {
            staker
                .get()
                .locktoken_amounts
                .into_iter()
                .map(|(locktoken, bal)| (locktoken.clone(), U128(bal)))
                .collect()
        } else {
            HashMap::new()
        }
    }

    pub fn list_user_locktoken_powers(&self, account_id: ValidAccountId) -> HashMap<LockTokenId, U128> {
        if let Some(staker) = self.get_staker_wrapped(account_id.as_ref()) {
            staker
                .get()
                .locktoken_powers
                .into_iter()
                .map(|(locktoken, bal)| (locktoken.clone(), U128(bal)))
                .collect()
        } else {
            HashMap::new()
        }
    }

    pub fn get_locktoken_info(&self, locktoken_id: LockTokenId) -> Option<LockTokenInfo> {
        if let Some(stakepool_locktoken) = self.get_locktoken_wrapped(&locktoken_id) {
            Some(stakepool_locktoken.get_ref().into())
        } else {
            None
        }
    }

    pub fn list_locktokens_info(&self, from_index: u64, limit: u64) -> HashMap<LockTokenId, LockTokenInfo> {
        let keys = self.data().locktokens.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.get_locktoken(&keys.get(index).unwrap()).get_ref().into(),
                )
            })
            .collect()
    }

    pub fn get_user_rps(&self, account_id: ValidAccountId, stakepool_id: StakePoolId) -> Option<String> {
        let staker = self.get_staker(account_id.as_ref());
        if let Some(rps) = staker.get().user_rps.get(&stakepool_id) {
            Some(format!("{}", U256::from_little_endian(&rps)))
        } else {
            None
        }
    }

    pub fn get_user_locktoken_info(&self, account_id: ValidAccountId, locktoken_id: LockTokenId) -> Option<UserLockTokenInfo> {
        if let Some(staker) = self.get_staker_wrapped(account_id.as_ref()){
            Some(self.user_locktoken_info(&staker, &locktoken_id))
        }else{
            None
        }
    }

    pub fn list_user_locktoken_info(&self, account_id: ValidAccountId, from_index: u64, limit: u64) -> HashMap<LockTokenId, UserLockTokenInfo> {
        if let Some(staker) = self.get_staker_wrapped(account_id.as_ref()){
            let keys = self.data().locktokens.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                let locktoken_id = keys.get(index).unwrap();
                (
                    locktoken_id.clone(),
                    self.user_locktoken_info(&staker, &locktoken_id),
                )
            })
            .collect()
        }else{
            HashMap::new()
        }
    }

    pub fn list_user_cd_account(&self, account_id: ValidAccountId, from_index: u64, limit: u64) -> Vec<CDAccountInfo> {
        let staker = self.get_staker(&account_id.into());

        (from_index..std::cmp::min(from_index + limit, staker.get_ref().cd_accounts.len()))
            .map(|index| {
                    let mut cd_account_info: CDAccountInfo = staker.get_ref().cd_accounts.get(index).unwrap().into();
                    cd_account_info.cd_account_id = index as u32;
                    cd_account_info
                }
            )
            .collect()
    }

    pub fn get_cd_strategy(&self) -> CDStrategyInfo {
        (&self.data().cd_strategy).into()
    }

    /// return slashed locktoken and its amount in this contract in a hashmap
    pub fn list_shashed(&self, from_index: u64, limit: u64) -> HashMap<LockTokenId, U128> {
        let keys = self.data().locktokens_slashed.keys_as_vector();

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.data().locktokens_slashed.get(&keys.get(index).unwrap()).unwrap().into()
                )
            })
            .collect()
    }

    /// return lostfound locktoken and its amount in this contract in a hashmap
    pub fn list_lostfound(&self, from_index: u64, limit: u64) -> HashMap<LockTokenId, U128> {
        let keys = self.data().locktokens_lostfound.keys_as_vector();

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.data().locktokens_lostfound.get(&keys.get(index).unwrap()).unwrap().into()
                )
            })
            .collect()
    }
}
