use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk_sim::{view, ContractAccount};

use super::utils::to_va;
use nstable_stakepooling_v2::{ContractContract as StakePooling, StakePoolInfo, CDStrategyInfo, UserLockTokenInfo};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Metadata {
    pub version: String,
    pub owner_id: String,
    pub operators: Vec<String>,
    pub staker_count: U64,
    pub stakepool_count: U64,
    pub locktoken_count: U64,
    pub reward_count: U64,
    pub stakepool_expire_sec: u32,
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct LockTokenInfo {
    pub locktoken_id: String,
    pub locktoken_type: String,
    pub stakepools: Vec<String>,
    pub next_index: u32,
    pub amount: U128,
    pub power: U128,
    pub min_deposit: U128,
    pub slash_rate: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
}

#[allow(dead_code)]
pub fn get_metadata(stakepooling: &ContractAccount<StakePooling>) -> Metadata {
    view!(stakepooling.get_metadata()).unwrap_json::<Metadata>()
}

#[allow(dead_code)]
pub(crate) fn show_stakepools_by_locktoken(
    stakepooling: &ContractAccount<StakePooling>,
    locktoken_id: String,
    show_print: bool,
) -> Vec<StakePoolInfo> {
    let stakepools_info = view!(stakepooling.list_stakepools_by_locktoken(locktoken_id)).unwrap_json::<Vec<StakePoolInfo>>();
    if show_print {
        println!("StakePools Info has {} stakepools ===>", stakepools_info.len());
        for stakepool_info in stakepools_info.iter() {
            println!(
                "  ID:{}, Status:{}, LockToken:{}, Reward:{}",
                stakepool_info.stakepool_id, stakepool_info.stakepool_status, stakepool_info.locktoken_id, stakepool_info.reward_token
            );
            println!(
                "  StartAt:{}, SessionReward:{}, SessionInterval:{}",
                stakepool_info.start_at, stakepool_info.reward_per_session.0, stakepool_info.session_interval
            );
            println!(
                "  TotalReward:{}, Claimed:{}, Unclaimed:{}, LastRound:{}, CurRound:{}",
                stakepool_info.total_reward.0,
                stakepool_info.claimed_reward.0,
                stakepool_info.unclaimed_reward.0,
                stakepool_info.last_round,
                stakepool_info.cur_round
            );
        }
    }
    stakepools_info
}

#[allow(dead_code)]
pub(crate) fn show_stakepoolinfo(
    stakepooling: &ContractAccount<StakePooling>,
    stakepool_id: String,
    show_print: bool,
) -> StakePoolInfo {
    let stakepool_info = get_stakepoolinfo(stakepooling, stakepool_id);
    if show_print {
        println!("StakePool Info ===>");
        println!(
            "  ID:{}, Status:{}, LockToken:{}, Reward:{}",
            stakepool_info.stakepool_id, stakepool_info.stakepool_status, stakepool_info.locktoken_id, stakepool_info.reward_token
        );
        println!(
            "  StartAt:{}, SessionReward:{}, SessionInterval:{}",
            stakepool_info.start_at, stakepool_info.reward_per_session.0, stakepool_info.session_interval
        );
        println!(
            "  TotalReward:{}, Claimed:{}, Unclaimed:{}, LastRound:{}, CurRound:{}",
            stakepool_info.total_reward.0,
            stakepool_info.claimed_reward.0,
            stakepool_info.unclaimed_reward.0,
            stakepool_info.last_round,
            stakepool_info.cur_round
        );
    }
    stakepool_info
}

#[allow(dead_code)]
pub(crate) fn show_outdated_stakepools(
    stakepooling: &ContractAccount<StakePooling>,
    show_print: bool,
) -> Vec<StakePoolInfo> {
    let outdated_stakepools_info = view!(stakepooling.list_outdated_stakepools(0, 100)).unwrap_json::<Vec<StakePoolInfo>>();
    if show_print {
        println!("StakePools Info has {} stakepools ===>", outdated_stakepools_info.len());
        for stakepool_info in outdated_stakepools_info.iter() {
            println!(
                "  ID:{}, Status:{}, LockToken:{}, Reward:{}",
                stakepool_info.stakepool_id, stakepool_info.stakepool_status, stakepool_info.locktoken_id, stakepool_info.reward_token
            );
            println!(
                "  StartAt:{}, SessionReward:{}, SessionInterval:{}",
                stakepool_info.start_at, stakepool_info.reward_per_session.0, stakepool_info.session_interval
            );
            println!(
                "  TotalReward:{}, Claimed:{}, Unclaimed:{}, LastRound:{}, CurRound:{}",
                stakepool_info.total_reward.0,
                stakepool_info.claimed_reward.0,
                stakepool_info.unclaimed_reward.0,
                stakepool_info.last_round,
                stakepool_info.cur_round
            );
        }
    }
    outdated_stakepools_info
}

#[allow(dead_code)]
pub(crate) fn show_outdated_stakepoolinfo(
    stakepooling: &ContractAccount<StakePooling>,
    stakepool_id: String,
    show_print: bool,
) -> StakePoolInfo {
    let stakepool_info = get_outdated_stakepoolinfo(stakepooling, stakepool_id);
    if show_print {
        println!("StakePool Info ===>");
        println!(
            "  ID:{}, Status:{}, LockToken:{}, Reward:{}",
            stakepool_info.stakepool_id, stakepool_info.stakepool_status, stakepool_info.locktoken_id, stakepool_info.reward_token
        );
        println!(
            "  StartAt:{}, SessionReward:{}, SessionInterval:{}",
            stakepool_info.start_at, stakepool_info.reward_per_session.0, stakepool_info.session_interval
        );
        println!(
            "  TotalReward:{}, Claimed:{}, Unclaimed:{}, LastRound:{}, CurRound:{}",
            stakepool_info.total_reward.0,
            stakepool_info.claimed_reward.0,
            stakepool_info.unclaimed_reward.0,
            stakepool_info.last_round,
            stakepool_info.cur_round
        );
    }
    stakepool_info
}

#[allow(dead_code)]
pub(crate) fn show_locktokensinfo(
    stakepooling: &ContractAccount<StakePooling>,
    show_print: bool,
) -> HashMap<String, LockTokenInfo> {
    let ret = view!(stakepooling.list_locktokens_info(0, 100)).unwrap_json::<HashMap<String, LockTokenInfo>>();
    if show_print {
        for (k, v) in &ret {
            println!("StakePoolLockToken=>  {}: {:#?}", k, v);
        }
    }
    ret
}

#[allow(dead_code)]
pub(crate) fn show_user_locktoken_amounts(
    stakepooling: &ContractAccount<StakePooling>,
    user_id: String,
    show_print: bool,
) -> HashMap<String, U128> {
    let ret = view!(stakepooling.list_user_locktoken_amounts(to_va(user_id.clone())))
        .unwrap_json::<HashMap<String, U128>>();
    if show_print {
        println!("User LockTokens for {}: {:#?}", user_id, ret);
    }
    ret
}

#[allow(dead_code)]
pub(crate) fn show_user_locktoken_powers(
    stakepooling: &ContractAccount<StakePooling>,
    user_id: String,
    show_print: bool,
) -> HashMap<String, U128> {
    let ret = view!(stakepooling.list_user_locktoken_powers(to_va(user_id.clone())))
        .unwrap_json::<HashMap<String, U128>>();
    if show_print {
        println!("User LockTokens for {}: {:#?}", user_id, ret);
    }
    ret
}

#[allow(dead_code)]
pub(crate) fn show_unclaim(
    stakepooling: &ContractAccount<StakePooling>,
    user_id: String,
    stakepool_id: String,
    show_print: bool,
) -> U128 {
    let stakepool_info = get_stakepoolinfo(stakepooling, stakepool_id.clone());
    let ret = view!(stakepooling.get_unclaimed_reward(to_va(user_id.clone()), stakepool_id.clone()))
        .unwrap_json::<U128>();
    if show_print {
        println!(
            "User Unclaimed for {}@{}:[CRR:{}, LRR:{}] {}",
            user_id, stakepool_id, stakepool_info.cur_round, stakepool_info.last_round, ret.0
        );
    }
    ret
}

#[allow(dead_code)]
pub(crate) fn show_reward(
    stakepooling: &ContractAccount<StakePooling>,
    user_id: String,
    reward_id: String,
    show_print: bool,
) -> U128 {
    let ret = view!(stakepooling.get_reward(to_va(user_id.clone()), to_va(reward_id.clone())))
        .unwrap_json::<U128>();
    if show_print {
        println!("Reward {} for {}: {}", reward_id, user_id, ret.0);
    }
    ret
}

#[allow(dead_code)]
pub(crate) fn show_storage_balance(stakepooling: &ContractAccount<StakePooling>, staker: String, show_print: bool) -> StorageBalance {
    let ret = view!(stakepooling.storage_balance_of(to_va(staker.clone()))).unwrap_json::<StorageBalance>();
    if show_print {
        println!("total {}, available {}", ret.total.0, ret.available.0);
    }
    ret
}

#[allow(dead_code)]
pub(crate) fn show_lostfound(
    stakepooling: &ContractAccount<StakePooling>,
    show_print: bool,
) -> HashMap<String, U128> {
    let ret = view!(stakepooling.list_lostfound(0, 100)).unwrap_json::<HashMap<String, U128>>();
    if show_print {
        for (k, v) in &ret {
            println!("StakePoolLockToken=>  {}: {:#?}", k, v);
        }
    }
    ret
}

#[allow(dead_code)]
pub(crate) fn show_shashed(
    stakepooling: &ContractAccount<StakePooling>,
    show_print: bool,
) -> HashMap<String, U128> {
    let ret = view!(stakepooling.list_shashed(0, 100)).unwrap_json::<HashMap<String, U128>>();
    if show_print {
        for (k, v) in &ret {
            println!("StakePoolLockToken=>  {}: {:#?}", k, v);
        }
    }
    ret
}

#[allow(dead_code)]
pub(crate) fn get_user_rps(
    stakepooling: &ContractAccount<StakePooling>,
    user_id: String,
    stakepool_id: String,
) -> Option<String> {
    view!(stakepooling.get_user_rps(to_va(user_id), stakepool_id)).unwrap_json::<Option<String>>()
}

#[allow(dead_code)]
pub(crate) fn get_user_locktoken_info(
    stakepooling: &ContractAccount<StakePooling>,
    user_id: String,
    locktoken_id: String,
) -> UserLockTokenInfo {
    view!(stakepooling.get_user_locktoken_info(to_va(user_id), locktoken_id.clone())).unwrap_json::<UserLockTokenInfo>()
}

// =============  Assertions  ===============
#[allow(dead_code)]
pub(crate) fn assert_stakepooling(
    stakepool_info: &StakePoolInfo,
    stakepool_status: String,
    total_reward: u128,
    cur_round: u32,
    last_round: u32,
    claimed_reward: u128,
    unclaimed_reward: u128,
    beneficiary_reward: u128,
) {
    assert_eq!(stakepool_info.stakepool_status, stakepool_status);
    assert_eq!(stakepool_info.total_reward.0, total_reward);
    assert_eq!(stakepool_info.cur_round, cur_round);
    assert_eq!(stakepool_info.last_round, last_round);
    assert_eq!(stakepool_info.claimed_reward.0, claimed_reward);
    assert_eq!(stakepool_info.unclaimed_reward.0, unclaimed_reward);
    assert_eq!(stakepool_info.beneficiary_reward.0, beneficiary_reward);
}

#[allow(dead_code)]
pub(crate) fn assert_strategy(
    strategy_info: &CDStrategyInfo,
    index: usize,
    lock_sec: u32,
    additional: u32,
    enable: bool,
    damage: u32,
) {
    assert_eq!(strategy_info.stake_strategy[index].lock_sec, lock_sec);
    assert_eq!(strategy_info.stake_strategy[index].power_reward_rate, additional);
    assert_eq!(strategy_info.stake_strategy[index].enable, enable);
    assert_eq!(strategy_info.locktoken_slash_rate, damage);
}

// =============  internal methods ================
fn get_stakepoolinfo(stakepooling: &ContractAccount<StakePooling>, stakepool_id: String) -> StakePoolInfo {
    view!(stakepooling.get_stakepool(stakepool_id)).unwrap_json::<StakePoolInfo>()
}

fn get_outdated_stakepoolinfo(stakepooling: &ContractAccount<StakePooling>, stakepool_id: String) -> StakePoolInfo {
    view!(stakepooling.get_outdated_stakepool(stakepool_id)).unwrap_json::<StakePoolInfo>()
}

