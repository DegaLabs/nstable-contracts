use near_sdk::json_types::U128;
use near_sdk::{AccountId, Balance, Promise};
use std::convert::TryInto;

use crate::errors::*;
use crate::stakepool_locktoken::LockTokenType;
use crate::utils::{
    assert_one_yocto, ext_fungible_token, ext_multi_fungible_token, ext_self, parse_locktoken_id,
    wrap_mft_token_id, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_WITHDRAW_LOCKTOKEN, MAX_CDACCOUNT_NUM,
};
use crate::*;

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn withdraw_locktoken(&mut self, locktoken_id: LockTokenId, amount: U128) -> Promise {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();

        let amount: Balance = amount.into();

        // update inner state
        let locktoken_type = self.internal_locktoken_withdraw(&locktoken_id, &sender_id, amount);

        match locktoken_type {
            LockTokenType::FT => {
                ext_fungible_token::ft_transfer(
                    sender_id.clone().try_into().unwrap(),
                    amount.into(),
                    None,
                    &locktoken_id,
                    1, // one yocto near
                    GAS_FOR_FT_TRANSFER,
                )
                .then(ext_self::callback_withdraw_locktoken(
                    locktoken_id,
                    sender_id,
                    amount.into(),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_RESOLVE_WITHDRAW_LOCKTOKEN,
                ))
            }
            LockTokenType::MFT => {
                let (receiver_id, token_id) = parse_locktoken_id(&locktoken_id);
                ext_multi_fungible_token::mft_transfer(
                    wrap_mft_token_id(&token_id),
                    sender_id.clone().try_into().unwrap(),
                    amount.into(),
                    None,
                    &receiver_id,
                    1, // one yocto near
                    GAS_FOR_FT_TRANSFER,
                )
                .then(ext_self::callback_withdraw_locktoken(
                    locktoken_id,
                    sender_id,
                    amount.into(),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_RESOLVE_WITHDRAW_LOCKTOKEN,
                ))
            }
        }
    }

    #[payable]
    pub fn withdraw_locktoken_from_cd_account(&mut self, index: u64, amount: U128) -> Promise {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        // update inner state
        let (locktoken_id, amount) =
            self.internal_locktoken_withdraw_from_cd_account(&sender_id, index, amount.0);
        let (receiver_id, token_id) = parse_locktoken_id(&locktoken_id);
        if receiver_id == token_id {
            ext_fungible_token::ft_transfer(
                sender_id.clone().try_into().unwrap(),
                amount.into(),
                None,
                &locktoken_id,
                1, // one yocto near
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::callback_withdraw_locktoken(
                locktoken_id.clone(),
                sender_id,
                amount.into(),
                &env::current_account_id(),
                0,
                GAS_FOR_RESOLVE_WITHDRAW_LOCKTOKEN,
            ))
        } else {
            ext_multi_fungible_token::mft_transfer(
                wrap_mft_token_id(&token_id),
                sender_id.clone().try_into().unwrap(),
                amount.into(),
                None,
                &receiver_id,
                1, // one yocto near
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::callback_withdraw_locktoken(
                locktoken_id.clone(),
                sender_id,
                amount.into(),
                &env::current_account_id(),
                0,
                GAS_FOR_RESOLVE_WITHDRAW_LOCKTOKEN,
            ))
        }
    }
}

/// Internal methods implementation.
impl Contract {
    #[inline]
    pub(crate) fn get_locktoken(&self, locktoken_id: &String) -> VersionedStakePoolLockToken {
        let orig = self
            .data()
            .locktokens
            .get(locktoken_id)
            .expect(&format!("{}", ERR31_LOCKTOKEN_NOT_EXIST));
        if orig.need_upgrade() {
            orig.upgrade()
        } else {
            orig
        }
    }

    #[inline]
    pub(crate) fn get_locktoken_wrapped(&self, locktoken_id: &String) -> Option<VersionedStakePoolLockToken> {
        if let Some(stakepool_locktoken) = self.data().locktokens.get(locktoken_id) {
            if stakepool_locktoken.need_upgrade() {
                Some(stakepool_locktoken.upgrade())
            } else {
                Some(stakepool_locktoken)
            }
        } else {
            None
        }
    }

