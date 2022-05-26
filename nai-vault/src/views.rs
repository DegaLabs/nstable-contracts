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
    collateral_value: U128
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
            let token_info = self.get_token_info(vault.token_id.clone());
            let price = self.price_data.price(&vault.token_id);
            let collateral_value = U256::from(vault.deposited.0) * U256::from(price.multiplier.0) / (10u128.pow(price.decimals as u32));
            let current_collateral_ratio = collateral_value * U256::from(10u128.pow(18 as u32)) * U256::from(10000 as u64) / (U256::from(vault.borrowed.0) * U256::from(10u128.pow(token_info.decimals as u32)));
            let b = BorrowInfo {
                owner_id: account_id.clone(),
                token_id: vault.token_id.clone(),
                deposited: vault.deposited,
                decimals: token_info.decimals,
                borrowed: vault.borrowed,
                last_deposit: vault.last_deposit,
                last_borrowed: vault.last_borrowed,
                current_collateral_ratio: current_collateral_ratio.as_u64(),
                collateral_token_price: price.multiplier,
                collateral_token_price_decimal: price.decimals,
                collateral_ratio: token_info.collateral_ratio,
                collateral_value: U128(collateral_value.as_u128())
            };
            ret.push(b);
        }
        ret
    }
}

impl Contract {
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
        let max_borrowable = borrowable * U256::from(100 as u64) / collateral_ratio;

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
