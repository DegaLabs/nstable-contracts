use crate::*;
use near_sdk::{near_bindgen, Promise, PromiseResult};

#[ext_contract(ext_nai_mint)]
pub trait NaiMinter {
    fn mint(&mut self, account_id: AccountId, amount: U128) -> Promise<U128>;
}

#[ext_contract(ext_self)]
pub trait MintCallback {
    fn mint_callback(&mut self, collateral_token_id: AccountId, collateral_amount: Balance, account: AccountId, borrowed: Balance) -> U128;
}

impl Contract {
    pub fn call_mint(&mut self, account: AccountId, amount: Balance) -> Promise {
        ext_nai_mint::mint(
            account.clone(),
            U128(amount),
            self.nai_token_id.clone(),
            NO_DEPOSIT,
            GAS_FOR_MINT,
        )
    }
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn mint_callback(&mut self, collateral_token_id: AccountId, collateral_amount: Balance, account: AccountId, borrowed: Balance) -> U128 {
        //update borrowed dai
        let actual_received = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(actual_received) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    actual_received
                } else {
                    U128(0)
                }
            }
            PromiseResult::Failed => U128(0),
        };

        self.finish_borrow(collateral_token_id.clone(), collateral_amount.clone(), account.clone(), borrowed.clone(), actual_received.0);

        actual_received
    }
}
