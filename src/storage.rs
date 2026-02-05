use alloy_primitives::{keccak256, Address, B256, U256};
use alloy_sol_types::SolValue;

/// Known V3 factory addresses for storage layout detection.
pub mod factories {
    use alloy_primitives::address;

    /// PancakeSwap V3 factory (has +1 slot offset due to `lmPool` storage).
    pub const PANCAKESWAP_V3: alloy_primitives::Address =
        address!("0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865");
}

/// `UniswapV3` storage slot constants (also used by SushiSwap V3).
pub mod v3 {
    pub const SLOT0: u8 = 0;
    pub const FEE_GROWTH_GLOBAL0_X128: u8 = 1;
    pub const FEE_GROWTH_GLOBAL1_X128: u8 = 2;
    pub const PROTOCOL_FEES: u8 = 3;
    pub const LIQUIDITY: u8 = 4;
    pub const TICKS: u8 = 5;
    pub const TICK_BITMAP: u8 = 6;
    pub const POSITIONS: u8 = 7;
    pub const OBSERVATIONS: u8 = 8;
}

/// `PancakeSwap V3` storage slot constants.
/// PancakeSwap V3 pools have an extra `lmPool` slot at position 1,
/// which shifts all subsequent slots by 1.
pub mod pancakeswap_v3 {
    pub const SLOT0: u8 = 0;
    pub const LM_POOL: u8 = 1; // Extra slot: address of LM pool
    pub const FEE_GROWTH_GLOBAL0_X128: u8 = 2;
    pub const FEE_GROWTH_GLOBAL1_X128: u8 = 3;
    pub const PROTOCOL_FEES: u8 = 4;
    pub const LIQUIDITY: u8 = 5;
    pub const TICKS: u8 = 6;
    pub const TICK_BITMAP: u8 = 7;
    pub const POSITIONS: u8 = 8;
    pub const OBSERVATIONS: u8 = 9;
}

/// Get V3 storage slots based on factory address.
/// Returns (slot0, liquidity, ticks, tick_bitmap) slot numbers.
pub fn v3_slots_for_factory(factory: Option<Address>) -> V3Slots {
    if let Some(f) = factory {
        if f == factories::PANCAKESWAP_V3 {
            return V3Slots {
                slot0: pancakeswap_v3::SLOT0,
                liquidity: pancakeswap_v3::LIQUIDITY,
                ticks: pancakeswap_v3::TICKS,
                tick_bitmap: pancakeswap_v3::TICK_BITMAP,
            };
        }
    }
    V3Slots {
        slot0: v3::SLOT0,
        liquidity: v3::LIQUIDITY,
        ticks: v3::TICKS,
        tick_bitmap: v3::TICK_BITMAP,
    }
}

/// V3 storage slot configuration.
#[derive(Debug, Clone, Copy)]
pub struct V3Slots {
    pub slot0: u8,
    pub liquidity: u8,
    pub ticks: u8,
    pub tick_bitmap: u8,
}

/// `UniswapV4` storage slot constants.
/// V4 uses a singleton pattern with poolId-based mapping.
pub mod v4 {
    // Main pools mapping slot.
    // PoolManager inherits from multiple contracts, so _pools is at slot 6.
    pub const POOLS_SLOT: u8 = 6;

    // Offsets within Pool.State struct (relative to pool's base slot).
    pub const SLOT0_OFFSET: u8 = 0;
    pub const FEE_GROWTH_GLOBAL0_X128_OFFSET: u8 = 1;
    pub const FEE_GROWTH_GLOBAL1_X128_OFFSET: u8 = 2;
    pub const LIQUIDITY_OFFSET: u8 = 3;
    pub const TICKS_OFFSET: u8 = 4;
    pub const TICK_BITMAP_OFFSET: u8 = 5;
}

/// `UniswapV2` storage slot constants.
pub mod v2 {
    pub const RESERVE: u8 = 8;
}

/// Calculate storage slot for a simple value at a fixed slot.
#[inline]
pub fn simple_slot(slot: u8) -> B256 {
    let mut data = [0u8; 32];
    data[31] = slot;
    B256::from(data)
}

/// Calculate storage slot for mapping(int16 => uint256) tickBitmap.
/// Formula: keccak256(abi.encode(wordPos, mappingSlot))
pub fn bitmap_slot(word_pos: i16, mapping_slot: u8) -> B256 {
    let encoded = (word_pos, U256::from(mapping_slot)).abi_encode();
    keccak256(&encoded)
}

/// Calculate storage slot for mapping(int24 => Tick) ticks.
/// Formula: keccak256(abi.encode(tick, mappingSlot))
pub fn tick_slot(tick: i32, mapping_slot: u8) -> B256 {
    let encoded = (tick, U256::from(mapping_slot)).abi_encode();
    keccak256(&encoded)
}

