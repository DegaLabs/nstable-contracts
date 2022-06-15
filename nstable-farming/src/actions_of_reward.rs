
use std::convert::TryInto;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, near_bindgen, AccountId, Balance, PromiseResult};

use crate::utils::{ext_fungible_token, ext_self, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_TRANSFER, parse_stakepool_id, U256};
use crate::errors::*;
use crate::*;


#[near_bindgen]
impl Contract {

    /// Clean invalid rps,
    /// return false if the rps is still valid.
    pub fn remove_user_rps_by_stakepool(&mut self, stakepool_id: StakePoolId) -> bool {
        let sender_id = env::predecessor_account_id();
        let mut staker = self.get_staker(&sender_id);
        let (locktoken_id, _) = parse_stakepool_id(&stakepool_id);
        let stakepool_locktoken = self.get_locktoken(&locktoken_id);
        if !stakepool_locktoken.get_ref().stakepools.contains(&stakepool_id) {
            staker.get_ref_mut().remove_rps(&stakepool_id);
            self.data_mut().stakers.insert(&sender_id, &staker);
            true
        } else {
            false
        }
    }

    pub fn claim_reward_by_stakepool(&mut self, stakepool_id: StakePoolId) {
        let sender_id = env::predecessor_account_id();
        self.internal_claim_user_reward_by_stakepool_id(&sender_id, &stakepool_id);
    }

    pub fn claim_reward_by_locktoken(&mut self, locktoken_id: LockTokenId) {
        let sender_id = env::predecessor_account_id();
        self.internal_claim_user_reward_by_locktoken_id(&sender_id, &locktoken_id);
    }

    /// Withdraws given reward token of given user.
    #[payable]
    pub fn withdraw_reward(&mut self, token_id: ValidAccountId, amount: Option<U128>) {
        assert_one_yocto();

        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.unwrap_or(U128(0)).into(); 

        let sender_id = env::predecessor_account_id();

        let mut staker = self.get_staker(&sender_id);

        // Note: subtraction, will be reverted if the promise fails.
        let amount = staker.get_ref_mut().sub_reward(&token_id, amount);
        self.data_mut().stakers.insert(&sender_id, &staker);
        ext_fungible_token::ft_transfer(
            sender_id.clone().try_into().unwrap(),
            amount.into(),
            None,
            &token_id,
            1,
            GAS_FOR_FT_TRANSFER,
        )
        .then(ext_self::callback_post_withdraw_reward(
            token_id,
            sender_id,
            amount.into(),
            &env::current_account_id(),
            0,
            GAS_FOR_RESOLVE_TRANSFER,
        ));
    }

    #[private]
    pub fn callback_post_withdraw_reward(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    ) -> U128 {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            ERR25_CALLBACK_POST_WITHDRAW_INVALID
        );
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                env::log(
                    format!(
                        "{} withdraw reward {} amount {}, Succeed.",
                        sender_id, token_id, amount.0,
                    )
                    .as_bytes(),
                );
                amount.into()
            }
            PromiseResult::Failed => {
                env::log(
                    format!(
                        "{} withdraw reward {} amount {}, Callback Failed.",
                        sender_id, token_id, amount.0,
                    )
                    .as_bytes(),
                );
                // This reverts the changes from withdraw function.
                let mut staker = self.get_staker(&sender_id);
                staker.get_ref_mut().add_reward(&token_id, amount.0);
                self.data_mut().stakers.insert(&sender_id, &staker);
                0.into()
            }
        }
    }
}

