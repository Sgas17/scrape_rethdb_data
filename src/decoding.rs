/// Utilities for decoding packed Solidity storage values
///
/// IMPORTANT: Solidity storage packing is different from ABI encoding!
/// - Storage packing: Variables packed RIGHT to LEFT (LSB to MSB)
/// - ABI encoding: Each value padded to 32 bytes, concatenated

use alloy_primitives::U256;
use eyre::Result;

use crate::types::{Reserves, Slot0, Tick};

/// Decode V2 reserves from packed storage
///
/// Solidity: `uint112 reserve0; uint112 reserve1; uint32 blockTimestampLast;`
/// Storage layout (256 bits, RIGHT to LEFT):
/// - Bits 0-111: reserve0 (uint112)
/// - Bits 112-223: reserve1 (uint112)
/// - Bits 224-255: blockTimestampLast (uint32)
pub fn decode_v2_reserves(storage_value: U256) -> Result<Reserves> {
    let raw_hex = format!("0x{:064x}", storage_value);

    // Extract from packed storage (RIGHT to LEFT)
    // reserve0 is in the lowest 112 bits
    let reserve0_mask = (U256::from(1u128) << 112) - U256::from(1u128);
    let reserve0_u256: U256 = storage_value & reserve0_mask;
    let reserve0 = reserve0_u256.to::<u128>();

    // reserve1 is in bits 112-223
    let reserve1_u256: U256 = (storage_value >> 112) & reserve0_mask;
    let reserve1 = reserve1_u256.to::<u128>();

    // blockTimestampLast is in the highest 32 bits
    let timestamp_u256: U256 = storage_value >> 224;
    let block_timestamp_last = timestamp_u256.to::<u32>();

    Ok(Reserves {
        raw_data: Some(raw_hex),
        reserve0,
        reserve1,
        block_timestamp_last,
    })
}

/// Decode V3/V4 Slot0 from packed storage
///
/// Solidity:
/// ```solidity
/// struct Slot0 {
///     uint160 sqrtPriceX96;
///     int24 tick;
///     uint16 observationIndex;
///     uint16 observationCardinality;
///     uint16 observationCardinalityNext;
///     uint8 feeProtocol;
///     bool unlocked;
/// }
/// ```
///
/// Packed storage layout (RIGHT to LEFT):
/// - Bits 0-159: sqrtPriceX96 (uint160)
/// - Bits 160-183: tick (int24)
/// - Bits 184-199: observationIndex (uint16)
/// - Bits 200-215: observationCardinality (uint16)
/// - Bits 216-231: observationCardinalityNext (uint16)
/// - Bits 232-239: feeProtocol (uint8)
/// - Bit 240: unlocked (bool)
pub fn decode_slot0(storage_value: U256) -> Result<Slot0> {
    let raw_hex = format!("0x{:064x}", storage_value);

    // sqrtPriceX96: bits 0-159 (160 bits)
    let sqrt_price_mask = (U256::from(1u128) << 160) - U256::from(1u128);
    let sqrt_price_x96 = storage_value & sqrt_price_mask;

    // tick: bits 160-183 (24 bits, signed)
    let tick_u256: U256 = (storage_value >> 160) & U256::from(0xFFFFFFu32);
    let tick_raw = tick_u256.to::<u32>();
    // Handle sign extension for int24
    let tick = if tick_raw & 0x800000 != 0 {
        // Negative number - sign extend
        (tick_raw | 0xFF000000) as i32
    } else {
        tick_raw as i32
    };

    // observationIndex: bits 184-199 (16 bits)
    let obs_idx_u256: U256 = (storage_value >> 184) & U256::from(0xFFFFu32);
    let observation_index = obs_idx_u256.to::<u16>();

    // observationCardinality: bits 200-215 (16 bits)
    let obs_card_u256: U256 = (storage_value >> 200) & U256::from(0xFFFFu32);
    let observation_cardinality = obs_card_u256.to::<u16>();

    // observationCardinalityNext: bits 216-231 (16 bits)
    let obs_card_next_u256: U256 = (storage_value >> 216) & U256::from(0xFFFFu32);
    let observation_cardinality_next = obs_card_next_u256.to::<u16>();

    // feeProtocol: bits 232-239 (8 bits)
    let fee_proto_u256: U256 = (storage_value >> 232) & U256::from(0xFFu32);
    let fee_protocol = fee_proto_u256.to::<u8>();

    // unlocked: bit 240 (1 bit)
    let unlocked_u256: U256 = (storage_value >> 240) & U256::from(1u32);
    let unlocked = unlocked_u256 != U256::ZERO;

    Ok(Slot0 {
        raw_data: Some(raw_hex),
        sqrt_price_x96,
        tick,
        observation_index,
        observation_cardinality,
        observation_cardinality_next,
        fee_protocol,
        unlocked,
    })
}

