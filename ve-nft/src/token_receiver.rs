use crate::*;
use near_sdk::json_types::U128;
use near_sdk::{serde_json, PromiseOrValue};

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// transfer reward token with specific msg indicate
    /// which farm to be deposited to.
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_id = env::predecessor_account_id();
        assert_eq!(token_id, self.locked_token, "invalid token");
        let sender: AccountId = sender_id.into();
        let amount: u128 = amount.into();

        self.internal_deposit_token(sender.clone(), amount.into());
        PromiseOrValue::Value(U128(0))
    }
}
