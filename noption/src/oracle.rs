use near_sdk::Timestamp;

use crate::*;

type DurationSec = u32;

// From https://github.com/NearDeFi/price-oracle/blob/main/src/utils.rs
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Copy)]
#[serde(crate = "near_sdk::serde")]
pub struct Price {
    pub multiplier: U128,   //price
    pub decimals: u8,   //price decimals
}

impl Default for Price {
    fn default() -> Price {
        Price {
            multiplier: U128(0),
            decimals: 0
        }
    }
}

// From https://github.com/NearDeFi/price-oracle/blob/main/src/asset.rs
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetOptionalPrice {
    pub asset_id: AccountId,
    pub price: Option<Price>,
}

// From https://github.com/NearDeFi/price-oracle/blob/main/src/lib.rs
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PriceData {
    timestamp: U64,
    recency_duration_sec: DurationSec,
    prices: Vec<AssetOptionalPrice>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExchangeRate {
    multiplier: u128,
    decimals: u8,
    timestamp: Timestamp,
    recency_duration: Timestamp,
}

impl ExchangeRate {
    pub fn multiplier(&self) -> u128 {
        self.multiplier
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Oracle {
    pub last_report: Option<ExchangeRate>,
}

impl Default for Oracle {
    fn default() -> Self {
        Self { last_report: None }
    }
}

impl Contract {
    pub fn get_exchange_rate(&self, asset_id: &AccountId) -> ExchangeRate {
        let price_data = self.get_price_data();

        let price = price_data.price(&asset_id);

        if env::block_timestamp() >= price_data.timestamp() + price_data.recency_duration() {
            env::panic_str("Oracle provided an outdated price data");
        }

        let exchange_rate = ExchangeRate {
            multiplier: price.multiplier.into(),
            decimals: price.decimals,
            timestamp: price_data.timestamp(),
            recency_duration: price_data.recency_duration(),
        };

        exchange_rate
    }
}

impl Default for PriceData {
    fn default() -> Self {
        PriceData {
            timestamp: U64(0),
            recency_duration_sec: 0,
            prices: vec![]
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_price_data(&self) -> &PriceData {
        &self.price_data
    }
    #[payable]
    pub fn push_price_data(&mut self, price_data: PriceData) {
        self.assert_price_feeder();
        let prev_storage = env::storage_usage();
        self.price_data = price_data;
        self.price_data.assert_price_data();
        let storage_cost = self.storage_cost(prev_storage);
        let refund = env::attached_deposit().checked_sub(storage_cost).expect(
            format!(
                "ERR_STORAGE_DEPOSIT need {}, attatched {}",
                storage_cost,
                env::attached_deposit()
            )
            .as_str(),
        );
        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
    }
}

impl PriceData {
    pub fn timestamp(&self) -> Timestamp {
        Timestamp::from(self.timestamp)
    }

    pub fn recency_duration(&self) -> Timestamp {
        Timestamp::from(self.recency_duration_sec) * 10u64.pow(9)
    }

    pub fn price(&self, asset: &AccountId) -> Price {
        let asset_error = format!("Oracle has NOT provided an exchange rate for {}", asset);
        self.prices
            .iter()
            .find(|aop| &aop.asset_id == asset)
            .expect(&asset_error)
            .price
            .expect(&asset_error)
    }

    pub fn assert_price_data(&self) {
        for price in &self.prices {
            let p = price.price.unwrap_or_default();
            if p.decimals != 8 || p.multiplier.0 == 0 {
                env::panic_str("invalid price data");
            } 
        }
    }
}

