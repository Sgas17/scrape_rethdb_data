/// Historical state queries using Reth changesets
///
/// This module provides functions to query storage state at specific block numbers
/// by using Reth's changeset mechanism.

use alloy_primitives::{Address, B256, U256};
use eyre::{eyre, Result};
use reth_db::{
    cursor::{DbCursorRO, DbDupCursorRO},
    tables,
    transaction::DbTx,
};

// BlockNumber is just u64 in Reth
type BlockNumber = u64;

use crate::{
    decoding,
    storage::{self, v2, v3},
    tick_math,
    types::{Bitmap, PoolInput, PoolOutput},
};

/// Query storage value at a specific block number using changesets
///
/// Returns the storage state AFTER the block executes (end of block), matching
/// standard Ethereum RPC semantics (eth_getStorageAt behavior).
///
/// Algorithm:
/// 1. Find the first changeset block STRICTLY GREATER than target block
/// 2. That changeset contains the "before" value, which is the state at our target block
/// 3. If no such changeset exists, use current PlainState (value hasn't changed since)
///
/// Example: If changes occurred at blocks [100, 200, 300] and we query block 150:
/// - First changeset > 150 is block 200
/// - Block 200's changeset has the value BEFORE block 200's transaction
/// - That's the value that existed at blocks 151-199, including our target block 150
///
/// Performance: O(log n) where n = number of changes to this slot
pub fn get_storage_at_block<TX: DbTx>(
    tx: &TX,
    address: Address,
    storage_key: B256,
    block_number: BlockNumber,
) -> Result<U256> {
    use reth_db::models::storage_sharded_key::StorageShardedKey;

    // Step 1: Use StoragesHistory index to find blocks where this slot changed
    let history_key = StorageShardedKey::new(address, storage_key, block_number);
    let mut history_cursor = tx.cursor_read::<tables::StoragesHistory>()?;

    if let Some((key, block_list)) = history_cursor.seek(history_key)? {
        // Verify this is the correct storage slot
        if key.address == address && key.sharded_key.key == storage_key {
            // Step 2: Find first changeset block STRICTLY GREATER than target
            // rank() returns count of blocks <= target
            let rank = block_list.rank(block_number);

            // select(rank) gives us the (rank+1)th smallest element
            // Since rank = count of elements <= target, select(rank) is the first element > target
            let change_block = block_list.select(rank);

            // Step 3: If found, read the "before" value from that changeset
            if let Some(change_block) = change_block {
                let mut changeset_cursor = tx.cursor_dup_read::<tables::StorageChangeSets>()?;

                if let Some(entry) = changeset_cursor
                    .seek_by_key_subkey((change_block, address).into(), storage_key)?
                {
                    if entry.key == storage_key {
                        return Ok(entry.value);
                    }
                }
            }
        }
    }

    // Step 4: No future change found - value hasn't changed since target block
    // Use current PlainState
    let mut storage_cursor = tx.cursor_dup_read::<tables::PlainStorageState>()?;
    if let Some(entry) = storage_cursor.seek_by_key_subkey(address, storage_key)? {
        if entry.key == storage_key {
            return Ok(entry.value);
        }
    }

    // Slot was never set
    Ok(U256::ZERO)
}

