use crate::*;
use near_sdk::{ext_contract, log, AccountId, Balance, Gas, PromiseResult};

/// Amount of gas for fungible token transfers, increased to 20T to support AS token contracts.
pub const GAS_FOR_FT_TRANSFER: Gas = Gas(20_000_000_000_000);
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(20_000_000_000_000);

#[ext_contract(ext_self)]
pub trait NaiVault {
    fn callback_post_withdraw(&mut self, token_id: AccountId, receiver_id: AccountId, amount: U128);
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
                let mut failed = false;
                if self.accounts.contains_key(&receiver_id) {
                    let mut account_deposit = self.get_account_info(receiver_id.clone());
                    let vault_index = account_deposit.get_vault_index(token_id.clone());
                    if vault_index < account_deposit.vaults.len() {
                        // cause storage already checked, here can directly save
                        let mut vault = account_deposit.get_vault(token_id.clone());
                        vault.deposited = U128(vault.deposited.0 + amount.0);
                        account_deposit.vaults[vault_index] = vault;
                        self.accounts.insert(&receiver_id, &account_deposit);
                    } else {
                        // we can ensure that internal_get_account here would NOT cause a version upgrade,
                        // cause it is callback, the account must be the current version or non-exist,
                        // so, here we can just leave it without insert, won't cause storage collection inconsistency.
                        log!(format!(
                            "Account {} has not enough storage. Depositing to owner.",
                            receiver_id
                        ));
                        failed = true;
                    }
                } else {
                    log!(format!(
                        "Account {} is not registered. Depositing to owner.",
                        receiver_id
                    ));
                    failed = true;
                }
                if failed {
                    //lost fund
                }
            }
        };
    }
}

impl Contract {
    pub(crate) fn internal_send_tokens(
        &self,
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
            token_id.clone(),
            receiver_id.clone(),
            U128(amount),
            env::current_account_id(),
            0,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }

    pub fn compute_cr(&self, collateral_value: &U256, borrowed: &Balance, collateral_decimals: u8) -> u64 {
        let ret = collateral_value * U256::from(10u128.pow(18 as u32)) * U256::from(COLLATERAL_RATIO_DIVISOR as u64)
                / (U256::from(borrowed.clone()) * U256::from(10u128.pow(collateral_decimals as u32)));
        ret.as_u64()
    }

    pub fn compute_collateral_value(&self, collateral_amount: &Balance, price: &Price) -> U256 {
        return U256::from(collateral_amount.clone()) * U256::from(price.multiplier.0) * U256::from(10u128.pow(price.decimals as u32));
    }
}
