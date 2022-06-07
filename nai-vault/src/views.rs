use crate::oracle::Price;
use crate::*;
use near_sdk::{near_bindgen, AccountId};
uint::construct_uint!(
    pub struct U256(4);
);

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct BorrowInfo {
    owner_id: AccountId,
    token_id: AccountId,
    decimals: u8,
    deposited: U128,
    borrowed: U128,
    last_deposit: U128,
    last_borrowed: U128,
    current_collateral_ratio: u64,
    collateral_ratio: u64,
    collateral_token_price: U128,
    collateral_token_price_decimal: u8,
    collateral_value: U128,
    liquidation_price: Price,
    max_borrowable: U128,
    liquidation_fee: u64,
    dust_limit: U128
}

#[near_bindgen]
impl Contract {
    pub fn is_token_supported(&self, token_id: &AccountId) -> bool {
        self.supported_tokens.contains_key(token_id)
    }

    pub fn get_token_info(&self, token_id: AccountId) -> TokenInfo {
        if !self.is_token_supported(&token_id) {
            return TokenInfo::new(token_id.clone());
        }
        self.supported_tokens.get(&token_id).unwrap()
    }

    pub fn get_all_token_info(&self) -> Vec<TokenInfo> {
        let mut ret = Vec::with_capacity(self.token_list.len());
        for token_id in &self.token_list {
            ret.push(self.get_token_info(token_id.clone()));
        }
        ret
    }

    pub fn blacklist_status(&self, account_id: &AccountId) -> BlackListStatus {
        return match self.black_list.get(account_id) {
            Some(x) => x.clone(),
            None => BlackListStatus::Allowable,
        };
    }

    pub fn compute_borrowable_amount(
        &self,
        collateral_token_id: AccountId,
        collateral_amount: U128,
    ) -> U128 {
        let borrowable = self.internal_compute_max_borrowable_amount(
            collateral_token_id.clone(),
            collateral_amount.0,
        );
        U128(borrowable)
    }

    pub fn get_account_info(&self, account_id: AccountId) -> AccountDeposit {
        let acc = self.accounts.get(&account_id).unwrap_or_default();
        acc
    }

    pub fn get_account_vault(
        &self,
        account_id: AccountId,
        collateral_token_id: AccountId,
    ) -> Vault {
        let account_deposit = self.get_account_info(account_id.clone());
        account_deposit.get_vault(collateral_token_id.clone())
    }

    pub fn compute_storage_usage_near(&self, account_id: AccountId) -> U128 {
        let account_deposit = self.get_account_info(account_id.clone());
        U128(account_deposit.storage_usage as u128 * env::storage_byte_cost())
    }

    /// Returns how much NEAR is available for storage.
    pub fn storage_available(&self, account_id: AccountId) -> U128 {
        let account_deposit = self.get_account_info(account_id.clone());
        let locked = self.compute_storage_usage_near(account_id.clone());
        if account_deposit.near_amount.0 > locked.0 {
            U128(account_deposit.near_amount.0 - locked.0)
        } else {
            U128(0)
        }
    }

    pub fn get_token_count(&self) -> usize {
        self.token_list.len()
    }

    pub fn get_token_list(&self) -> &[AccountId] {
        &self.token_list
    }

    pub fn compute_max_borrowable(
        &self,
        collateral_token_id: AccountId,
        collateral_amount: U128,
    ) -> U128 {
        U128(self.internal_compute_max_borrowable_amount(
            collateral_token_id.clone(),
            collateral_amount.0,
        ))
    }

    pub fn get_current_borrow_info(&self, account_id: AccountId) -> Vec<BorrowInfo> {
        let deposit_account = self.get_account_info(account_id.clone());
        let mut ret = Vec::with_capacity(deposit_account.vaults.len());
        for vault in &deposit_account.vaults {
            if vault.deposited.0 == 0 {
                continue;
            }
            let token_info = self.get_token_info(vault.token_id.clone());
            let price = self.price_data.price(&vault.token_id);
            if price.multiplier.0 == 0 {
                continue;
            }
            let collateral_value = self.compute_collateral_value(&vault.deposited.0, &price);
            let mut current_collateral_ratio = 0 as u64;
            if vault.borrowed.0 > 0 {
                current_collateral_ratio = self.compute_cr(
                    &collateral_value,
                    &vault.borrowed.0,
                    token_info.decimals.clone(),
                );
            }
            let b = BorrowInfo {
                owner_id: account_id.clone(),
                token_id: vault.token_id.clone(),
                deposited: vault.deposited,
                decimals: token_info.decimals,
                borrowed: vault.borrowed,
                last_deposit: vault.last_deposit,
                last_borrowed: vault.last_borrowed,
                current_collateral_ratio: current_collateral_ratio,
                collateral_token_price: price.multiplier,
                collateral_token_price_decimal: price.decimals,
                collateral_ratio: token_info.collateral_ratio,
                collateral_value: U128(collateral_value.as_u128()),
                liquidation_price: self.compute_liquidation_price(
                    account_id.clone(),
                    vault.token_id.clone(),
                    None,
                    None,
                ),
                max_borrowable: self.compute_max_borrowable_for_account(
                    account_id.clone(),
                    vault.token_id.clone(),
                    U128(0),
                ),
                liquidation_fee: token_info.liquidation_price_fee,
                dust_limit: self.get_min_borrow()
            };
            ret.push(b);
        }
        ret
    }

