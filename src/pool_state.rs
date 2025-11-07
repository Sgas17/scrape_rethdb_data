//! Lightweight pool state reading for filtering
//! 
//! This module provides functions to quickly read just slot0 + liquidity
//! without loading all tick data, which is much faster for initial filtering.

use alloy_primitives::{Address, U256, B256};
use eyre::Result;
use reth_db::transaction::DbTx;
use reth_db::cursor::DbDupCursorRO;
use reth_db::tables;

use crate::storage::{self, v3};
use crate::types::{PoolInput, PoolOutput};
use crate::decoding::decode_slot0;

/// Read lightweight V3 pool state (slot0 + liquidity only)
pub fn read_v3_pool_state<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
) -> Result<PoolOutput> {
    let mut cursor = tx.cursor_dup_read::<tables::PlainStorageState>()?;

    // Read slot0
    let slot0 = read_slot0_helper(&mut cursor, pool.address, v3::SLOT0)?;

    // Read liquidity from slot 4
    let liquidity_slot = storage::simple_slot(v3::LIQUIDITY);
    let liquidity_value = cursor
        .seek_by_key_subkey(pool.address, liquidity_slot)?
        .filter(|entry| entry.key == liquidity_slot)
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    // Extract liquidity as u128 (it's stored in lower 128 bits)
    let liquidity = liquidity_value.to::<u128>();

    Ok(PoolOutput::new_v3(
        pool.address,
        slot0,
        liquidity,
        Vec::new(), // No ticks in slot0_only mode
        Vec::new(), // No bitmaps in slot0_only mode
    ))
}

/// Read lightweight V4 pool state (slot0 + liquidity only)
pub fn read_v4_pool_state<TX: DbTx>(
    tx: &TX,
    pool: &PoolInput,
    pool_id: B256,
) -> Result<PoolOutput> {
    let mut cursor = tx.cursor_dup_read::<tables::PlainStorageState>()?;

    // Read slot0 for this poolId
    let slot0_slot = storage::v4_slot0_slot(pool_id);
    let slot0_value = cursor
        .seek_by_key_subkey(pool.address, slot0_slot)?
        .filter(|entry| entry.key == slot0_slot)
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    let mut slot0 = decode_slot0(slot0_value)?;
    slot0.raw_data = Some(format!("0x{:064x}", slot0_value));

    // Read liquidity
    let liquidity_slot = storage::v4_liquidity_slot(pool_id);
    let liquidity_value = cursor
        .seek_by_key_subkey(pool.address, liquidity_slot)?
        .filter(|entry| entry.key == liquidity_slot)
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    let liquidity = liquidity_value.to::<u128>();

    Ok(PoolOutput::new_v4(
        pool.address,
        pool_id,
        slot0,
        liquidity,
        Vec::new(), // No ticks in slot0_only mode
        Vec::new(), // No bitmaps in slot0_only mode
    ))
}

/// Helper to read slot0 (extracted from readers.rs for reuse)
fn read_slot0_helper<C: DbDupCursorRO<tables::PlainStorageState>>(
    cursor: &mut C,
    address: Address,
    slot: u8,
) -> Result<crate::types::Slot0> {
    let slot0_slot = storage::simple_slot(slot);

    let value = cursor
        .seek_by_key_subkey(address, slot0_slot)?
        .filter(|entry| entry.key == slot0_slot)
        .map(|entry| entry.value)
        .unwrap_or(U256::ZERO);

    let mut slot0 = decode_slot0(value)?;
    slot0.raw_data = Some(format!("0x{:064x}", value));
    Ok(slot0)
}
