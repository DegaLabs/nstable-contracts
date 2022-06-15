use crate::errors::*;
use crate::stakepool_locktoken::LockTokenType;
use crate::utils::MFT_TAG;
use crate::*;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
enum TokenReceiverMessage {
    NewCDAccount { index: u32, cd_strategy: u32 },
    AppendCDAccount { index: u32 },
    Reward { stakepool_id: StakePoolId },
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// transfer reward token with specific msg indicate
    /// which stakepool to be deposited to.
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let sender: AccountId = sender_id.into();
        let amount: u128 = amount.into();
        if msg.is_empty() {
            // ****** locktoken Token deposit in ********
            self.internal_locktoken_deposit(
                &env::predecessor_account_id(),
                &sender,
                amount.into(),
                LockTokenType::FT,
            );
            PromiseOrValue::Value(U128(0))
        } else {
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR51_WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::NewCDAccount { index, cd_strategy } => {
                    let locktoken_id = env::predecessor_account_id();
                    self.internal_locktoken_deposit_to_new_cd_account(
                        &sender,
                        &locktoken_id,
                        index.into(),
                        cd_strategy as usize,
                        amount,
                        LockTokenType::FT,
                    );
                    PromiseOrValue::Value(U128(0))
                }
                TokenReceiverMessage::AppendCDAccount { index } => {
                    let locktoken_id = env::predecessor_account_id();
                    self.internal_locktoken_deposit_to_exist_cd_account(
                        &sender,
                        &locktoken_id,
                        index.into(),
                        amount,
                    );
                    PromiseOrValue::Value(U128(0))
                }
                TokenReceiverMessage::Reward { stakepool_id } => {
                    // ****** reward Token deposit in ********
                    let mut stakepool = self.data().stakepools.get(&stakepool_id).expect(ERR41_STAKEPOOL_NOT_EXIST);

                    // update stakepool
                    assert_eq!(
                        stakepool.get_reward_token(),
                        env::predecessor_account_id(),
                        "{}",
                        ERR44_INVALID_STAKEPOOL_REWARD
                    );
                    if let Some(cur_remain) = stakepool.add_reward(&amount) {
                        self.data_mut().stakepools.insert(&stakepool_id, &stakepool);
                        let old_balance = self
                            .data()
                            .reward_info
                            .get(&env::predecessor_account_id())
                            .unwrap_or(0);
                        self.data_mut()
                            .reward_info
                            .insert(&env::predecessor_account_id(), &(old_balance + amount));

                        env::log(
                            format!(
                                "{} added {} Reward Token, Now has {} left",
                                sender, amount, cur_remain
                            )
                            .as_bytes(),
                        );
                        PromiseOrValue::Value(U128(0))
                    } else {
                        env::panic(format!("{}", ERR43_INVALID_STAKEPOOL_STATUS).as_bytes())
                    }
                }
            }
        }
    }
}

pub trait MFTTokenReceiver {
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

enum TokenOrPool {
    Token(AccountId),
    Pool(u64),
}

/// a sub token would use a format ":<u64>"
fn try_identify_sub_token_id(token_id: &String) -> Result<u64, &'static str> {
    if token_id.starts_with(":") {
        if let Ok(pool_id) = str::parse::<u64>(&token_id[1..token_id.len()]) {
            Ok(pool_id)
        } else {
            Err("Illegal pool id")
        }
    } else {
        Err("Illegal pool id")
    }
}

fn parse_token_id(token_id: String) -> TokenOrPool {
    if let Ok(pool_id) = try_identify_sub_token_id(&token_id) {
        TokenOrPool::Pool(pool_id)
    } else {
        TokenOrPool::Token(token_id)
    }
}

/// locktoken token deposit
#[near_bindgen]
impl MFTTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let amount: u128 = amount.into();
        let locktoken_id: String;
        match parse_token_id(token_id.clone()) {
            TokenOrPool::Pool(pool_id) => {
                locktoken_id = format!("{}{}{}", env::predecessor_account_id(), MFT_TAG, pool_id);
            }
            TokenOrPool::Token(_) => {
                // for locktoken deposit, using mft to transfer 'root' token is not supported.
                env::panic(ERR35_ILLEGAL_TOKEN_ID.as_bytes());
            }
        }
        if msg.is_empty() {
            self.internal_locktoken_deposit(&locktoken_id, &sender_id, amount, LockTokenType::MFT);
            PromiseOrValue::Value(U128(0))
        } else {
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR51_WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::NewCDAccount { index, cd_strategy } => {
                    self.internal_locktoken_deposit_to_new_cd_account(
                        &sender_id,
                        &locktoken_id,
                        index.into(),
                        cd_strategy as usize,
                        amount,
                        LockTokenType::MFT,
                    );
                    PromiseOrValue::Value(U128(0))
                }
                TokenReceiverMessage::AppendCDAccount { index } => {
                    self.internal_locktoken_deposit_to_exist_cd_account(
                        &sender_id,
                        &locktoken_id,
                        index.into(),
                        amount,
                    );
                    PromiseOrValue::Value(U128(0))
                }
                _ => {
                    // ****** not support other msg format ********
                    env::panic(format!("{}", ERR52_MSG_NOT_SUPPORT).as_bytes())
                }
            }
        }
    }
}
