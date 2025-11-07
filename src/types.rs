use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};

// BlockNumber is just u64 in Reth
pub type BlockNumber = u64;

/// Pool protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[serde(alias = "v2", alias = "V2", alias = "uniswapv2")]
    UniswapV2,
    #[serde(alias = "v3", alias = "V3", alias = "uniswapv3")]
    UniswapV3,
    #[serde(alias = "v4", alias = "V4", alias = "uniswapv4")]
    UniswapV4,
}

/// Input configuration for a single pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInput {
    pub address: Address,
    pub protocol: Protocol,
    /// Tick spacing (required for V3/V4, ignored for V2)
    pub tick_spacing: Option<i32>,
    /// If true, only collect slot0 + liquidity (skip ticks/bitmaps for fast filtering)
    #[serde(default)]
    pub slot0_only: bool,
}

impl PoolInput {
    pub fn new_v2(address: Address) -> Self {
        Self {
            address,
            protocol: Protocol::UniswapV2,
            tick_spacing: None,
            slot0_only: false,
        }
    }

    pub fn new_v3(address: Address, tick_spacing: i32) -> Self {
        Self {
            address,
            protocol: Protocol::UniswapV3,
            tick_spacing: Some(tick_spacing),
            slot0_only: false,
        }
    }

    pub fn new_v4(address: Address, tick_spacing: i32) -> Self {
        Self {
            address,
            protocol: Protocol::UniswapV4,
            tick_spacing: Some(tick_spacing),
            slot0_only: false,
        }
    }
}

/// UniswapV3/V4 Slot0 data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Slot0 {
    /// Raw storage value as hex string for Python decoding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<String>,
    pub sqrt_price_x96: U256,
    pub tick: i32,
    pub observation_index: u16,
    pub observation_cardinality: u16,
    pub observation_cardinality_next: u16,
    pub fee_protocol: u8,
    pub unlocked: bool,
}

/// Tick data for V3/V4 pools
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tick {
    pub tick: i32,
    /// Raw storage value as hex string for Python decoding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<String>,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
    pub fee_growth_outside_0_x128: U256,
    pub fee_growth_outside_1_x128: U256,
    pub tick_cumulative_outside: i64,
    pub seconds_per_liquidity_outside_x128: U256,
    pub seconds_outside: u32,
    pub initialized: bool,
}

/// Bitmap data for a word position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bitmap {
    pub word_pos: i16,
    pub bitmap: U256,
}

/// UniswapV2 reserve data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Reserves {
    /// Raw storage value as hex string for Python decoding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<String>,
    pub reserve0: u128,
    pub reserve1: u128,
    pub block_timestamp_last: u32,
}

/// Complete output data for a single pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolOutput {
    pub address: Address,
    pub protocol: Protocol,
    /// Pool ID (only for V4 pools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<B256>,
    /// V2 reserves (only for V2 pools)
    pub reserves: Option<Reserves>,
    /// Slot0 data (only for V3/V4 pools)
    pub slot0: Option<Slot0>,
    /// Current liquidity (only for V3/V4 pools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity: Option<u128>,
    /// Tick data (only for V3/V4 pools)
    pub ticks: Vec<Tick>,
    /// Bitmap data (only for V3/V4 pools)
    pub bitmaps: Vec<Bitmap>,
}

impl PoolOutput {
    pub fn new_v2(address: Address, reserves: Reserves) -> Self {
        Self {
            address,
            protocol: Protocol::UniswapV2,
            pool_id: None,
            reserves: Some(reserves),
            slot0: None,
            liquidity: None,
            ticks: Vec::new(),
            bitmaps: Vec::new(),
        }
    }

    pub fn new_v3(
        address: Address,
        slot0: Slot0,
        liquidity: u128,
        ticks: Vec<Tick>,
        bitmaps: Vec<Bitmap>,
    ) -> Self {
        Self {
            address,
            protocol: Protocol::UniswapV3,
            pool_id: None,
            reserves: None,
            slot0: Some(slot0),
            liquidity: Some(liquidity),
            ticks,
            bitmaps,
        }
    }

    pub fn new_v4(
        address: Address,
        pool_id: B256,
        slot0: Slot0,
        liquidity: u128,
        ticks: Vec<Tick>,
        bitmaps: Vec<Bitmap>,
    ) -> Self {
        Self {
            address,
            protocol: Protocol::UniswapV4,
            pool_id: Some(pool_id),
            reserves: None,
            slot0: Some(slot0),
            liquidity: Some(liquidity),
            ticks,
            bitmaps,
        }
    }
}

/// Historical pool output with block number
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalPoolOutput {
    /// The pool data at the specified block
    #[serde(flatten)]
    pub pool_data: PoolOutput,
    /// Block number where this state was queried
    pub block_number: BlockNumber,
}
