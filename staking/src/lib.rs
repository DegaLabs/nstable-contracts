/*!
* STAKING
*
* lib.rs is the main entry point.
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    Promise, PromiseResult, StorageUsage
};
//use near_sdk::require;
//use crate::*;

use near_sdk::json_types::U128;

near_sdk::setup_alloc!();


#[derive(BorshDeserialize, BorshSerialize)]
pub struct TokenDeposit {
    tokenAmount: u128,  
    weight: u128,
    lockedFrom: u128,
    lockedUntil: u128,
}
#[derive(BorshDeserialize, BorshSerialize)]
pub struct TokenLock {
    tokenAmount: u128,  
    lockedFrom: u128,
    lockedUntil: u128,
}
#[derive(BorshDeserialize, BorshSerialize)]
pub struct UserInfo {
    lockAmount: u128,  
    stakeFrom: u128,
    rewardDebt: u128,
    pendingReward:u128,
    stakeWeight: u128,
    deposits : Vec<TokenDeposit>,
    locks : Vec<TokenLock>
}
#[derive(BorshDeserialize, BorshSerialize)]
pub struct PoolInfo {
    lpToken: ValidAccountId,  
    totalWeight: u128,
    allocPoint: u128,
    lastRewardBlock:u128,
    accRewardPerBlock: u128,
    minLockedDuration : u128,
    earlyWithdrawPenaltyRate : u128,

}


//pub pools: Vector<PoolInfo>


#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractData {

    // owner of this contract
    owner_id: AccountId,
    tokenPerBlock: u64,
    startBlock: u64,
    rewardToken: ValidAccountId,
    totalAllocPoint: u8,
    poolLockedTimeAfterUnstake: u128,
    pools: Vec<PoolInfo>
    
}





/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedContractData {
    Current(ContractData),
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
    pub fn new(owner_id: ValidAccountId, _tokenPerBlock: u64, _startBlock: u64, _rewardToken: ValidAccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        assert!(_startBlock > 0, "Start block should be larger than 0");
        Self {
            data: VersionedContractData::Current(ContractData {
                owner_id: owner_id.into(),
                tokenPerBlock: _tokenPerBlock,
                startBlock: _startBlock,     
                rewardToken: _rewardToken,
                totalAllocPoint:0,
                poolLockedTimeAfterUnstake: 2*86400,
                pools: Vec::new(),
            }),
        }
    }
}

impl Contract {
    fn data(&self) -> &ContractData {
        match &self.data {
            VersionedContractData::Current(data) => data,
        }
    }

    fn data_mut(&mut self) -> &mut ContractData {
        match &mut self.data {
            VersionedContractData::Current(data) => data,
        }
    }
}

#[near_bindgen]
impl Contract {

    pub fn set_owner(&mut self, owner_id: ValidAccountId) {
        self.assert_owner();
        self.data_mut().owner_id = owner_id.into();
    }

    /// Migration function between versions.
    /// For next version upgrades, change this function.
    /// 
    /// 
    /// 
    


    pub fn addNewPool(&mut self,
        _lpToken: ValidAccountId,  
        _totalWeight: u128,
        _allocPoint: u128,
        _lastRewardBlock:u128,
        _accRewardPerBlock: u128,
        _minLockedDuration : u128,
        _earlyWithdrawPenaltyRate : u128,    

    ) -> usize {
        self.assert_owner();
        //assert_eq!(self.pools.len() as u32, 0, "{}", ERR104_INITIALIZED);
        //check_token_duplicates(&tokens);
        let newPool =  PoolInfo {
            lpToken: _lpToken,  
            totalWeight: 1,
            allocPoint: 1,
            lastRewardBlock: _lastRewardBlock,
            accRewardPerBlock: _accRewardPerBlock,
            minLockedDuration: _minLockedDuration ,
            earlyWithdrawPenaltyRate: _earlyWithdrawPenaltyRate,
        };

        let prev_storage = env::storage_usage();
        self.internal_check_storage(prev_storage);

        //let id = self.data.Current().pools.len() as u64;
        // exchange share was registered at creation time


        self.data_mut().pools.push(newPool);
        let id = self.data_mut().pools.len();
        println!("Success create pool with pool id is " );
        return id - 1;
       // newPool.share_register(&env::current_account_id());
        //self.data.pools.push(&pool);
        
    

    }


    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "ERR_NOT_ALLOWED"
        );
        let contract: Contract = env::state_read().expect("ERR_NOT_INITIALIZED");
        contract
    }

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.data().owner_id,
            "ERR_NOT_ALLOWED"
        );
    }


        /// Check how much storage taken costs and refund the left over back.
        fn internal_check_storage(&self, prev_storage: StorageUsage) {
            let storage_cost = env::storage_usage()
                .checked_sub(prev_storage)
                .unwrap_or_default() as Balance
                * env::storage_byte_cost();
    
            let refund = env::attached_deposit().checked_sub(storage_cost).expect(
                format!(
                    "ERR_STORAGE_DEPOSIT need {}, attatched {}",
                    storage_cost,
                    env::attached_deposit()
                )
                .as_str(),
            );
            if refund > 0 {
                Promise::new(env::predecessor_account_id()).transfer(refund);
            }
        }
}          