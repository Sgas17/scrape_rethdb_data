/// Pool data readers using Alloy-based decoding
/// Clean implementation with proper storage unpacking

use alloy_primitives::{B256, U256};
use eyre::{eyre, Result};
use reth_db::{
    cursor::DbDupCursorRO,
    tables,
    transaction::DbTx,
};

use crate::{
    decoding,
    storage::{self, v2, v3},
    tick_math,
    types::{Bitmap, PoolInput, PoolOutput},
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

    // Decode using Alloy-based decoder
    let reserves = decoding::decode_v2_reserves(value)?;

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
    let slot0_slot = storage::simple_slot(v3::SLOT0);
    let slot0_value = cursor
        .seek_by_key_subkey(pool.address, slot0_slot)?
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    let slot0 = decoding::decode_slot0(slot0_value)?;

    // Read liquidity
    let liquidity_slot = storage::simple_slot(v3::LIQUIDITY);
    let liquidity_value = cursor
        .seek_by_key_subkey(pool.address, liquidity_slot)?
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    // Liquidity is stored as u128 in the lower 128 bits of the U256 storage slot
    let liquidity = liquidity_value.to::<u128>();

    // Generate word positions to query based on tick spacing
    let word_positions = tick_math::generate_word_positions(tick_spacing);

    // Read all bitmaps
    let mut bitmaps = Vec::new();
    for word_pos in &word_positions {
        let bitmap_slot = storage::bitmap_slot(*word_pos, v3::TICK_BITMAP);

        if let Some(entry) = cursor.seek_by_key_subkey(pool.address, bitmap_slot)? {
            // IMPORTANT: seek_by_key_subkey returns first entry >= requested slot
            // We must verify it's an EXACT match!
            if entry.key == bitmap_slot {
                let value = entry.value;
                if value != U256::ZERO {
                    bitmaps.push(Bitmap {
                        word_pos: *word_pos,
                        bitmap: value,
                    });
                }
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
            // Verify exact match
            if entry.key == tick_slot {
                let value = entry.value;
                if value != U256::ZERO {
                    let tick_data = decoding::decode_tick_info(tick_value, value)?;
                    ticks.push(tick_data);
                }
            }
        }
    }

    Ok(PoolOutput::new_v3(pool.address, slot0, liquidity, ticks, bitmaps))
}

/// Read V4 pool data from reth database
pub fn read_v4_pool<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
    pool_id: B256,
) -> Result<PoolOutput> {
    let tick_spacing = pool.tick_spacing.ok_or_else(|| eyre!("V4 pool missing tick_spacing"))?;

    let mut cursor = tx.cursor_dup_read::<tables::PlainStorageState>()?;

    // Read slot0 for this poolId
    let slot0_slot = storage::v4_slot0_slot(pool_id);
    let slot0_value = cursor
        .seek_by_key_subkey(pool.address, slot0_slot)?
        .filter(|entry| entry.key == slot0_slot)  // Verify exact match!
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    let slot0 = decoding::decode_slot0(slot0_value)?;

    // Read liquidity for this poolId
    let liquidity_slot = storage::v4_liquidity_slot(pool_id);
    let liquidity_value = cursor
        .seek_by_key_subkey(pool.address, liquidity_slot)?
        .filter(|entry| entry.key == liquidity_slot)  // Verify exact match!
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    // Liquidity is stored as u128 in the lower 128 bits of the U256 storage slot
    let liquidity = liquidity_value.to::<u128>();

    // Generate word positions
    let word_positions = tick_math::generate_word_positions(tick_spacing);

    // Read all bitmaps for this pool
    let mut bitmaps = Vec::new();
    for word_pos in &word_positions {
        let bitmap_slot = storage::v4_bitmap_slot(pool_id, *word_pos);

        if let Some(entry) = cursor.seek_by_key_subkey(pool.address, bitmap_slot)? {
            // Verify exact match
            if entry.key == bitmap_slot {
                let value = entry.value;
                if value != U256::ZERO {
                    bitmaps.push(Bitmap {
                        word_pos: *word_pos,
                        bitmap: value,
                    });
                }
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
            // Verify exact match
            if entry.key == tick_slot {
                let value = entry.value;
                if value != U256::ZERO {
                    let tick_data = decoding::decode_tick_info(tick_value, value)?;
                    ticks.push(tick_data);
                }
            }
        }
    }

    Ok(PoolOutput::new_v4(pool.address, pool_id, slot0, liquidity, ticks, bitmaps))
}
