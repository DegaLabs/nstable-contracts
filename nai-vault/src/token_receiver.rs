use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{PromiseOrValue};

use crate::*;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    /// Alternative to deposit + execute actions call.
    Borrow {
        borrow_amount: U128,
        /// List of sequential actions.
        receiver_id: Option<AccountId>,
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
            // Simple deposit.
            self.deposit_to_vault(&token_in, &amount.0, &sender_id);
            PromiseOrValue::Value(U128(0))
        } else {
            env::panic_str("unsupported operation");
        }
    }
}
