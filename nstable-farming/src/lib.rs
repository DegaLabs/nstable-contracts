/*!
* nstable-StakePooling-v2
*
* lib.rs is the main entry point.
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::{env, near_bindgen, Balance, AccountId, PanicOnDefault, PromiseResult};
use near_sdk::BorshStorageKey;

use crate::stakepool::{StakePool, StakePoolId};
use crate::simple_stakepool::{RPS};
use crate::stakepool_locktoken::{VersionedStakePoolLockToken, LockTokenId};
use crate::staker::{VersionedStaker, Staker};
use crate::utils::{STRATEGY_LIMIT, DENOM, DEFAULT_STAKEPOOL_EXPIRE_SEC};
use crate::errors::{ERR32_NOT_ENOUGH_LOCKTOKEN, ERR25_CALLBACK_POST_WITHDRAW_INVALID};

// for simulator test
pub use crate::simple_stakepool::HRSimpleStakePoolTerms;
pub use crate::view::StakePoolInfo;
pub use crate::view::CDAccountInfo;
pub use crate::view::CDStrategyInfo;
pub use crate::view::UserLockTokenInfo;

use crate::legacy::ContractDataV200;


mod utils;
mod errors;
mod staker;
mod token_receiver;
mod stakepool_locktoken;
mod stakepool;
mod simple_stakepool;
mod storage_impl;

mod actions_of_stakepool;
// mod actions_of_staker;
mod actions_of_locktoken;
mod actions_of_reward;
mod view;

mod owner;
mod legacy;

near_sdk::setup_alloc!();


#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKeys {
    LockToken,
    StakePool,
    OutdatedStakePool,
    Staker,
    RewardInfo,
    UserRps { account_id: AccountId },
    CDAccount { account_id: AccountId },
    LockTokenSlashed,
    LockTokenLostfound,
    Operator,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CDStakeItem{
    pub enable: bool,
    /// minimum duration of locktoken lock in secs.
    pub lock_sec: u32,
    /// power reward multiple rate numerator.
    pub power_reward_rate: u32,
    
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct CDStrategy {
    /// total of STRATEGY_LIMIT different strategies are supported.
    pub stake_strategy: Vec<CDStakeItem>,
    /// locktoken slash rate numerator.
    pub locktoken_slash_rate: u32,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractData {

    // owner of this contract
    owner_id: AccountId,
    
    // record locktokens and the stakepools under it.
    // locktokens: UnorderedMap<LockTokenId, StakePoolLockToken>,
    locktokens: UnorderedMap<LockTokenId, VersionedStakePoolLockToken>,

    // all slashed locktoken would recorded in here
    locktokens_slashed: UnorderedMap<LockTokenId, Balance>,

    // if unstake locktoken encounter error, the locktoken would go to here
    locktokens_lostfound: UnorderedMap<LockTokenId, Balance>,

    // each staker has a structure to describe
    // stakers: LookupMap<AccountId, Staker>,
    stakers: LookupMap<AccountId, VersionedStaker>,

    stakepools: UnorderedMap<StakePoolId, StakePool>,
    outdated_stakepools: UnorderedMap<StakePoolId, StakePool>,

    // for statistic
    staker_count: u64,
    reward_info: UnorderedMap<AccountId, Balance>,

    // strategy for staker CDAccount
    cd_strategy: CDStrategy,

    stakepool_expire_sec: u32,

    /// Set of guardians.
    operators: UnorderedSet<AccountId>,
}

/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedContractData {
    V200(ContractDataV200),
    V201(ContractData),
}

impl VersionedContractData {}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {

    data: VersionedContractData,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            data: VersionedContractData::V201(ContractData {
                owner_id: owner_id.into(),
                staker_count: 0,
                locktokens: UnorderedMap::new(StorageKeys::LockToken),
                locktokens_slashed: UnorderedMap::new(StorageKeys::LockTokenSlashed),
                locktokens_lostfound: UnorderedMap::new(StorageKeys::LockTokenLostfound),
                stakers: LookupMap::new(StorageKeys::Staker),
                stakepools: UnorderedMap::new(StorageKeys::StakePool),
                outdated_stakepools: UnorderedMap::new(StorageKeys::OutdatedStakePool),
                reward_info: UnorderedMap::new(StorageKeys::RewardInfo),
                cd_strategy: CDStrategy{
                    stake_strategy: vec![CDStakeItem{
                        lock_sec: 0,
                        power_reward_rate: 0,
                        enable: false
                    }; STRATEGY_LIMIT],
                    locktoken_slash_rate: 0,
                },
                stakepool_expire_sec: DEFAULT_STAKEPOOL_EXPIRE_SEC,
                operators: UnorderedSet::new(StorageKeys::Operator),
            }),
        }
    }

    /// if withdraw locktoken encounter async error, it would go to locktokens_lostfound
    #[private]
    pub fn callback_withdraw_locktoken(&mut self, locktoken_id: LockTokenId, sender_id: AccountId, amount: U128) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            ERR25_CALLBACK_POST_WITHDRAW_INVALID
        );
        let amount: Balance = amount.into();
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                env::log(
                    format!(
                        "{} withdraw {} locktoken with amount {}, Failed.",
                        sender_id, locktoken_id, amount,
                    )
                    .as_bytes(),
                );
                // all locktoken amount go to lostfound
                let locktoken_amount = self.data().locktokens_lostfound.get(&locktoken_id).unwrap_or(0);
                self.data_mut().locktokens_lostfound.insert(&locktoken_id, &(locktoken_amount + amount));
            },
            PromiseResult::Successful(_) => {
                env::log(
                    format!(
                        "{} withdraw {} locktoken with amount {}, Succeed.",
                        sender_id, locktoken_id, amount,
                    )
                    .as_bytes(),
                );
            }
        }
    }

    /// if withdraw locktoken lostfound encounter async error, it would go to locktokens_lostfound
    #[private]
    pub fn callback_withdraw_locktoken_lostfound(&mut self, locktoken_id: LockTokenId, sender_id: AccountId, amount: U128) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            ERR25_CALLBACK_POST_WITHDRAW_INVALID
        );
        let amount: Balance = amount.into();
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                env::log(
                    format!(
                        "Owner help {} to withdraw {} locktoken from lostfound with amount {}, Failed.",
                        sender_id, locktoken_id, amount,
                    )
                    .as_bytes(),
                );
                // all locktoken amount go to lostfound
                let locktoken_amount = self.data().locktokens_lostfound.get(&locktoken_id).unwrap_or(0);
                self.data_mut().locktokens_lostfound.insert(&locktoken_id, &(locktoken_amount + amount));
            },
            PromiseResult::Successful(_) => {
                env::log(
                    format!(
                        "Owner help {} withdraw {} locktoken from lostfound with amount {}, Succeed.",
                        sender_id, locktoken_id, amount,
                    )
                    .as_bytes(),
                );
            }
        }
    }

    /// if withdraw locktoken slashed encounter async error, it would go back to locktokens_slashed
    #[private]
    pub fn callback_withdraw_locktoken_slashed(&mut self, locktoken_id: LockTokenId, amount: U128) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            ERR25_CALLBACK_POST_WITHDRAW_INVALID
        );
        let amount: Balance = amount.into();
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                env::log(
                    format!(
                        "Owner withdraw {} locktoken slashed with amount {}, Failed.",
                        locktoken_id, amount,
                    )
                    .as_bytes(),
                );
                // all locktoken amount go back to locktoken slashed
                let locktoken_amount = self.data().locktokens_slashed.get(&locktoken_id).unwrap_or(0);
                self.data_mut().locktokens_slashed.insert(&locktoken_id, &(locktoken_amount + amount));
            },
            PromiseResult::Successful(_) => {
                env::log(
                    format!(
                        "Owner withdraw {} locktoken with amount {}, Succeed.",
                        locktoken_id, amount,
                    )
                    .as_bytes(),
                );
            }
        }
    }
}

impl Contract {
    fn data(&self) -> &ContractData {
        match &self.data {
            VersionedContractData::V201(data) => data,
            _ => unimplemented!(),
        }
    }

    fn data_mut(&mut self) -> &mut ContractData {
        match &mut self.data {
            VersionedContractData::V201(data) => data,
            _ => unimplemented!(),
        }
    }

    fn is_owner_or_operators(&self) -> bool {
        env::predecessor_account_id() == self.data().owner_id 
            || self.data().operators.contains(&env::predecessor_account_id())
    }
}

#[cfg(test)]
mod tests {

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance, MockedBlockchain};
    use near_sdk::json_types::{ValidAccountId, U128};
    use simple_stakepool::{HRSimpleStakePoolTerms};
    use near_contract_standards::storage_management::{StorageBalance, StorageManagement};

    use super::utils::*;
    use super::*;

    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new(accounts(0));
        (context, contract)
    }

    fn create_stakepool(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        locktoken: ValidAccountId,
        reward: ValidAccountId,
        session_amount: Balance,
        session_interval: u32,
    ) -> StakePoolId {
        // storage needed: 341
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(env::storage_byte_cost() * 575)
            .build());
        contract.create_simple_stakepool(HRSimpleStakePoolTerms {
            locktoken_id: locktoken.into(),
            reward_token: reward.into(),
            start_at: 0,
            reward_per_session: U128(session_amount),
            session_interval: session_interval,
        }, Some(U128(10)))
    }

    fn deposit_reward(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        amount: u128,
        time_stamp: u32,
    ) {
        testing_env!(context
            .predecessor_account_id(accounts(2))
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(accounts(0), U128(amount), format!("{{\"Reward\": {{\"stakepool_id\": \"{}\"}}}}", "bob#0"));
    }

    fn register_staker(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        staker: ValidAccountId,
    ) -> StorageBalance {
        testing_env!(context
            .predecessor_account_id(staker.clone())
            .is_view(false)
            .attached_deposit(to_yocto("0.1"))
            .build());
        contract.storage_deposit(Some(staker), Some(true))
    }

    fn storage_withdraw(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        staker: ValidAccountId,
    ) -> StorageBalance {
        testing_env!(context
            .predecessor_account_id(staker.clone())
            .is_view(false)
            .attached_deposit(1)
            .build());
        contract.storage_withdraw(None)
    }

    fn deposit_locktoken(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        staker: ValidAccountId,
        time_stamp: u32,
        amount: Balance,
    ) {
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(staker, U128(amount), String::from(""));
    }    

    fn withdraw_locktoken(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        staker: ValidAccountId,
        time_stamp: u32,
        amount: Balance,
    ) {
        testing_env!(context
            .predecessor_account_id(staker)
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.withdraw_locktoken(accounts(1).into(), U128(amount));
    } 

    fn claim_reward(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        staker: ValidAccountId,
        time_stamp: u32
    ) {
        testing_env!(context
            .predecessor_account_id(staker)
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.claim_reward_by_stakepool(String::from("bob#0"));
    }

    fn claim_reward_by_locktoken(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        staker: ValidAccountId,
        time_stamp: u32
    ) {
        testing_env!(context
            .predecessor_account_id(staker)
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.claim_reward_by_locktoken(String::from("bob"));
    }

    fn remove_stakepool(context: &mut VMContextBuilder, contract: &mut Contract, time_stamp: u32) {
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .build());
        contract.force_clean_stakepool(String::from("bob#0"));
    }

    fn remove_user_rps(context: &mut VMContextBuilder, contract: &mut Contract, staker: ValidAccountId, stakepool_id: String, time_stamp: u32) -> bool {
        testing_env!(context
            .predecessor_account_id(staker)
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .build());
        contract.remove_user_rps_by_stakepool(stakepool_id)
    }

    fn to_yocto(value: &str) -> u128 {
        let vals: Vec<_> = value.split('.').collect();
        let part1 = vals[0].parse::<u128>().unwrap() * 10u128.pow(24);
        if vals.len() > 1 {
            let power = vals[1].len() as u32;
            let part2 = vals[1].parse::<u128>().unwrap() * 10u128.pow(24 - power);
            part1 + part2
        } else {
            part1
        }
    }

    #[test]
    fn test_basics() {

        let (mut context, mut contract) = setup_contract();
        // locktoken is bob, reward is charlie
        let stakepool_id = create_stakepool(&mut context, &mut contract,
            accounts(1), accounts(2), 5000, 50);
        assert_eq!(stakepool_id, String::from("bob#0"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.stakepool_kind, String::from("SIMPLE_STAKEPOOL"));
        assert_eq!(stakepool_info.stakepool_status, String::from("Created"));
        assert_eq!(stakepool_info.locktoken_id, String::from("bob"));
        assert_eq!(stakepool_info.reward_token, String::from("charlie"));
        assert_eq!(stakepool_info.reward_per_session, U128(5000));
        assert_eq!(stakepool_info.session_interval, 50);

        // deposit 50k, can last 10 rounds from 0 to 9
        deposit_reward(&mut context, &mut contract, 50000, 100);
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.stakepool_status, String::from("Running"));
        assert_eq!(stakepool_info.start_at, 100);

        // Staker accounts(0) come in round 1
        register_staker(&mut context, &mut contract, accounts(0));
        deposit_locktoken(&mut context, &mut contract, accounts(0), 160, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.beneficiary_reward, U128(5000));
        assert_eq!(stakepool_info.cur_round, 1);
        assert_eq!(stakepool_info.last_round, 1);

        // move to round 2, 5k unclaimed for accounts(0)
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(210))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(5000));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 2);
        assert_eq!(stakepool_info.last_round, 1);

        // Staker accounts(3) come in 
        register_staker(&mut context, &mut contract, accounts(3));
        // deposit locktoken
        deposit_locktoken(&mut context, &mut contract, accounts(3), 260, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(10000));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 3);
        assert_eq!(stakepool_info.last_round, 3);

        // move to round 4, 
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(320))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(12500));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(2500));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 4);
        assert_eq!(stakepool_info.last_round, 3);

        // remove all locktokens at round 5
        println!("----> remove all locktokens at round 5");
        withdraw_locktoken(&mut context, &mut contract, accounts(0), 360, 10);
        withdraw_locktoken(&mut context, &mut contract, accounts(3), 370, 10);
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(380)).is_view(true).build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let rewarded = contract.get_reward(accounts(0), accounts(2));
        assert_eq!(rewarded, U128(15000));
        let rewarded = contract.get_reward(accounts(3), accounts(2));
        assert_eq!(rewarded, U128(5000));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 5);
        assert_eq!(stakepool_info.last_round, 5);


        // move to round 7, account3 come in again
        println!("----> move to round 7, account3 come in again");
        deposit_locktoken(&mut context, &mut contract, accounts(3), 460, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.beneficiary_reward, U128(15000));
        assert_eq!(stakepool_info.cur_round, 7);
        assert_eq!(stakepool_info.last_round, 7);

        // move to round 8, account0 come in again
        println!("----> move to round 8, account0 come in again");
        deposit_locktoken(&mut context, &mut contract, accounts(0), 520, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(5000));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 8);
        assert_eq!(stakepool_info.last_round, 8);

        // move to round 9,
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(580))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(2500));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(7500));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 9);
        assert_eq!(stakepool_info.last_round, 8);
        assert_eq!(stakepool_info.stakepool_status, String::from("Running"));

        // move to round 10,
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(610))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(5000));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(10000));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 10);
        assert_eq!(stakepool_info.last_round, 8);
        assert_eq!(stakepool_info.stakepool_status, String::from("Ended"));

        // claim reward 
        println!("----> accounts(0) and accounts(3) claim reward");
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(710))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(5000));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(10000));
        claim_reward(&mut context, &mut contract, accounts(0), 720);
        claim_reward(&mut context, &mut contract, accounts(3), 730);
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(740)).is_view(true).build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let rewarded = contract.get_reward(accounts(0), accounts(2));
        assert_eq!(rewarded, U128(20000));
        let rewarded = contract.get_reward(accounts(3), accounts(2));
        assert_eq!(rewarded, U128(15000));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 10);
        assert_eq!(stakepool_info.last_round, 10);

        // clean stakepool
        println!("----> clean stakepool");
        remove_stakepool(&mut context, &mut contract, 750 + DEFAULT_STAKEPOOL_EXPIRE_SEC);
        assert!(contract.get_stakepool(stakepool_id.clone()).is_none());

        // remove user rps
        println!("----> remove user rps");
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(760)).is_view(true).build());
        let ret = remove_user_rps(&mut context, &mut contract, accounts(0).into(), String::from("bob#0"), 770);
        assert!(ret);
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(780)).is_view(true).build());

        // withdraw locktoken
        println!("----> accounts(0) and accounts(3) withdraw locktoken");
        withdraw_locktoken(&mut context, &mut contract, accounts(0), 800, 10);
        withdraw_locktoken(&mut context, &mut contract, accounts(3), 810, 10);
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(820)).is_view(true).build());
        let rewarded = contract.get_reward(accounts(0), accounts(2));
        assert_eq!(rewarded, U128(20000));
        let rewarded = contract.get_reward(accounts(3), accounts(2));
        assert_eq!(rewarded, U128(15000));
        
    }

    #[test]
    fn test_unclaimed_rewards() {

        let (mut context, mut contract) = setup_contract();
        // locktoken is bob, reward is charlie
        let stakepool_id = create_stakepool(&mut context, &mut contract,
            accounts(1), accounts(2), to_yocto("1"), 50);
        assert_eq!(stakepool_id, String::from("bob#0"));

        // deposit 10, can last 10 rounds from 0 to 9
        deposit_reward(&mut context, &mut contract, to_yocto("10"), 100);

        // Staker1 accounts(0) come in round 0
        register_staker(&mut context, &mut contract, accounts(0));
        deposit_locktoken(&mut context, &mut contract, accounts(0), 110, to_yocto("1"));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed, U128(0));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 0);
        assert_eq!(stakepool_info.last_round, 0);
        assert_eq!(stakepool_info.claimed_reward.0, 0);
        assert_eq!(stakepool_info.unclaimed_reward.0, 0);

        // move to round 1,
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(160))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("1"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 1);
        assert_eq!(stakepool_info.last_round, 0);
        assert_eq!(stakepool_info.claimed_reward.0, to_yocto("0"));
        assert_eq!(stakepool_info.unclaimed_reward.0, to_yocto("1"));

        // Staker2 accounts(3) come in round 1
        register_staker(&mut context, &mut contract, accounts(3));
        // deposit locktoken
        deposit_locktoken(&mut context, &mut contract, accounts(3), 180, to_yocto("1"));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("1"));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0"));

        // move to round 2,
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(210))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("1.5"));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0.5"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 2);
        assert_eq!(stakepool_info.last_round, 1);
        assert_eq!(stakepool_info.claimed_reward.0, to_yocto("0"));
        assert_eq!(stakepool_info.unclaimed_reward.0, to_yocto("2"));

        // staker1 claim reward by stakepool_id at round 3
        claim_reward(&mut context, &mut contract, accounts(0), 260);
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0"));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("1"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 3);
        assert_eq!(stakepool_info.last_round, 3);
        assert_eq!(stakepool_info.claimed_reward.0, to_yocto("2"));
        assert_eq!(stakepool_info.unclaimed_reward.0, to_yocto("1"));

        // staker2 claim reward by locktoken_id at round 4
        claim_reward_by_locktoken(&mut context, &mut contract, accounts(3), 310);
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0.5"));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 4);
        assert_eq!(stakepool_info.last_round, 4);
        assert_eq!(stakepool_info.claimed_reward.0, to_yocto("3.5"));
        assert_eq!(stakepool_info.unclaimed_reward.0, to_yocto("0.5"));

        // staker1 unstake half lpt at round 5
        withdraw_locktoken(&mut context, &mut contract, accounts(0), 360, to_yocto("0.4"));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0"));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0.5"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 5);
        assert_eq!(stakepool_info.last_round, 5);
        assert_eq!(stakepool_info.claimed_reward.0, to_yocto("4.5"));
        assert_eq!(stakepool_info.unclaimed_reward.0, to_yocto("0.5"));

        // staker2 unstake all his lpt at round 6
        withdraw_locktoken(&mut context, &mut contract, accounts(3), 410, to_yocto("1"));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0.375"));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 6);
        assert_eq!(stakepool_info.last_round, 6);
        assert_eq!(stakepool_info.claimed_reward.0, to_yocto("5.625"));
        assert_eq!(stakepool_info.unclaimed_reward.0, to_yocto("0.375"));

        // move to round 7
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(460))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("1.374999999999999999999999"));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 7);
        assert_eq!(stakepool_info.last_round, 6);
        assert_eq!(stakepool_info.claimed_reward.0, to_yocto("5.625"));
        assert_eq!(stakepool_info.unclaimed_reward.0, to_yocto("1.375"));
        withdraw_locktoken(&mut context, &mut contract, accounts(0), 470, to_yocto("0.6"));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0"));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), stakepool_id.clone());
        assert_eq!(unclaimed.0, to_yocto("0"));
        let stakepool_info = contract.get_stakepool(stakepool_id.clone()).expect("Error");
        assert_eq!(stakepool_info.cur_round, 7);
        assert_eq!(stakepool_info.last_round, 7);
        assert_eq!(stakepool_info.claimed_reward.0, to_yocto("6.999999999999999999999999"));
        assert_eq!(stakepool_info.unclaimed_reward.0, 1);
        
    }

    #[test]
    #[should_panic(expected = "E14: no storage can withdraw")]
    fn test_storage_withdraw() {
        let (mut context, mut contract) = setup_contract();
        // Staker1 accounts(0) come in round 0
        register_staker(&mut context, &mut contract, accounts(0));
        // println!("locked: {}, deposited: {}", sb.total.0, sb.available.0);
        storage_withdraw(&mut context, &mut contract, accounts(0));
    }
}