/// Calculate storage slot for V4 nested mapping (`PoolId` => mapping(int24 => Tick)).
/// First hash: `base_slot` = keccak256(abi.encode(poolId, poolsSlot))
/// Then add offset for ticks mapping.
/// Final hash: keccak256(abi.encode(tick, `base_slot` + offset))
pub fn v4_tick_slot(pool_id: B256, tick: i32) -> B256 {
    let base_slot = pool_base_slot(pool_id);
    let ticks_mapping_slot = add_offset(base_slot, v4::TICKS_OFFSET);
    tick_slot_from_base(tick, ticks_mapping_slot)
}

/// Calculate storage slot for V4 nested bitmap mapping.
pub fn v4_bitmap_slot(pool_id: B256, word_pos: i16) -> B256 {
    let base_slot = pool_base_slot(pool_id);
    let bitmap_mapping_slot = add_offset(base_slot, v4::TICK_BITMAP_OFFSET);
    bitmap_slot_from_base(word_pos, bitmap_mapping_slot)
}

/// Calculate V4 slot0 storage slot.
pub fn v4_slot0_slot(pool_id: B256) -> B256 {
    let base_slot = pool_base_slot(pool_id);
    add_offset(base_slot, v4::SLOT0_OFFSET)
}

/// Calculate V4 liquidity storage slot.
pub fn v4_liquidity_slot(pool_id: B256) -> B256 {
    let base_slot = pool_base_slot(pool_id);
    add_offset(base_slot, v4::LIQUIDITY_OFFSET)
}

/// Get base storage slot for a V4 pool.
/// This is the base slot where Pool.State struct begins for a given poolId.
pub fn v4_base_slot(pool_id: B256) -> B256 {
    pool_base_slot(pool_id)
}

/// Helper: Get base storage slot for a V4 pool.
fn pool_base_slot(pool_id: B256) -> B256 {
    let encoded = (pool_id, U256::from(v4::POOLS_SLOT)).abi_encode();
    keccak256(&encoded)
}

/// Helper: Add offset to a storage slot.
fn add_offset(slot: B256, offset: u8) -> B256 {
    let mut value = U256::from_be_bytes(*slot);
    value += U256::from(offset);
    B256::from(value.to_be_bytes::<32>())
}

/// Helper: Calculate tick slot given a base mapping slot (as B256).
fn tick_slot_from_base(tick: i32, mapping_slot: B256) -> B256 {
    let mapping_u256 = U256::from_be_bytes(*mapping_slot);
    let encoded = (tick, mapping_u256).abi_encode();
    keccak256(&encoded)
}

/// Helper: Calculate bitmap slot given a base mapping slot (as B256).
fn bitmap_slot_from_base(word_pos: i16, mapping_slot: B256) -> B256 {
    let mapping_u256 = U256::from_be_bytes(*mapping_slot);
    let encoded = (word_pos, mapping_u256).abi_encode();
    keccak256(&encoded)
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
    fn test_pancakeswap_factory_detection() {
        // Test that lowercase factory from DB matches the constant
        let pancake_lower: Address = "0x0bfbcf9fa4f9c56b0f40a671ad40e0805a091865"
            .parse()
            .unwrap();
        let pancake_mixed: Address = "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"
            .parse()
            .unwrap();

        // Both should equal the constant
        assert_eq!(pancake_lower, factories::PANCAKESWAP_V3);
        assert_eq!(pancake_mixed, factories::PANCAKESWAP_V3);

        // And v3_slots_for_factory should return PancakeSwap slots
        let slots_lower = v3_slots_for_factory(Some(pancake_lower));
        let slots_mixed = v3_slots_for_factory(Some(pancake_mixed));

        assert_eq!(slots_lower.liquidity, pancakeswap_v3::LIQUIDITY);
        assert_eq!(slots_mixed.liquidity, pancakeswap_v3::LIQUIDITY);
        assert_eq!(slots_lower.liquidity, 5); // PancakeSwap liquidity is at slot 5
    }

    #[test]
    fn test_uniswap_factory_uses_default_slots() {
        let uniswap_v3: Address = "0x1F98431c8aD98523631AE4a59f267346ea31F984"
            .parse()
            .unwrap();

        let slots = v3_slots_for_factory(Some(uniswap_v3));
        assert_eq!(slots.liquidity, v3::LIQUIDITY);
        assert_eq!(slots.liquidity, 4); // Uniswap liquidity is at slot 4
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
        let pool_id = B256::from([0x42; 32]);

        let slot0 = v4_slot0_slot(pool_id);
        let tick = v4_tick_slot(pool_id, 100);
        let bitmap = v4_bitmap_slot(pool_id, 1);

        // All should be unique
        assert_ne!(slot0, tick);
        assert_ne!(slot0, bitmap);
        assert_ne!(tick, bitmap);
    }
}