fn claim_user_reward_from_stakepool(
    stakepool: &mut StakePool, 
    staker: &mut Staker, 
    total_locktokens: &Balance,
    silent: bool,
) {
    let user_locktokens = staker.locktoken_powers.get(&stakepool.get_locktoken_id()).unwrap_or(&0_u128);
    let user_rps = staker.get_rps(&stakepool.get_stakepool_id());
    let (new_user_rps, reward_amount) = stakepool.claim_user_reward(&user_rps, user_locktokens, total_locktokens, silent);
    if !silent {
        env::log(
            format!(
                "user_rps@{} increased to {}",
                stakepool.get_stakepool_id(), U256::from_little_endian(&new_user_rps),
            )
            .as_bytes(),
        );
    }
        
    staker.set_rps(&stakepool.get_stakepool_id(), new_user_rps);
    if reward_amount > 0 {
        staker.add_reward(&stakepool.get_reward_token(), reward_amount);
        if !silent {
            env::log(
                format!(
                    "claimed {} {} as reward from {}",
                    reward_amount, stakepool.get_reward_token() , stakepool.get_stakepool_id(),
                )
                .as_bytes(),
            );
        }
    }
}

impl Contract {

    pub(crate) fn internal_claim_user_reward_by_locktoken_id(
        &mut self, 
        sender_id: &AccountId,
        locktoken_id: &LockTokenId) {
        let mut staker = self.get_staker(sender_id);
        if let Some(mut stakepool_locktoken) = self.get_locktoken_wrapped(locktoken_id) {
            let amount = stakepool_locktoken.get_ref().total_locktoken_power;
            for stakepool_id in &mut stakepool_locktoken.get_ref_mut().stakepools.iter() {
                let mut stakepool = self.data().stakepools.get(stakepool_id).unwrap();
                claim_user_reward_from_stakepool(
                    &mut stakepool, 
                    staker.get_ref_mut(),  
                    &amount,
                    true,
                );
                self.data_mut().stakepools.insert(stakepool_id, &stakepool);
            }
            self.data_mut().locktokens.insert(locktoken_id, &stakepool_locktoken);
            self.data_mut().stakers.insert(sender_id, &staker);
        }
    }

    pub(crate) fn internal_claim_user_reward_by_stakepool_id(
        &mut self, 
        sender_id: &AccountId, 
        stakepool_id: &StakePoolId) {
        let mut staker = self.get_staker(sender_id);

        let (locktoken_id, _) = parse_stakepool_id(stakepool_id);

        if let Some(stakepool_locktoken) = self.get_locktoken_wrapped(&locktoken_id) {
            let amount = stakepool_locktoken.get_ref().total_locktoken_power;
            if let Some(mut stakepool) = self.data().stakepools.get(stakepool_id) {
                claim_user_reward_from_stakepool(
                    &mut stakepool, 
                    staker.get_ref_mut(), 
                    &amount,
                    false,
                );
                self.data_mut().stakepools.insert(stakepool_id, &stakepool);
                self.data_mut().stakers.insert(sender_id, &staker);
            }
        }
    }


    #[inline]
    pub(crate) fn get_staker(&self, from: &AccountId) -> VersionedStaker {
        let orig = self.data().stakers
            .get(from)
            .expect(ERR10_ACC_NOT_REGISTERED);
        if orig.need_upgrade() {
                orig.upgrade()
            } else {
                orig
            }
    }

    #[inline]
    pub(crate) fn get_staker_default(&self, from: &AccountId) -> VersionedStaker {
        let orig = self.data().stakers.get(from).unwrap_or(VersionedStaker::new(from.clone()));
        if orig.need_upgrade() {
            orig.upgrade()
        } else {
            orig
        }
    }

    #[inline]
    pub(crate) fn get_staker_wrapped(&self, from: &AccountId) -> Option<VersionedStaker> {
        if let Some(staker) = self.data().stakers.get(from) {
            if staker.need_upgrade() {
                Some(staker.upgrade())
            } else {
                Some(staker)
            }
        } else {
            None
        }
    }

    /// Returns current balance of given token for given user. 
    /// If there is nothing recorded, returns 0.
    pub(crate) fn internal_get_reward(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
    ) -> Balance {
        self.get_staker_default(sender_id)
            .get_ref().rewards.get(token_id).cloned()
            .unwrap_or_default()
    }
}