/// Decode tick info from storage
///
/// Uniswap V3/V4 Tick storage layout (slot 0):
/// - Bits 0-127: liquidityGross (uint128)
/// - Bits 128-255: liquidityNet (int128)
///
/// Note: Additional tick data (fee growth, etc.) is in subsequent slots
/// but we only need liquidity values for basic functionality
pub fn decode_tick_info(tick: i32, storage_value: U256) -> Result<Tick> {
    let raw_hex = format!("0x{:064x}", storage_value);

    let initialized = storage_value != U256::ZERO;

    // Extract liquidityGross (lower 128 bits)
    let liquidity_gross_mask = (U256::from(1u128) << 128) - U256::from(1u128);
    let liquidity_gross_u256: U256 = storage_value & liquidity_gross_mask;
    let liquidity_gross = liquidity_gross_u256.to::<u128>();

    // Extract liquidityNet (upper 128 bits, signed int128)
    let liquidity_net_u256: U256 = storage_value >> 128;
    let liquidity_net_raw = liquidity_net_u256.to::<u128>();

    // Convert to signed int128 using two's complement
    let liquidity_net = if liquidity_net_raw > (u128::MAX / 2) {
        // Negative number in two's complement
        -(((!liquidity_net_raw).wrapping_add(1)) as i128)
    } else {
        liquidity_net_raw as i128
    };

    Ok(Tick {
        tick,
        raw_data: Some(raw_hex),
        liquidity_gross,
        liquidity_net,
        fee_growth_outside_0_x128: U256::ZERO,  // Would need to read additional slots
        fee_growth_outside_1_x128: U256::ZERO,
        tick_cumulative_outside: 0,
        seconds_per_liquidity_outside_x128: U256::ZERO,
        seconds_outside: 0,
        initialized,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v2_reserves_decoding() {
        // Example: reserve0=1000, reserve1=2000, timestamp=123456
        // Layout (R to L): timestamp (32 bits) | reserve1 (112 bits) | reserve0 (112 bits)
        let reserve0 = U256::from(1000u128);
        let reserve1 = U256::from(2000u128) << 112;
        let timestamp = U256::from(123456u32) << 224;
        let packed = reserve0 | reserve1 | timestamp;

        let decoded = decode_v2_reserves(packed).unwrap();

        assert_eq!(decoded.reserve0, 1000);
        assert_eq!(decoded.reserve1, 2000);
        assert_eq!(decoded.block_timestamp_last, 123456);
    }

    #[test]
    fn test_slot0_decoding() {
        // Create a test slot0 value
        let sqrt_price = U256::from(1000u128);
        let tick = U256::from(100u32) << 160;  // Positive tick
        let obs_idx = U256::from(5u32) << 184;
        let obs_card = U256::from(10u32) << 200;
        let obs_card_next = U256::from(20u32) << 216;
        let fee_protocol = U256::from(3u32) << 232;
        let unlocked = U256::from(1u32) << 240;

        let packed = sqrt_price | tick | obs_idx | obs_card | obs_card_next | fee_protocol | unlocked;

        let decoded = decode_slot0(packed).unwrap();

        assert_eq!(decoded.sqrt_price_x96, U256::from(1000u128));
        assert_eq!(decoded.tick, 100);
        assert_eq!(decoded.observation_index, 5);
        assert_eq!(decoded.observation_cardinality, 10);
        assert_eq!(decoded.observation_cardinality_next, 20);
        assert_eq!(decoded.fee_protocol, 3);
        assert_eq!(decoded.unlocked, true);
    }

    #[test]
    fn test_slot0_negative_tick() {
        // Test with negative tick
        let sqrt_price = U256::from(1000u128);
        // -100 in int24 = 0xFFFF9C (two's complement)
        let tick_raw = 0xFFFF9Cu32;
        let tick = U256::from(tick_raw) << 160;

        let packed = sqrt_price | tick;
        let decoded = decode_slot0(packed).unwrap();

        assert_eq!(decoded.tick, -100);
    }
}
