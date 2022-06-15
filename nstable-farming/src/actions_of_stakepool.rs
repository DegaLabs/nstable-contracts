use crate::errors::*;
use crate::utils::{gen_stakepool_id, parse_stakepool_id, MAX_STAKEPOOL_NUM, MIN_LOCKTOKEN_DEPOSIT};
use crate::*;
use near_sdk::json_types::U128;
use near_sdk::{env, near_bindgen};
use simple_stakepool::{HRSimpleStakePoolTerms, SimpleStakePool};

#[near_bindgen]
impl Contract {
    /// create stakepool and pay for its storage fee
    #[payable]
    pub fn create_simple_stakepool(
        &mut self,
        terms: HRSimpleStakePoolTerms,
        min_deposit: Option<U128>,
    ) -> StakePoolId {
        assert!(self.is_owner_or_operators(), "ERR_NOT_ALLOWED");

        let min_deposit: u128 = min_deposit.unwrap_or(U128(MIN_LOCKTOKEN_DEPOSIT)).0;

        let stakepool_id = self.internal_add_stakepool(&terms, min_deposit);

        stakepool_id
    }

    /// force clean, only those stakepool_expire_sec after ended can be clean
    pub fn force_clean_stakepool(&mut self, stakepool_id: String) {
        assert!(self.is_owner_or_operators(), "ERR_NOT_ALLOWED");
        self.internal_remove_stakepool_by_stakepool_id(&stakepool_id)
    }

    /// Only a stakepool without any reward deposited can be cancelled
    pub fn cancel_stakepool(&mut self, stakepool_id: String) {
        assert!(self.is_owner_or_operators(), "ERR_NOT_ALLOWED");
        self.internal_cancel_stakepool(&stakepool_id)
    }
}

impl Contract {
    /// Adds given stakepool to the vec and returns it's id.
    /// If there is not enough attached balance to cover storage, fails.
    /// If too much attached - refunds it back.
    fn internal_add_stakepool(&mut self, terms: &HRSimpleStakePoolTerms, min_deposit: Balance) -> StakePoolId {
        // let mut stakepool_locktoken = self.get_locktoken_default(&terms.locktoken_id, min_deposit);
        let mut stakepool_locktoken: VersionedStakePoolLockToken;
        if let Some(fs) = self.get_locktoken_wrapped(&terms.locktoken_id) {
            stakepool_locktoken = fs;
            env::log(
                format!(
                    "New stakepool created In locktoken {}, with existed min_deposit {}",
                    terms.locktoken_id,
                    stakepool_locktoken.get_ref().min_deposit
                )
                .as_bytes(),
            );
        } else {
            stakepool_locktoken = VersionedStakePoolLockToken::new(
                &terms.locktoken_id,
                min_deposit,
                self.data().cd_strategy.locktoken_slash_rate,
            );
            env::log(
                format!(
                    "The first stakepool created In locktoken {}, with min_deposit {}",
                    terms.locktoken_id,
                    stakepool_locktoken.get_ref().min_deposit
                )
                .as_bytes(),
            );
        }

        assert!(
            stakepool_locktoken.get_ref().stakepools.len() < MAX_STAKEPOOL_NUM,
            "{}",
            ERR36_STAKEPOOLS_NUM_HAS_REACHED_LIMIT
        );

        let stakepool_id: StakePoolId = gen_stakepool_id(&terms.locktoken_id, stakepool_locktoken.get_ref().next_index as usize);

        let stakepool = StakePool::SimpleStakePool(SimpleStakePool::new(stakepool_id.clone(), terms.into()));

        stakepool_locktoken.get_ref_mut().stakepools.insert(stakepool_id.clone());
        stakepool_locktoken.get_ref_mut().next_index += 1;
        self.data_mut().locktokens.insert(&terms.locktoken_id, &stakepool_locktoken);
        self.data_mut().stakepools.insert(&stakepool_id.clone(), &stakepool);
        stakepool_id
    }

    fn internal_remove_stakepool_by_stakepool_id(&mut self, stakepool_id: &StakePoolId) {
        assert!(
            self.data()
                .stakepools
                .get(stakepool_id)
                .expect(ERR41_STAKEPOOL_NOT_EXIST)
                .can_be_removed(self.data().stakepool_expire_sec),
            "StakePool can NOT be removed now"
        );

        let mut stakepool = self
            .data_mut()
            .stakepools
            .remove(stakepool_id)
            .expect(ERR41_STAKEPOOL_NOT_EXIST);
        stakepool.move_to_clear();
        self.data_mut().outdated_stakepools.insert(stakepool_id, &stakepool);

        let (locktoken_id, _) = parse_stakepool_id(stakepool_id);
        let mut stakepool_locktoken = self.get_locktoken_wrapped(&locktoken_id).expect(ERR31_LOCKTOKEN_NOT_EXIST);
        stakepool_locktoken.get_ref_mut().stakepools.remove(stakepool_id);
        self.data_mut().locktokens.insert(&locktoken_id, &stakepool_locktoken);
    }

    fn internal_cancel_stakepool(&mut self, stakepool_id: &StakePoolId) {
        assert!(
            self.data()
                .stakepools
                .get(stakepool_id)
                .expect(ERR41_STAKEPOOL_NOT_EXIST)
                .can_be_cancelled(),
            "This stakepool can NOT be cancelled"
        );

        self.data_mut().stakepools.remove(stakepool_id).expect(ERR41_STAKEPOOL_NOT_EXIST);

        let (locktoken_id, _) = parse_stakepool_id(stakepool_id);
        let mut stakepool_locktoken = self.get_locktoken_wrapped(&locktoken_id).expect(ERR31_LOCKTOKEN_NOT_EXIST);
        stakepool_locktoken.get_ref_mut().stakepools.remove(stakepool_id);
        self.data_mut().locktokens.insert(&locktoken_id, &stakepool_locktoken);
    }
}
