use crate::*;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PrevContractData {

    // owner of this contract
    pub owner_id: AccountId,
    
    // record locktokens and the stakepools under it.
    // locktokens: UnorderedMap<LockTokenId, StakePoolLockToken>,
    pub locktokens: UnorderedMap<LockTokenId, VersionedStakePoolLockToken>,

    // all slashed locktoken would recorded in here
    pub locktokens_slashed: UnorderedMap<LockTokenId, Balance>,

    // if unstake locktoken encounter error, the locktoken would go to here
    pub locktokens_lostfound: UnorderedMap<LockTokenId, Balance>,

    // each staker has a structure to describe
    // stakers: LookupMap<AccountId, Staker>,
    pub stakers: LookupMap<AccountId, VersionedStaker>,

    pub stakepools: UnorderedMap<StakePoolId, StakePool>,
    pub outdated_stakepools: UnorderedMap<StakePoolId, StakePool>,

    // for statistic
    pub staker_count: u64,
    pub reward_info: UnorderedMap<AccountId, Balance>,

    // strategy for staker CDAccount
    pub cd_strategy: CDStrategy,
}

/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum PrevVersionedContractData {
    Current(PrevContractData),
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct PrevContract {

    pub data: PrevVersionedContractData,
}


#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractDataV200 {

    // owner of this contract
    pub owner_id: AccountId,
    
    // record locktokens and the stakepools under it.
    // locktokens: UnorderedMap<LockTokenId, StakePoolLockToken>,
    pub locktokens: UnorderedMap<LockTokenId, VersionedStakePoolLockToken>,

    // all slashed locktoken would recorded in here
    pub locktokens_slashed: UnorderedMap<LockTokenId, Balance>,

    // if unstake locktoken encounter error, the locktoken would go to here
    pub locktokens_lostfound: UnorderedMap<LockTokenId, Balance>,

    // each staker has a structure to describe
    // stakers: LookupMap<AccountId, Staker>,
    pub stakers: LookupMap<AccountId, VersionedStaker>,

    pub stakepools: UnorderedMap<StakePoolId, StakePool>,
    pub outdated_stakepools: UnorderedMap<StakePoolId, StakePool>,

    // for statistic
    pub staker_count: u64,
    pub reward_info: UnorderedMap<AccountId, Balance>,

    // strategy for staker CDAccount
    pub cd_strategy: CDStrategy,
}