    pub fn get_current_borrow_info_for_collateral(
        &self,
        account_id: AccountId,
        collateral_token_id: AccountId,
        collateral_amount: Option<U128>,
        borrow: Option<U128>,
    ) -> BorrowInfo {
        let deposit_account = self.get_account_info(account_id.clone());
        let token_info = self.get_token_info(collateral_token_id.clone());
        let price = self.price_data.price(&collateral_token_id);
        let vault = deposit_account.get_vault_or_default(account_id.clone(), collateral_token_id.clone());
        let collateral_amount = collateral_amount.unwrap_or(U128(0));
        let borrow = borrow.unwrap_or(U128(0));

        let collateral_value = self.compute_collateral_value(&(vault.deposited.0 + collateral_amount.0), &price);
        let mut current_collateral_ratio = 0 as u64;
        if vault.borrowed.0 + borrow.0 > 0 {
            current_collateral_ratio = self.compute_cr(
                &collateral_value,
                &(vault.borrowed.0 + borrow.0),
                token_info.decimals.clone(),
            );
        }
        let b = BorrowInfo {
            owner_id: account_id.clone(),
            token_id: vault.token_id.clone(),
            deposited: U128(vault.deposited.0),
            decimals: token_info.decimals,
            borrowed: U128(vault.borrowed.0),
            last_deposit: vault.last_deposit,
            last_borrowed: vault.last_borrowed,
            current_collateral_ratio: current_collateral_ratio,
            collateral_token_price: price.multiplier,
            collateral_token_price_decimal: price.decimals,
            collateral_ratio: token_info.collateral_ratio,
            collateral_value: U128(collateral_value.as_u128()),
            liquidation_price: self.compute_liquidation_price(
                account_id.clone(),
                vault.token_id.clone(),
                Some(collateral_amount),
                Some(borrow),
            ),
            max_borrowable: self.compute_max_borrowable_for_account(
                account_id.clone(),
                vault.token_id.clone(),
                collateral_amount,
            ),
            liquidation_fee: token_info.liquidation_price_fee,
            dust_limit: self.get_min_borrow()
        };

        b
    }

    pub fn compute_collateral_ratio(
        &self,
        collateral_token_id: AccountId,
        collateral_amount: U128,
        borrowed: U128,
    ) -> u64 {
        self.internal_compute_collateral_ratio(
            &collateral_token_id,
            collateral_amount.0,
            borrowed.0,
        )
    }

    pub fn compute_max_borrowable_for_account(
        &self,
        account_id: AccountId,
        collateral_token_id: AccountId,
        collateral_amount: U128,
    ) -> U128 {
        let account_deposit = self.get_account_info(account_id.clone());
        let vault =
            account_deposit.get_vault_or_default(account_id.clone(), collateral_token_id.clone());
        let new_collateral_amount = vault.deposited.0 + collateral_amount.0;
        let max =
            self.internal_compute_max_borrowable_amount(collateral_token_id, new_collateral_amount);
        if max > vault.borrowed.0 {
            return U128(max.clone() - vault.borrowed.0);
        }
        U128(0)
    }

    pub fn compute_max_withdrawal(
        &self,
        account_id: AccountId,
        collateral_token_id: AccountId,
    ) -> U128 {
        let account_deposit = self.get_account_info(account_id.clone());
        let vault =
            account_deposit.get_vault_or_default(account_id.clone(), collateral_token_id.clone());

        if vault.borrowed.0 == 0 {
            return vault.deposited;
        }

        let price = self.price_data.price(&collateral_token_id);
        let token_info = self.get_token_info(collateral_token_id.clone());
        let min_collateral_ratio = token_info.collateral_ratio;

        let required_collateral_value = (U256::from(vault.borrowed.0)
            * U256::from(10u128.pow(token_info.decimals as u32)))
            * U256::from(min_collateral_ratio)
            / (U256::from(10u128.pow(18 as u32)) * U256::from(COLLATERAL_RATIO_DIVISOR as u64));
        let required_deposited = required_collateral_value
            * U256::from(10u128.pow(price.decimals as u32))
            / U256::from(price.multiplier.0);
        if required_deposited.as_u128() < vault.deposited.0 * 99 / 100 {
            return U128(vault.deposited.0 - required_deposited.as_u128());
        }
        U128(0)
    }

