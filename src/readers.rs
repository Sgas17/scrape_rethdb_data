use alloy_primitives::{Address, B256, U256};
use alloy_sol_types::{sol, SolType, SolValue};
use eyre::{eyre, Result};
use reth_db::{
    cursor::DbDupCursorRO,
    tables,
    transaction::DbTx,
};

use crate::{
    storage::{self, v2, v3},
    tick_math,
    types::{Bitmap, PoolInput, PoolOutput, Reserves, Slot0, Tick},
};

// Define Solidity types for decoding
sol! {
    // V2 reserves: packed (uint112, uint112, uint32)
    struct V2Reserves {
        uint112 reserve0;
        uint112 reserve1;
        uint32 blockTimestampLast;
    }

    // V3/V4 Slot0: packed storage
    struct Slot0Data {
        uint160 sqrtPriceX96;
        int24 tick;
        uint16 observationIndex;
        uint16 observationCardinality;
        uint16 observationCardinalityNext;
        uint8 feeProtocol;
        bool unlocked;
    }
}

/// Read V2 reserve data from reth database
pub fn read_v2_pool<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
) -> Result<PoolOutput> {
    let mut cursor = tx.cursor_dup_read::<tables::PlainStorageState>()?;

    // Read reserves from slot 8
    let reserve_slot = storage::simple_slot(v2::RESERVE);

    let value = cursor
        .seek_by_key_subkey(pool.address, reserve_slot)?
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    // Parse packed storage (NOT ABI encoded!)
    // Solidity packs variables tightly from RIGHT to LEFT in storage
    // For: uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast
    // Actual layout in U256 (as bytes): timestamp | reserve1 | reserve0
    // Total: 32 + 112 + 112 = 256 bits

    let bytes = value.to_be_bytes::<32>();

    // BlockTimestampLast: first 4 bytes (bytes 0-3)
    let block_timestamp_last = u32::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3]
    ]);

    // Reserve1: next 14 bytes (bytes 4-17)
    let reserve1 = u128::from_be_bytes([
        0, 0,  // Pad to 128 bits
        bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10],
        bytes[11], bytes[12], bytes[13], bytes[14], bytes[15], bytes[16], bytes[17]
    ]);

    // Reserve0: last 14 bytes (bytes 18-31)
    let reserve0 = u128::from_be_bytes([
        0, 0,  // Pad to 128 bits
        bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23], bytes[24],
        bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31]
    ]);

    let reserves = Reserves {
        raw_data: Some(format!("0x{:064x}", value)),
        reserve0,
        reserve1,
        block_timestamp_last,
    };

    Ok(PoolOutput::new_v2(pool.address, reserves))
}

/// Read V3 pool data from reth database
pub fn read_v3_pool<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
) -> Result<PoolOutput> {
    let tick_spacing = pool.tick_spacing.ok_or_else(|| eyre!("V3 pool missing tick_spacing"))?;

    let mut cursor = tx.cursor_dup_read::<tables::PlainStorageState>()?;

    // Read slot0
    let slot0 = read_slot0(&mut cursor, pool.address, v3::SLOT0)?;

    // Generate word positions to query based on tick spacing
    let word_positions = tick_math::generate_word_positions(tick_spacing);

    // Read all bitmaps
    let mut bitmaps = Vec::new();
    for word_pos in &word_positions {
        let bitmap_slot = storage::bitmap_slot(*word_pos, v3::TICK_BITMAP);

        if let Some(entry) = cursor.seek_by_key_subkey(pool.address, bitmap_slot)? {
            let value = entry.value;
            if value != U256::ZERO {
                bitmaps.push(Bitmap {
                    word_pos: *word_pos,
                    bitmap: value,
                });
            }
        }
    }

    // Extract initialized ticks from bitmaps
    let mut tick_values = Vec::new();
    for bitmap in &bitmaps {
        let bitmap_bytes = bitmap.bitmap.to_be_bytes::<32>();
        let ticks = tick_math::extract_ticks_from_bitmap_u256(
            bitmap.word_pos,
            &bitmap_bytes,
            tick_spacing,
        );
        tick_values.extend(ticks);
    }

    // Read tick data for each initialized tick
    let mut ticks = Vec::new();
    for tick_value in tick_values {
        let tick_slot = storage::tick_slot(tick_value, v3::TICKS);

        if let Some(entry) = cursor.seek_by_key_subkey(pool.address, tick_slot)? {
            let value = entry.value;
            if value != U256::ZERO {
                // Parse tick data (simplified - tick data is stored across multiple slots)
                let mut tick_data = parse_tick_data(tick_value, value);
                tick_data.raw_data = Some(format!("0x{:064x}", value));
                ticks.push(tick_data);
            }
        }
    }

    Ok(PoolOutput::new_v3(
        pool.address,
        slot0,
        ticks,
        bitmaps,
    ))
}