    pub(crate) fn internal_locktoken_deposit(
        &mut self,
        locktoken_id: &String,
        sender_id: &AccountId,
        locktoken_amount: Balance,
        locktoken_type: LockTokenType,
    ) {
        let mut stakepool_locktoken = self.get_locktoken(&locktoken_id);
        if locktoken_amount < stakepool_locktoken.get_ref().min_deposit {
            env::panic(
                format!(
                    "{} {}",
                    ERR34_BELOW_MIN_LOCKTOKEN_DEPOSITED,
                    stakepool_locktoken.get_ref().min_deposit
                )
                .as_bytes(),
            )
        }
        // 1. claim all reward of the user for this locktoken stakepools
        //    to update user reward_per_locktoken in each stakepool
        self.internal_claim_user_reward_by_locktoken_id(sender_id, locktoken_id);

        // 2. update staker locktoken_power
        let mut staker = self.get_staker(sender_id);
        staker.get_ref_mut().add_locktoken_amount(&locktoken_id, locktoken_amount);
        staker.get_ref_mut().add_locktoken_power(&locktoken_id, locktoken_amount);
        self.data_mut().stakers.insert(sender_id, &staker);

        // 3. update locktoken
        stakepool_locktoken.get_ref_mut().locktoken_type = locktoken_type;
        stakepool_locktoken.get_ref_mut().add_locktoken_amount(locktoken_amount);
        stakepool_locktoken.get_ref_mut().add_locktoken_power(locktoken_amount);
        self.data_mut().locktokens.insert(&locktoken_id, &stakepool_locktoken);

        // 4. output log/event
        env::log(
            format!(
                "{} deposit locktoken {} with amount {}.",
                sender_id, locktoken_id, locktoken_amount,
            )
            .as_bytes(),
        );
    }

    pub(crate) fn internal_locktoken_deposit_to_new_cd_account(
        &mut self,
        sender: &AccountId,
        locktoken_id: &LockTokenId,
        index: u64,
        cd_strategy: usize,
        amount: Balance,
        locktoken_type: LockTokenType,
    ) {
        let mut stakepool_locktoken = self.get_locktoken(locktoken_id);
        if amount < stakepool_locktoken.get_ref().min_deposit {
            env::panic(
                format!(
                    "{} {}",
                    ERR34_BELOW_MIN_LOCKTOKEN_DEPOSITED,
                    stakepool_locktoken.get_ref().min_deposit
                )
                .as_bytes(),
            )
        }
        // 1. claim all reward of the user for this locktoken stakepools
        //    to update user reward_per_locktoken in each stakepool
        self.internal_claim_user_reward_by_locktoken_id(sender, locktoken_id);

        // 2. update CD Account and staker locktoken_power
        let mut staker = self.get_staker(sender);
        assert!(
            index < MAX_CDACCOUNT_NUM,
            "{}",
            ERR63_INVALID_CD_ACCOUNT_INDEX
        );
        assert!(
            index <= staker.get_ref().cd_accounts.len(),
            "{}",
            ERR63_INVALID_CD_ACCOUNT_INDEX
        );
        assert!(
            cd_strategy < STRATEGY_LIMIT,
            "{}",
            ERR62_INVALID_CD_STRATEGY_INDEX
        );
        let strategy = &self.data().cd_strategy.stake_strategy[cd_strategy];
        assert!(strategy.enable, "{}", ERR62_INVALID_CD_STRATEGY_INDEX);
        let mut cd_account = staker.get_ref().cd_accounts.get(index).unwrap_or_default();
        let locktoken_power = cd_account.occupy(
            &locktoken_id,
            amount,
            strategy.power_reward_rate,
            strategy.lock_sec,
        );
        if index < staker.get_ref().cd_accounts.len() {
            staker.get_ref_mut().cd_accounts.replace(index, &cd_account);
        } else {
            staker.get_ref_mut().cd_accounts.push(&cd_account);
        }
        staker.get_ref_mut().add_locktoken_power(locktoken_id, locktoken_power);
        self.data_mut().stakers.insert(sender, &staker);

        // 3. update locktoken
        stakepool_locktoken.get_ref_mut().locktoken_type = locktoken_type;
        stakepool_locktoken.get_ref_mut().add_locktoken_amount(amount);
        stakepool_locktoken.get_ref_mut().add_locktoken_power(locktoken_power);
        self.data_mut().locktokens.insert(locktoken_id, &stakepool_locktoken);

        // 4. output log/event
        env::log(
            format!(
                "{} create CD account with locktoken amount {}, locktoken power {}",
                sender, amount, locktoken_power
            )
            .as_bytes(),
        );
    }

    pub(crate) fn internal_locktoken_deposit_to_exist_cd_account(
        &mut self,
        sender: &AccountId,
        locktoken_id: &LockTokenId,
        index: u64,
        amount: Balance,
    ) {
        let mut stakepool_locktoken = self.get_locktoken(&locktoken_id);
        if amount < stakepool_locktoken.get_ref().min_deposit {
            env::panic(
                format!(
                    "{} {}",
                    ERR34_BELOW_MIN_LOCKTOKEN_DEPOSITED,
                    stakepool_locktoken.get_ref().min_deposit
                )
                .as_bytes(),
            )
        }
        // 1. claim all reward of the user for this locktoken stakepools
        //    to update user reward_per_locktoken in each stakepool
        self.internal_claim_user_reward_by_locktoken_id(sender, locktoken_id);

        // 2. update CD Account and staker locktoken_power
        let mut staker = self.get_staker(sender);
        assert!(
            index < staker.get_ref().cd_accounts.len(),
            "{}",
            ERR63_INVALID_CD_ACCOUNT_INDEX
        );
        let mut cd_account = staker.get_ref().cd_accounts.get(index).unwrap();
        let power_added = cd_account.append(locktoken_id, amount);
        staker.get_ref_mut().cd_accounts.replace(index, &cd_account);
        staker.get_ref_mut().add_locktoken_power(locktoken_id, power_added);
        self.data_mut().stakers.insert(sender, &staker);

        // 3. update locktoken
        stakepool_locktoken.get_ref_mut().add_locktoken_amount(amount);
        stakepool_locktoken.get_ref_mut().add_locktoken_power(power_added);
        self.data_mut().locktokens.insert(locktoken_id, &stakepool_locktoken);

        // 4. output log/event
        env::log(
            format!(
                "{} append CD account {} with locktoken amount {}, locktoken power {}",
                sender, index, amount, power_added
            )
            .as_bytes(),
        );
    }