    pub fn compute_liquidation_price(
        &self,
        account_id: AccountId,
        collateral_token_id: AccountId,
        collateral_amount: Option<U128>,
        borrow_amount: Option<U128>,
    ) -> Price {
        let price = self.price_data.price(&collateral_token_id);
        let token_info = self.get_token_info(collateral_token_id.clone());
        let collateral_amount = collateral_amount.unwrap_or(U128(0));
        let borrow_amount = borrow_amount.unwrap_or(U128(0));

        let account_deposit = self.get_account_info(account_id.clone());
        let vault =
            account_deposit.get_vault_or_default(account_id.clone(), collateral_token_id.clone());
        let total_deposit = collateral_amount.0 + vault.deposited.0;
        let total_borrow = borrow_amount.0 + vault.borrowed.0;
        let total_borrow_value = U256::from(total_borrow);
        let min_required_collateral_value =
            total_borrow_value * token_info.collateral_ratio / COLLATERAL_RATIO_DIVISOR;

        let liquidation_price = min_required_collateral_value
            * U256::from(10u128.pow(price.decimals as u32))
            * U256::from(10u128.pow(token_info.decimals as u32))
            / (U256::from(total_deposit) * U256::from(10u128.pow(18 as u32)));
        Price {
            multiplier: U128(liquidation_price.as_u128()),
            decimals: price.decimals,
        }
    }

    pub fn compute_new_ratio_after_borrow(
        &self,
        account_id: AccountId,
        collateral_token_id: AccountId,
        collateral_amount: U128,
        borrow_amount: U128,
    ) -> (u64, u64) {
        let account_deposit = self.get_account_info(account_id.clone());
        let vault =
            account_deposit.get_vault_or_default(account_id.clone(), collateral_token_id.clone());
        let new_deposit = vault.deposited.0 + collateral_amount.0;
        let new_borrow = vault.borrowed.0 + borrow_amount.0;
        let token_info = self.get_token_info(vault.token_id.clone());
        if new_borrow == 0 {
            return (100000000, token_info.collateral_ratio);
        }

        let price = self.price_data.price(&vault.token_id);
        let collateral_value = self.compute_collateral_value(&new_deposit, &price);
        let new_collateral_ratio = collateral_value
            * U256::from(10u128.pow(18 as u32))
            * U256::from(COLLATERAL_RATIO_DIVISOR)
            / (U256::from(new_borrow.clone()) * U256::from(10u128.pow(token_info.decimals as u32)));
        (new_collateral_ratio.as_u64(), token_info.collateral_ratio)
    }

    pub fn get_min_borrow(&self) -> U128 {
        return U128(100 * 10u128.pow(18));
    }
}

impl Contract {
    pub fn internal_compute_collateral_ratio(
        &self,
        collateral_token_id: &AccountId,
        collateral_amount: Balance,
        borrowed: Balance,
    ) -> u64 {
        let token_info = self.get_token_info(collateral_token_id.clone());
        let price = self.price_data.price(&collateral_token_id);
        let collateral_value = self.compute_collateral_value(&collateral_amount, &price);
        let current_collateral_ratio =
            self.compute_cr(&collateral_value, &borrowed, token_info.decimals.clone());
        current_collateral_ratio
    }

    pub fn internal_compute_max_borrowable_amount(
        &self,
        collateral_token_id: AccountId,
        collateral_amount: Balance,
    ) -> Balance {
        if !self.is_token_supported(&collateral_token_id) {
            return 0;
        }
        let price_data = self.price_data.price(&collateral_token_id);

        let price = U256::from(price_data.multiplier.0);
        let decimals = price_data.decimals;
        let token_info = self.get_token_info(collateral_token_id.clone());
        let token_decimals = token_info.decimals;
        let collateral_ratio = token_info.collateral_ratio;
        let collateral_amount_u256 = U256::from(collateral_amount);

        //price decimals is less than 18
        let borrowable = (collateral_amount_u256 * price) * U256::from(10u128.pow(18))
            / (U256::from(10u128.pow(decimals as u32))
                * U256::from(10u128.pow(token_decimals as u32)));
        let max_borrowable =
            borrowable * U256::from(COLLATERAL_RATIO_DIVISOR as u64) / collateral_ratio;

        max_borrowable.as_u128()
    }

    pub fn storage_cost(&self, prev_storage: StorageUsage) -> Balance {
        let storage_cost = env::storage_usage()
            .checked_sub(prev_storage)
            .unwrap_or_default() as Balance
            * env::storage_byte_cost();
        storage_cost
    }
}