/// Read V4 pool data from reth database
pub fn read_v4_pool<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
    pool_id: B256,
) -> Result<PoolOutput> {
    let tick_spacing = pool.tick_spacing.ok_or_else(|| eyre!("V4 pool missing tick_spacing"))?;

    let mut cursor = tx.cursor_dup_read::<tables::PlainStorageState>()?;

    // For V4, we need to query using the singleton pattern
    // The "pool address" in V4 is the PoolManager singleton
    // Individual pool data is accessed via poolId

    // Read slot0 for this poolId
    let slot0_slot = storage::v4_slot0_slot(pool_id);
    let slot0_value = cursor
        .seek_by_key_subkey(pool.address, slot0_slot)?
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    let mut slot0 = parse_slot0(slot0_value);
    slot0.raw_data = Some(format!("0x{:064x}", slot0_value));

    // Generate word positions
    let word_positions = tick_math::generate_word_positions(tick_spacing);

    // Read all bitmaps for this pool
    let mut bitmaps = Vec::new();
    for word_pos in &word_positions {
        let bitmap_slot = storage::v4_bitmap_slot(pool_id, *word_pos);

        if let Some(entry) = cursor.seek_by_key_subkey(pool.address, bitmap_slot)? {
            let value = entry.value;
            if value != U256::ZERO {
                bitmaps.push(Bitmap {
                    word_pos: *word_pos,
                    bitmap: value,
                });
            }
        }
    }

    // Extract initialized ticks
    let mut tick_values = Vec::new();
    for bitmap in &bitmaps {
        let bitmap_bytes = bitmap.bitmap.to_be_bytes::<32>();
        let ticks = tick_math::extract_ticks_from_bitmap_u256(
            bitmap.word_pos,
            &bitmap_bytes,
            tick_spacing,
        );
        tick_values.extend(ticks);
    }

    // Read tick data
    let mut ticks = Vec::new();
    for tick_value in tick_values {
        let tick_slot = storage::v4_tick_slot(pool_id, tick_value);

        if let Some(entry) = cursor.seek_by_key_subkey(pool.address, tick_slot)? {
            let value = entry.value;
            if value != U256::ZERO {
                let mut tick_data = parse_tick_data(tick_value, value);
                tick_data.raw_data = Some(format!("0x{:064x}", value));
                ticks.push(tick_data);
            }
        }
    }

    Ok(PoolOutput::new_v4(
        pool.address,
        pool_id,
        slot0,
        ticks,
        bitmaps,
    ))
}

/// Helper: Read slot0 from storage
fn read_slot0<C: DbDupCursorRO<tables::PlainStorageState>>(
    cursor: &mut C,
    address: Address,
    slot: u8,
) -> Result<Slot0> {
    let slot0_slot = storage::simple_slot(slot);

    let value = cursor
        .seek_by_key_subkey(address, slot0_slot)?
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    let mut slot0 = parse_slot0(value);
    slot0.raw_data = Some(format!("0x{:064x}", value));
    Ok(slot0)
}

/// Parse slot0 from storage value - manual parsing of packed storage
/// Slot0 is packed from right to left:
/// unlocked (1 byte) | feeProtocol (1 byte) | observationCardinalityNext (2 bytes) |
/// observationCardinality (2 bytes) | observationIndex (2 bytes) | tick (3 bytes) | sqrtPriceX96 (20 bytes)
fn parse_slot0(value: U256) -> Slot0 {
    // Extract sqrtPriceX96 (160 bits = 20 bytes, rightmost)
    let mask_160 = (U256::from(1u128) << 160) - U256::from(1u128);
    let sqrt_price_x96 = value & mask_160;

    // Extract tick (24 bits = 3 bytes, signed, next)
    let tick_u256: U256 = (value >> 160) & U256::from(0xFFFFFFu64);
    let tick_raw = tick_u256.to::<u32>();
    // Sign extend from 24 bits to 32 bits
    let tick = if tick_raw & 0x800000 != 0 {
        // Negative number - sign extend
        (tick_raw | 0xFF000000) as i32
    } else {
        tick_raw as i32
    };

    // Extract observationIndex (16 bits, next)
    let obs_index_u256: U256 = (value >> 184) & U256::from(0xFFFFu64);
    let observation_index = obs_index_u256.to::<u16>();

    // Extract observationCardinality (16 bits, next)
    let obs_card_u256: U256 = (value >> 200) & U256::from(0xFFFFu64);
    let observation_cardinality = obs_card_u256.to::<u16>();

    // Extract observationCardinalityNext (16 bits, next)
    let obs_card_next_u256: U256 = (value >> 216) & U256::from(0xFFFFu64);
    let observation_cardinality_next = obs_card_next_u256.to::<u16>();

    // Extract feeProtocol (8 bits, next)
    let fee_proto_u256: U256 = (value >> 232) & U256::from(0xFFu64);
    let fee_protocol = fee_proto_u256.to::<u8>();

    // Extract unlocked (8 bits, leftmost)
    let unlocked_u256: U256 = (value >> 240) & U256::from(0xFFu64);
    let unlocked = unlocked_u256.to::<u8>() != 0;

    Slot0 {
        raw_data: None,
        sqrt_price_x96,
        tick,
        observation_index,
        observation_cardinality,
        observation_cardinality_next,
        fee_protocol,
        unlocked,
    }
}

/// Parse tick data from storage value
/// NOTE: In reality, tick data is stored across multiple storage slots
/// This is a simplified version that reads the primary slot
fn parse_tick_data(tick: i32, _value: U256) -> Tick {
    // TODO: Implement full tick parsing across multiple slots
    // For now, return a minimal tick structure
    Tick {
        tick,
        raw_data: None,
        liquidity_gross: 0,
        liquidity_net: 0,
        fee_growth_outside_0_x128: U256::ZERO,
        fee_growth_outside_1_x128: U256::ZERO,
        tick_cumulative_outside: 0,
        seconds_per_liquidity_outside_x128: U256::ZERO,
        seconds_outside: 0,
        initialized: true,
    }
}
