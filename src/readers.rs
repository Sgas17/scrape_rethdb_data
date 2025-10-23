use alloy_primitives::{Address, B256, U256};
use eyre::{eyre, Result};
use reth_db::{
    cursor::DbDupCursorRO,
    tables,
    transaction::DbTx,
};

use crate::{
    storage::{self, v2, v3},
    tick_math,
    types::{Bitmap, PoolInput, PoolOutput, Protocol, Reserves, Slot0, Tick},
};

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

    // Parse packed reserves: reserve0 (112 bits) | reserve1 (112 bits) | timestamp (32 bits)
    let reserves = parse_v2_reserves(value);

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
                let tick_data = parse_tick_data(tick_value, value);
                ticks.push(tick_data);
            }
        }
    }

    Ok(PoolOutput::new_v3_v4(
        pool.address,
        Protocol::UniswapV3,
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

    let slot0 = parse_slot0(slot0_value);

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
                let tick_data = parse_tick_data(tick_value, value);
                ticks.push(tick_data);
            }
        }
    }

    Ok(PoolOutput::new_v3_v4(
        pool.address,
        Protocol::UniswapV4,
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

    Ok(parse_slot0(value))
}

/// Parse slot0 from storage value
/// Slot0 structure (packed):
/// - sqrtPriceX96: uint160
/// - tick: int24
/// - observationIndex: uint16
/// - observationCardinality: uint16
/// - observationCardinalityNext: uint16
/// - feeProtocol: uint8
/// - unlocked: bool
fn parse_slot0(value: U256) -> Slot0 {
    let bytes = value.to_be_bytes::<32>();

    // Parse from right to left (little-endian packing)
    // sqrtPriceX96 (20 bytes) at bytes[12..32]
    let sqrt_price_x96 = U256::from_be_slice(&bytes[12..32]);

    // tick (3 bytes, signed) at bytes[9..12]
    let tick = i32::from_be_bytes([
        if bytes[9] & 0x80 != 0 { 0xff } else { 0 },
        bytes[9],
        bytes[10],
        bytes[11],
    ]);

    // observationIndex (2 bytes) at bytes[7..9]
    let observation_index = u16::from_be_bytes([bytes[7], bytes[8]]);

    // observationCardinality (2 bytes) at bytes[5..7]
    let observation_cardinality = u16::from_be_bytes([bytes[5], bytes[6]]);

    // observationCardinalityNext (2 bytes) at bytes[3..5]
    let observation_cardinality_next = u16::from_be_bytes([bytes[3], bytes[4]]);

    // feeProtocol (1 byte) at bytes[2]
    let fee_protocol = bytes[2];

    // unlocked (1 byte bool) at bytes[1]
    let unlocked = bytes[1] != 0;

    Slot0 {
        sqrt_price_x96,
        tick,
        observation_index,
        observation_cardinality,
        observation_cardinality_next,
        fee_protocol,
        unlocked,
    }
}

/// Parse V2 reserves from storage value
fn parse_v2_reserves(value: U256) -> Reserves {
    let bytes = value.to_be_bytes::<32>();

    // Packed as: reserve0 (112 bits) | reserve1 (112 bits) | blockTimestampLast (32 bits)
    // Reading from right to left:
    // blockTimestampLast: bytes[28..32] (4 bytes)
    let block_timestamp_last = u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);

    // reserve1: bytes[14..28] (14 bytes = 112 bits)
    let reserve1 = u128::from_be_bytes([
        0,
        0,
        bytes[14],
        bytes[15],
        bytes[16],
        bytes[17],
        bytes[18],
        bytes[19],
        bytes[20],
        bytes[21],
        bytes[22],
        bytes[23],
        bytes[24],
        bytes[25],
        bytes[26],
        bytes[27],
    ]);

    // reserve0: bytes[0..14] (14 bytes = 112 bits)
    let reserve0 = u128::from_be_bytes([
        0,
        0,
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
    ]);

    Reserves {
        reserve0,
        reserve1,
        block_timestamp_last,
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
