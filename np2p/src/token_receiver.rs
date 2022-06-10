use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};

use crate::*;
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    /// Alternative to deposit + execute actions call.
    Deposit {
        pool_id: u32
    },
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// `msg` format is either "" for deposit or `TokenReceiverMessage`.
    #[allow(unreachable_code)]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.abort_if_pause();
        self.abort_if_blacklisted(sender_id.clone());
        let token_in = env::predecessor_account_id();

        self.abort_if_unsupported_token(token_in.clone());
        if msg.is_empty() {
            env::panic_str("unsupported operation");
        } else {
            // instant swap
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect("wrong message format");
            match message {
                TokenReceiverMessage::Deposit {
                    pool_id
                } => {
                    self.internal_deposit(pool_id, &sender_id, &token_in, amount.0);
                    PromiseOrValue::Value(U128(0))
                }
            }
        }
    }
}
