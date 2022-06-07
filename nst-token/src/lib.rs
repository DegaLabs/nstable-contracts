/*!
Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{
    env, log, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    PromiseOrValue,
};
use near_sdk::json_types::ValidAccountId;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    FungibleToken,
    Metadata,
}

const DATA_IMAGE_SVG_ICON: &str =
    "data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iVVRGLTgiPz4KPHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCAzMiAzMiIgdmVyc2lvbj0iMS4xIiB2aWV3Qm94PSIwIDAgMzIgMzIiIHhtbDpzcGFjZT0icHJlc2VydmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+CjxzdHlsZSB0eXBlPSJ0ZXh0L2NzcyI+Cgkuc3Q2e2ZpbGw6I0ZGRkZGRjt9Cgkuc3Q1OXtmaWxsOiM4NjQ3OUY7fQo8L3N0eWxlPgo8Y2lyY2xlIGNsYXNzPSJzdDU5IiBjeD0iMTYiIGN5PSIxNiIgcj0iMTYiLz4KPHBhdGggY2xhc3M9InN0NiIgZD0ibTIzLjU1IDguNzNjLTAuMDQtMC4wMi0wLjA3LTAuMDQtMC4xMS0wLjA2bC0wLjE2LTAuMDl2MC4wMWMtMS4zOC0wLjY2LTMuMDMtMC4zMi00LjA0IDAuODQtMC4yMiAwLjI1LTAuNDEgMC41Mi0wLjU3IDAuODFsLTAuMDEgMC4wMi0wLjA4IDAuMTQtMC4wMSAwLjAyLTAuODEgMS40OC01LjMxLTMuMDVjLTEuMTYtMC42Ny0yLjU0LTAuNjctMy43LTAuMDMtMS4yMiAwLjY4LTEuOTQgMi0xLjk0IDMuMzl2OC4yYzAgMS4xNyAwLjYzIDIuMjYgMS42NCAyLjg2IDAuMDMgMC4wMiAwLjA3IDAuMDQgMC4xIDAuMDZsMC4xNyAwLjA5YzAuNDcgMC4yMiAwLjk3IDAuMzMgMS40NyAwLjMzIDAuOTYgMCAxLjkxLTAuNDEgMi41Ny0xLjE4IDAuMjItMC4yNSAwLjQxLTAuNTIgMC41Ny0wLjgxbDAuMDEtMC4wMiAwLjA4LTAuMTQgMC4wMS0wLjAxIDAuODEtMS40OCA1LjMxIDMuMDVjMC41OSAwLjM0IDEuMjMgMC41MSAxLjg4IDAuNTEgMC42MiAwIDEuMjUtMC4xNiAxLjgyLTAuNDcgMS4yMi0wLjY4IDEuOTUtMS45OSAxLjk1LTMuMzl2LTguMmMtMC4wMS0xLjE5LTAuNjQtMi4yOS0xLjY1LTIuODh6bS0xMC43NyAxMi4yNi0wLjAzIDAuMDYtMC40NSAwLjgzYy0wLjM0IDAuNjItMC45NyAwLjk5LTEuNjcgMWgtMC4wMWMtMC43MSAwLTEuMzMtMC4zNy0xLjY3LTAuOTgtMC4xLTAuMTgtMC4xNy0wLjM3LTAuMjItMC41OC0wLjA0LTAuMTctMC4wNi0wLjM0LTAuMDYtMC41MXYtMC43OGMwLTAuMTYgMC4wMi0wLjMyIDAuMDYtMC40OSAwLjA4LTAuMzQgMC4yNS0wLjY1IDAuNDctMC45IDAuNDktMC41NSAxLjMyLTAuNjggMS45Ni0wLjMxbDIuMzUgMS4zNS0wLjczIDEuMzF6bS0wLjkyLTQuMzFjLTEuMDEtMC41Ny0yLjI1LTAuNDYtMy4xNCAwLjI3di01LjE0YzAtMC42MSAwLjMyLTEuMTYgMC44NC0xLjQ3IDAuNTMtMC4zMSAxLjE2LTAuMzEgMS43LTAuMDJsNS42NCAzLjE3LTIuNTEgNC42MS0yLjUzLTEuNDJ6bTExLjQyIDMuNWMwIDAuNjEtMC4zMiAxLjE2LTAuODQgMS40Ny0wLjUzIDAuMzEtMS4xNiAwLjMxLTEuNyAwLjAybC01LjY0LTMuMTcgMi41MS00LjYxIDIuNTMgMS40MmMwLjQzIDAuMjQgMC45IDAuMzYgMS4zOCAwLjM2IDAuNjMgMCAxLjI1LTAuMjIgMS43Ny0wLjYzdjUuMTR6bTAuMDYtOC4yMWMwIDAuMTYtMC4wMiAwLjMyLTAuMDYgMC40OS0wLjA4IDAuMzQtMC4yNSAwLjY1LTAuNDcgMC45LTAuNDkgMC41NS0xLjMyIDAuNjgtMS45NiAwLjMxbC0yLjM1LTEuMzUgMS4yLTIuMjFjMC4zNC0wLjYyIDAuOTctMC45OSAxLjY3LTFoMC4wMWMwLjcxIDAgMS4zMyAwLjM3IDEuNjcgMC45OCAwLjEgMC4xOCAwLjE3IDAuMzcgMC4yMiAwLjU4IDAuMDQgMC4xNyAwLjA2IDAuMzQgMC4wNiAwLjUxdjAuNzl6Ii8+Cjwvc3ZnPgo=";

const TOKEN_NAME: &str = "nStable Governance";

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(owner_id: AccountId, icon: Option<String>) -> Self {
        let metadata = FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: TOKEN_NAME.to_string(),
            symbol: "NST".to_string(),
            decimals: 6,
            icon: icon,
            reference: None,
            reference_hash: None
        };
        assert_eq!(env::state_exists(), false, "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        };
        let total_supply = U128(100_000_000_000_000);
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, total_supply.into());
        this
    }
    
    pub fn set_icon(&mut self) {
        let mut metadata = self.ft_metadata();
        metadata.icon = Some(DATA_IMAGE_SVG_ICON.to_string());
        self.metadata.set(&metadata);
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        let mut m = self.metadata.get().unwrap();
        m.name = TOKEN_NAME.to_string();
        m
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance};

    use super::*;

    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_default_meta(accounts(1).into(), TOTAL_SUPPLY.into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(2).into(), TOTAL_SUPPLY.into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
