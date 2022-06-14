use crate::*;
use near_sdk::{ext_contract, AccountId, Balance, Gas, PromiseResult};

/// Amount of gas for fungible token transfers, increased to 20T to support AS token contracts.
pub const GAS_FOR_FT_TRANSFER: Gas = Gas(20_000_000_000_000);
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(20_000_000_000_000);

#[ext_contract(ext_self)]
pub trait NaiVault {
    fn callback_post_withdraw(&mut self, pool_id: u32, token_id: AccountId, receiver_id: AccountId, amount: U128);
}

#[ext_contract(ext_ft_core)]
pub trait FungibleTokenCore {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn callback_post_withdraw(
        &mut self,
        pool_id: u32,
        token_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            "CALLBACK_POST_WITHDRAW_INVALID"
        );
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {}
            PromiseResult::Failed => {
                // This reverts the changes from withdraw function.
                // If account doesn't exit, deposits to the owner's account as lostfound.
                self.internal_deposit(pool_id, &receiver_id, &token_id, amount.0)
            }
        };
    }
}

impl Contract {
    pub(crate) fn internal_send_tokens(
        &self,
        pool_id: u32,
        token_id: &AccountId,
        receiver_id: &AccountId,
        amount: Balance,
    ) -> Promise {
        ext_ft_core::ft_transfer(
            receiver_id.clone(),
            U128(amount),
            None,
            token_id.clone(),
            1,
            GAS_FOR_FT_TRANSFER,
        )
        .then(ext_self::callback_post_withdraw(
            pool_id,
            token_id.clone(),
            receiver_id.clone(),
            U128(amount),
            env::current_account_id(),
            0,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }

    pub fn get_storage_usage(&self, prev: StorageUsage) -> StorageUsage {
        if env::storage_usage() > prev {
            return env::storage_usage() - prev;
        }
        return 0;
    }
}

pub fn compute_token_value(amount: Balance, price: &Price) -> U256 {
    return U256::from(amount.clone()) * U256::from(price.multiplier.0)
        / U256::from(10u128.pow(price.decimals as u32));
}

pub fn compute_token_value_usd(amount: Balance, decimals: u8, price: &Price) -> U128 {
    let ret = U256::from(amount.clone()) * U256::from(price.multiplier.0)
        / U256::from(10u128.pow(decimals as u32));
    U128(ret.as_u128())
}

pub fn compute_cr(
    collateral_amount: Balance,
    collateral_decimals: u8,
    collateral_price: &Price,
    borrow_amount: Balance,
    borrow_price: &Price,
    borrow_decimals: u8,
) -> u64 {
    let collateral_value = compute_token_value(collateral_amount.clone(), collateral_price);
    let borrow_value = compute_token_value(borrow_amount.clone(), borrow_price);

    if borrow_value.as_u128() == 0 {
        return 10000000 * COLLATERAL_RATIO_DIVISOR as u64; //infinite number
    }

    let cr = U256::from(collateral_value)
        * U256::from(10u128.pow(borrow_decimals as u32))
        * U256::from(COLLATERAL_RATIO_DIVISOR)
        / (U256::from(borrow_value) * U256::from(10u128.pow(collateral_decimals as u32)));
    cr.as_u64()
}
