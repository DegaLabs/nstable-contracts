use crate::*;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
pub type ActionName = String;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    DepositFor { account_id: AccountId },
    IncreaseAmountAndUnlockTime {days: u64, action_name: ActionName }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// transfer reward token with specific msg indicate
    /// which farm to be deposited to.
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_id = env::predecessor_account_id();
        assert_eq!(token_id, self.locked_token, "invalid token");
        let sender: AccountId = sender_id.into();
        let amount: u128 = amount.into();
        if msg.is_empty() {
            // ****** increase amount
            self.increase_amount(sender.clone(), amount.into());
            PromiseOrValue::Value(U128(0))
        } else {
            let message = serde_json::from_str::<TokenReceiverMessage>(&msg)
                .expect(&format!("wrong message format {}", msg));
            match message {
                TokenReceiverMessage::DepositFor { account_id } => {
                    self.deposit_for(account_id, amount.into());
                    PromiseOrValue::Value(U128(0))
                },
                TokenReceiverMessage::IncreaseAmountAndUnlockTime {days, action_name } => {
                    if action_name == "IncreaseAmountAndUnlockTime" {
                        self.increase_amount_and_unlock_time(sender.clone(), amount, days.into());
                        PromiseOrValue::Value(U128(0))
                    } else if action_name == "CreateLock" {
                        self.create_lock(sender.clone(), amount, days.into());
                        PromiseOrValue::Value(U128(0))
                    } else {
                        env::panic("wrong action name".as_bytes())
                    }
                },
            }
        }
    }
}