    fn internal_locktoken_withdraw(
        &mut self,
        locktoken_id: &LockTokenId,
        sender_id: &AccountId,
        amount: Balance,
    ) -> LockTokenType {
        // first claim all reward of the user for this locktoken stakepools
        // to update user reward_per_locktoken in each stakepool
        self.internal_claim_user_reward_by_locktoken_id(sender_id, locktoken_id);

        let mut stakepool_locktoken = self.get_locktoken(locktoken_id);
        let mut staker = self.get_staker(sender_id);

        // Then update user locktoken and total locktoken of this LPT
        let _staker_locktoken_amount_remain = staker.get_ref_mut().sub_locktoken_amount(locktoken_id, amount);
        let staker_locktoken_power_remain = staker.get_ref_mut().sub_locktoken_power(locktoken_id, amount);
        let _locktoken_amount_remain = stakepool_locktoken.get_ref_mut().sub_locktoken_amount(amount);
        let _locktoken_power_remain = stakepool_locktoken.get_ref_mut().sub_locktoken_power(amount);

        if staker_locktoken_power_remain == 0 {
            // remove staker rps of relative stakepool
            for stakepool_id in stakepool_locktoken.get_ref().stakepools.iter() {
                staker.get_ref_mut().remove_rps(stakepool_id);
            }
        }
        self.data_mut().stakers.insert(sender_id, &staker);
        self.data_mut().locktokens.insert(locktoken_id, &stakepool_locktoken);
        stakepool_locktoken.get_ref().locktoken_type.clone()
    }

    fn internal_locktoken_withdraw_from_cd_account(
        &mut self,
        sender_id: &AccountId,
        index: u64,
        amount: Balance,
    ) -> (LockTokenId, Balance) {
        let staker = self.get_staker(sender_id);
        assert!(
            index < staker.get_ref().cd_accounts.len(),
            "{}",
            ERR63_INVALID_CD_ACCOUNT_INDEX
        );
        let locktoken_id = &staker.get_ref().cd_accounts.get(index).unwrap().locktoken_id;
        // 1. claim all reward of the user for this locktoken stakepools
        //    to update user reward_per_locktoken in each stakepool
        self.internal_claim_user_reward_by_locktoken_id(sender_id, locktoken_id);

        // 2. remove locktoken from cd account
        let mut staker = self.get_staker(sender_id);
        let mut cd_account = staker.get_ref().cd_accounts.get(index).unwrap();
        let mut stakepool_locktoken = self.get_locktoken(locktoken_id);

        let (power_removed, locktoken_slashed) =
            cd_account.remove(locktoken_id, amount, stakepool_locktoken.get_ref().slash_rate);

        // 3. update user locktoken and total locktoken of this LPT
        let staker_locktoken_power_remain = staker.get_ref_mut().sub_locktoken_power(locktoken_id, power_removed);
        let _ = stakepool_locktoken.get_ref_mut().sub_locktoken_amount(amount);
        let _ = stakepool_locktoken.get_ref_mut().sub_locktoken_power(power_removed);

        // 4. collect locktoken_slashed
        if locktoken_slashed > 0 {
            env::log(
                format!(
                    "{} got slashed of {} locktoken with amount {}.",
                    sender_id, locktoken_id, locktoken_slashed,
                )
                .as_bytes(),
            );
            // all locktoken amount go to locktoken_slashed
            let locktoken_amount = self.data().locktokens_slashed.get(&locktoken_id).unwrap_or(0);
            self.data_mut()
                .locktokens_slashed
                .insert(&locktoken_id, &(locktoken_amount + locktoken_slashed));
        }

        // 5. remove user_rps if needed
        if staker_locktoken_power_remain == 0 {
            // remove staker rps of relative stakepool
            for stakepool_id in stakepool_locktoken.get_ref().stakepools.iter() {
                staker.get_ref_mut().remove_rps(stakepool_id);
            }
        }

        // 6. save back to storage
        staker.get_ref_mut().cd_accounts.replace(index, &cd_account);
        self.data_mut().stakers.insert(sender_id, &staker);
        self.data_mut().locktokens.insert(locktoken_id, &stakepool_locktoken);

        (locktoken_id.clone(), amount - locktoken_slashed)
    }
}
