//! StakePoolLockToken stores information per locktoken about 
//! staked locktoken amount and stakepools under it.

use std::collections::HashSet;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{Balance};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::{U128};
use crate::errors::*;
use crate::stakepool::StakePoolId;
use crate::utils::parse_locktoken_id;


/// For MFT, LockTokenId composes of token_contract_id 
/// and token's inner_id in that contract. 
/// For FT, LockTokenId is the token_contract_id.
pub(crate) type LockTokenId = String;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum LockTokenType {
    FT,
    MFT,
}


#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct StakePoolLockToken {
    /// The StakePooling Token this StakePoolLockToken represented for
    pub locktoken_id: LockTokenId,
    /// The locktoken is a FT or MFT, enum size is 2 bytes?
    pub locktoken_type: LockTokenType,
    /// all stakepools that accepted this locktoken
    /// StakePoolId = {locktoken_id}#{next_index}
    pub stakepools: HashSet<StakePoolId>,
    pub next_index: u32,
    /// total (staked) balance of this locktoken (StakePooling Token)
    pub total_locktoken_amount: Balance,
    pub total_locktoken_power: Balance,
    pub min_deposit: Balance,
    /// the CD Account slash rate for this locktoken
    pub slash_rate: u32,
}

impl StakePoolLockToken {
    pub fn new(locktoken_id: &LockTokenId, min_deposit: Balance, default_slash_rate: u32) -> Self {
        let (token_id, token_index) = parse_locktoken_id(locktoken_id);
        let locktoken_type: LockTokenType;
        if token_id == token_index {
            locktoken_type = LockTokenType::FT;
        } else {
            locktoken_type = LockTokenType::MFT;
        }
        Self {
            locktoken_id: locktoken_id.clone(),
            locktoken_type,
            stakepools: HashSet::new(),
            next_index: 0,
            total_locktoken_amount: 0,
            total_locktoken_power: 0,
            min_deposit,
            slash_rate: default_slash_rate,
        }
    }

    pub fn add_locktoken_amount(&mut self, amount: Balance) {
        self.total_locktoken_amount += amount;
    }

    /// return locktoken amount remains.
    pub fn sub_locktoken_amount(&mut self, amount: Balance) -> Balance {
        assert!(self.total_locktoken_amount >= amount, "{}", ERR500);
        self.total_locktoken_amount -= amount;
        self.total_locktoken_amount
    }

    pub fn add_locktoken_power(&mut self, amount: Balance) {
        self.total_locktoken_power += amount;
    }

    /// return locktoken power remains.
    pub fn sub_locktoken_power(&mut self, amount: Balance) -> Balance {
        assert!(self.total_locktoken_power >= amount, "{}", ERR500);
        self.total_locktoken_power -= amount;
        self.total_locktoken_power
    }

}

/// Versioned StakePoolLockToken, used for lazy upgrade.
/// Which means this structure would upgrade automatically when used.
/// To achieve that, each time the new version comes in, 
/// each function of this enum should be carefully re-code!
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedStakePoolLockToken {
    V101(StakePoolLockToken),
}

impl VersionedStakePoolLockToken {

    pub fn new(locktoken_id: &LockTokenId, min_deposit: Balance, default_slash_rate: u32) -> Self {
        VersionedStakePoolLockToken::V101(StakePoolLockToken::new(locktoken_id, min_deposit, default_slash_rate))
    }

    /// Upgrades from other versions to the currently used version.
    pub fn upgrade(self) -> Self {
        match self {
            VersionedStakePoolLockToken::V101(stakepool_locktoken) => VersionedStakePoolLockToken::V101(stakepool_locktoken),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn need_upgrade(&self) -> bool {
        match self {
            VersionedStakePoolLockToken::V101(_) => false,
            _ => true,
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref(&self) -> &StakePoolLockToken {
        match self {
            VersionedStakePoolLockToken::V101(stakepool_locktoken) => stakepool_locktoken,
            _ => unimplemented!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref_mut(&mut self) -> &mut StakePoolLockToken {
        match self {
            VersionedStakePoolLockToken::V101(stakepool_locktoken) => stakepool_locktoken,
            _ => unimplemented!(),
        }
    }
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct LockTokenInfo {
    pub locktoken_id: LockTokenId,
    pub locktoken_type: String,
    pub stakepools: Vec<StakePoolId>,
    pub next_index: u32,
    pub amount: U128,
    pub power: U128,
    pub min_deposit: U128,
    pub slash_rate: u32,
}

impl From<&StakePoolLockToken> for LockTokenInfo {
    fn from(fs: &StakePoolLockToken) -> Self {

        let locktoken_type = match fs.locktoken_type {
            LockTokenType::FT => "FT".to_string(),
            LockTokenType::MFT => "MFT".to_string(),
        };
        Self {
            locktoken_id: fs.locktoken_id.clone(),
            locktoken_type,
            next_index: fs.next_index,
            amount: fs.total_locktoken_amount.into(),
            power: fs.total_locktoken_power.into(),
            min_deposit: fs.min_deposit.into(),
            slash_rate: fs.slash_rate,
            stakepools: fs.stakepools.iter().map(|key| key.clone()).collect(),
        }
    }
}
