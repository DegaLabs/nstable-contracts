use near_sdk::Timestamp;

use crate::*;

type DurationSec = u32;

// From https://github.com/NearDeFi/price-oracle/blob/main/src/utils.rs
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Copy)]
#[serde(crate = "near_sdk::serde")]
pub struct Price {
    pub multiplier: U128,
    pub decimals: u8,
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

    pub fn push_price_data(&mut self, price_data: PriceData) {
        self.assert_price_feeder();
        self.price_data = price_data;
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
}

#[cfg(test)]
impl ExchangeRate {
    // pub fn test_fresh_rate() -> Self {
    //     Self {
    //         multiplier: 111439,
    //         decimals: 28,
    //         timestamp: env::block_timestamp(),
    //         recency_duration: env::block_timestamp() + 1000000000,
    //     }
    // }

    // pub fn test_old_rate() -> Self {
    //     Self {
    //         multiplier: 111439,
    //         decimals: 28,
    //         timestamp: env::block_timestamp(),
    //         recency_duration: env::block_timestamp(),
    //     }
    // }
}
