use alloy_primitives::{keccak256, B256, U256};

/// UniswapV3 storage slot constants
pub mod v3 {
    pub const SLOT0: u8 = 0;
    pub const FEE_GROWTH_GLOBAL0_X128: u8 = 1;
    pub const FEE_GROWTH_GLOBAL1_X128: u8 = 2;
    pub const PROTOCOL_FEES: u8 = 3;
    pub const TICKS: u8 = 4;
    pub const TICK_BITMAP: u8 = 5;
    pub const POSITIONS: u8 = 6;
    pub const OBSERVATIONS: u8 = 7;
}

/// UniswapV4 storage slot constants
/// Note: V4 uses a singleton pattern with poolId-based mapping
pub mod v4 {
    // Main pools mapping slot
    pub const POOLS_SLOT: u8 = 0;

    // Offsets within Pool.State struct (relative to pool's base slot)
    pub const SLOT0_OFFSET: u8 = 0;
    pub const FEE_GROWTH_GLOBAL0_X128_OFFSET: u8 = 1;
    pub const FEE_GROWTH_GLOBAL1_X128_OFFSET: u8 = 2;
    pub const LIQUIDITY_OFFSET: u8 = 3;
    pub const TICKS_OFFSET: u8 = 4;
    pub const TICK_BITMAP_OFFSET: u8 = 5;
}

/// UniswapV2 storage slot constants
pub mod v2 {
    pub const RESERVE: u8 = 8;
}

/// Calculate storage slot for a simple value at a fixed slot
#[inline]
pub fn simple_slot(slot: u8) -> B256 {
    let mut data = [0u8; 32];
    data[31] = slot;
    B256::from(data)
}

/// Calculate storage slot for mapping(int16 => uint256) tickBitmap
/// Formula: keccak256(abi.encode(wordPos, mappingSlot))
pub fn bitmap_slot(word_pos: i16, mapping_slot: u8) -> B256 {
    let mut data = [0u8; 64];

    // Encode word_pos as int16 with sign extension (left-padded to 32 bytes)
    if word_pos < 0 {
        data[0..30].fill(0xff); // Sign extension for negative
    }
    let word_pos_bytes = word_pos.to_be_bytes();
    data[30..32].copy_from_slice(&word_pos_bytes);

    // Encode mapping slot (left-padded to 32 bytes)
    data[63] = mapping_slot;

    keccak256(&data)
}

/// Calculate storage slot for mapping(int24 => Tick) ticks
/// Formula: keccak256(abi.encode(tick, mappingSlot))
pub fn tick_slot(tick: i32, mapping_slot: u8) -> B256 {
    let mut data = [0u8; 64];

    // Encode tick as int24 with sign extension (left-padded to 32 bytes)
    if tick < 0 {
        data[0..29].fill(0xff); // Sign extension for negative
    }
    let tick_bytes = tick.to_be_bytes();
    // int24 uses 3 bytes
    data[29..32].copy_from_slice(&tick_bytes[1..4]);

    // Encode mapping slot (left-padded to 32 bytes)
    data[63] = mapping_slot;

    keccak256(&data)
}

/// Calculate storage slot for V4 nested mapping (PoolId => mapping(int24 => Tick))
/// First hash: base_slot = keccak256(abi.encode(poolId, poolsSlot))
/// Then add offset for ticks mapping
/// Final hash: keccak256(abi.encode(tick, base_slot + offset))
pub fn v4_tick_slot(pool_id: B256, tick: i32) -> B256 {
    // Get base slot for this pool
    let base_slot = pool_base_slot(pool_id);

    // Add offset for ticks mapping
    let ticks_mapping_slot = add_offset(base_slot, v4::TICKS_OFFSET);

    // Calculate final tick slot
    tick_slot_from_base(tick, ticks_mapping_slot)
}

/// Calculate storage slot for V4 nested bitmap mapping
pub fn v4_bitmap_slot(pool_id: B256, word_pos: i16) -> B256 {
    // Get base slot for this pool
    let base_slot = pool_base_slot(pool_id);

    // Add offset for tickBitmap mapping
    let bitmap_mapping_slot = add_offset(base_slot, v4::TICK_BITMAP_OFFSET);

    // Calculate final bitmap slot
    bitmap_slot_from_base(word_pos, bitmap_mapping_slot)
}

/// Calculate V4 slot0 storage slot
pub fn v4_slot0_slot(pool_id: B256) -> B256 {
    let base_slot = pool_base_slot(pool_id);
    add_offset(base_slot, v4::SLOT0_OFFSET)
}

/// Helper: Get base storage slot for a V4 pool
fn pool_base_slot(pool_id: B256) -> B256 {
    let mut data = [0u8; 64];
    data[0..32].copy_from_slice(pool_id.as_slice());
    data[63] = v4::POOLS_SLOT;
    keccak256(&data)
}

/// Helper: Add offset to a storage slot
fn add_offset(slot: B256, offset: u8) -> B256 {
    let mut value = U256::from_be_bytes(*slot);
    value += U256::from(offset);
    B256::from(value.to_be_bytes::<32>())
}

/// Helper: Calculate tick slot given a base mapping slot (as B256)
fn tick_slot_from_base(tick: i32, mapping_slot: B256) -> B256 {
    let mut data = [0u8; 64];

    // Encode tick as int24
    if tick < 0 {
        data[0..29].fill(0xff);
    }
    let tick_bytes = tick.to_be_bytes();
    data[29..32].copy_from_slice(&tick_bytes[1..4]);

    // Encode mapping slot
    data[32..64].copy_from_slice(mapping_slot.as_slice());

    keccak256(&data)
}

/// Helper: Calculate bitmap slot given a base mapping slot (as B256)
fn bitmap_slot_from_base(word_pos: i16, mapping_slot: B256) -> B256 {
    let mut data = [0u8; 64];

    // Encode word_pos as int16
    if word_pos < 0 {
        data[0..30].fill(0xff);
    }
    let word_pos_bytes = word_pos.to_be_bytes();
    data[30..32].copy_from_slice(&word_pos_bytes);

    // Encode mapping slot
    data[32..64].copy_from_slice(mapping_slot.as_slice());

    keccak256(&data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_slot() {
        let slot = simple_slot(v3::SLOT0);
        assert_eq!(slot.as_slice()[31], 0);
    }

    #[test]
    fn test_bitmap_slot_positive() {
        let slot = bitmap_slot(10, v3::TICK_BITMAP);
        assert_ne!(slot, B256::ZERO);
    }

    #[test]
    fn test_bitmap_slot_negative() {
        let slot = bitmap_slot(-10, v3::TICK_BITMAP);
        assert_ne!(slot, B256::ZERO);

        // Should be different from positive
        let slot_pos = bitmap_slot(10, v3::TICK_BITMAP);
        assert_ne!(slot, slot_pos);
    }

    #[test]
    fn test_tick_slot() {
        let slot = tick_slot(887220, v3::TICKS);
        assert_ne!(slot, B256::ZERO);

        let slot_neg = tick_slot(-887220, v3::TICKS);
        assert_ne!(slot_neg, B256::ZERO);
        assert_ne!(slot, slot_neg);
    }

    #[test]
    fn test_v4_slots() {
        let pool_id = B256::random();

        let slot0 = v4_slot0_slot(pool_id);
        let tick_slot = v4_tick_slot(pool_id, 100);
        let bitmap_slot = v4_bitmap_slot(pool_id, 1);

        // All should be unique
        assert_ne!(slot0, tick_slot);
        assert_ne!(slot0, bitmap_slot);
        assert_ne!(tick_slot, bitmap_slot);
    }
}
