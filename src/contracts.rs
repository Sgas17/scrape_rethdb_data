/// Contract type definitions using Alloy sol! macro
/// These match the exact Solidity storage layouts for direct DB decoding

use alloy_sol_types::sol;

// UniswapV2 Pair contract storage
sol! {
    /// UniswapV2 reserves are packed into a single storage slot (slot 8)
    /// Layout: reserve0 (uint112) | reserve1 (uint112) | blockTimestampLast (uint32)
    struct ReservesStorage {
        uint112 reserve0;
        uint112 reserve1;
        uint32 blockTimestampLast;
    }
}

// UniswapV3/V4 Pool storage structures
sol! {
    /// Slot0 contains the pool's current price and state (slot 0 for V3, varies for V4)
    /// This is a packed struct in Solidity storage
    struct Slot0Storage {
        uint160 sqrtPriceX96;
        int24 tick;
        uint16 observationIndex;
        uint16 observationCardinality;
        uint16 observationCardinalityNext;
        uint8 feeProtocol;
        bool unlocked;
    }

    /// Tick info stored in the ticks mapping (slot 4 for V3)
    /// Each tick consumes multiple storage slots due to size
    struct TickInfo {
        uint128 liquidityGross;
        int128 liquidityNet;
        uint256 feeGrowthOutside0X128;
        uint256 feeGrowthOutside1X128;
        int56 tickCumulativeOutside;
        uint160 secondsPerLiquidityOutsideX128;
        uint32 secondsOutside;
        bool initialized;
    }
}

// These types can be used for both storage decoding AND RPC calls
// They provide automatic ABI encoding/decoding via alloy-sol-types

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;
    use alloy_sol_types::SolValue;

    #[test]
    fn test_reserves_abi_encoding() {
        let reserves = ReservesStorage {
            reserve0: 1000u128.into(),
            reserve1: 2000u128.into(),
            blockTimestampLast: 123456,
        };

        // Can encode to ABI bytes
        let encoded = reserves.abi_encode();

        // Can decode from ABI bytes
        let decoded = ReservesStorage::abi_decode(&encoded, true).unwrap();

        assert_eq!(decoded.reserve0, 1000u128.into());
        assert_eq!(decoded.reserve1, 2000u128.into());
        assert_eq!(decoded.blockTimestampLast, 123456);
    }

    #[test]
    fn test_slot0_abi_encoding() {
        let slot0 = Slot0Storage {
            sqrtPriceX96: U256::from(1000).try_into().unwrap(),
            tick: -100,
            observationIndex: 1,
            observationCardinality: 10,
            observationCardinalityNext: 20,
            feeProtocol: 5,
            unlocked: true,
        };

        let encoded = slot0.abi_encode();
        let decoded = Slot0Storage::abi_decode(&encoded, true).unwrap();

        assert_eq!(decoded.tick, -100);
        assert_eq!(decoded.unlocked, true);
    }
}