/// Read V3 pool data at a specific block number
pub fn read_v3_pool_at_block<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
    block_number: BlockNumber,
) -> Result<PoolOutput> {
    let tick_spacing = pool.tick_spacing.ok_or_else(|| eyre!("V3 pool missing tick_spacing"))?;

    // Read slot0 at historical block
    let slot0_slot = storage::simple_slot(v3::SLOT0);
    let slot0_value = get_storage_at_block(tx, pool.address, slot0_slot, block_number)?;
    let slot0 = decoding::decode_slot0(slot0_value)?;

    // Read liquidity at historical block
    let liquidity_slot = storage::simple_slot(v3::LIQUIDITY);
    let liquidity_value = get_storage_at_block(tx, pool.address, liquidity_slot, block_number)?;
    let liquidity = liquidity_value.to::<u128>();

    // Generate word positions to query based on tick spacing
    let word_positions = tick_math::generate_word_positions(tick_spacing);

    // Read all bitmaps at historical block
    let mut bitmaps = Vec::new();
    for word_pos in &word_positions {
        let bitmap_slot = storage::bitmap_slot(*word_pos, v3::TICK_BITMAP);
        let value = get_storage_at_block(tx, pool.address, bitmap_slot, block_number)?;

        if value != U256::ZERO {
            bitmaps.push(Bitmap {
                word_pos: *word_pos,
                bitmap: value,
            });
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

    // Read tick data for each initialized tick at historical block
    let mut ticks = Vec::new();
    for tick_value in tick_values {
        let tick_slot = storage::tick_slot(tick_value, v3::TICKS);
        let value = get_storage_at_block(tx, pool.address, tick_slot, block_number)?;

        if value != U256::ZERO {
            let tick_data = decoding::decode_tick_info(tick_value, value)?;
            ticks.push(tick_data);
        }
    }

    Ok(PoolOutput::new_v3(pool.address, slot0, liquidity, ticks, bitmaps))
}

/// Read V2 pool data at a specific block number
pub fn read_v2_pool_at_block<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
    block_number: BlockNumber,
) -> Result<PoolOutput> {
    // Read reserves at historical block from slot 8
    let reserve_slot = storage::simple_slot(v2::RESERVE);
    let value = get_storage_at_block(tx, pool.address, reserve_slot, block_number)?;

    // Decode using Alloy-based decoder
    let reserves = decoding::decode_v2_reserves(value)?;

    Ok(PoolOutput::new_v2(pool.address, reserves))
}

/// Read V4 pool data at a specific block number
pub fn read_v4_pool_at_block<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
    pool_id: B256,
    block_number: BlockNumber,
) -> Result<PoolOutput> {
    let tick_spacing = pool.tick_spacing.ok_or_else(|| eyre!("V4 pool missing tick_spacing"))?;

    // Calculate base slot for this pool
    let base_slot = crate::storage::v4_base_slot(pool_id);

    // Read slot0 at historical block (base_slot + 0)
    let slot0_slot = base_slot;
    let slot0_value = get_storage_at_block(tx, pool.address, slot0_slot, block_number)?;

    // Decode V4 slot0 (same structure as V3)
    let slot0 = decoding::decode_slot0(slot0_value)?;

    // Read liquidity at historical block (base_slot + 3)
    let liquidity_slot = crate::storage::v4_liquidity_slot(pool_id);
    let liquidity_value = get_storage_at_block(tx, pool.address, liquidity_slot, block_number)?;
    let liquidity = liquidity_value.to::<u128>();

    // Generate word positions to query based on tick spacing
    let word_positions = tick_math::generate_word_positions(tick_spacing);

    // Read all bitmaps at historical block
    // V4 bitmaps are at keccak256(abi.encode(wordPos, base_slot + 5))
    let mut bitmaps = Vec::new();
    for word_pos in &word_positions {
        let bitmap_slot = crate::storage::v4_bitmap_slot(pool_id, *word_pos);
        let value = get_storage_at_block(tx, pool.address, bitmap_slot, block_number)?;

        if value != U256::ZERO {
            bitmaps.push(Bitmap {
                word_pos: *word_pos,
                bitmap: value,
            });
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

    // Read tick data for each initialized tick at historical block
    // V4 ticks are at keccak256(abi.encode(tick, base_slot + 4))
    let mut ticks = Vec::new();
    for tick_value in tick_values {
        let tick_slot = crate::storage::v4_tick_slot(pool_id, tick_value);
        let value = get_storage_at_block(tx, pool.address, tick_slot, block_number)?;

        if value != U256::ZERO {
            // V4 ticks only have liquidityGross and liquidityNet (decode_tick_info extracts these)
            let tick_data = decoding::decode_tick_info(tick_value, value)?;
            ticks.push(tick_data);
        }
    }

    Ok(PoolOutput::new_v4(
        pool.address,
        pool_id,
        slot0,
        liquidity,
        ticks,
        bitmaps,
    ))
}

/// Query multiple storage slots at a specific block (batch optimization)
pub fn get_storage_batch_at_block<TX: DbTx>(
    tx: &TX,
    address: Address,
    storage_keys: &[B256],
    block_number: BlockNumber,
) -> Result<Vec<U256>> {
    storage_keys
        .iter()
        .map(|key| get_storage_at_block(tx, address, *key, block_number))
        .collect()
}

#[cfg(test)]
mod tests {
    // Note: These tests require a real Reth database with historical data
    // They are mostly for documentation of usage patterns

    #[test]
    #[ignore] // Requires real database
    fn test_historical_storage_query() {
        // This test demonstrates the usage pattern
        // In reality, you'd need a real database connection

        // Example:
        // let db = open_db_read_only(db_path)?;
        // let tx = db.tx()?;
        // let value = get_storage_at_block(
        //     &tx,
        //     address,
        //     storage_key,
        //     12345678  // block number
        // )?;
    }
}
