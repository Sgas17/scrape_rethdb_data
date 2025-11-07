#!/usr/bin/env python3
"""
Verify that data collected from Reth DB matches RPC contract calls

This test:
1. Gets slot0 from both DB and RPC (finds current tick)
2. Generates test ticks around nearest initializable tick
3. Collects tick data from DB using scrape_rethdb_data
4. Calls RPC to get the same tick data
5. Compares DB vs RPC results
6. Does the same for bitmaps

Run on a machine with both Reth DB access and RPC access
"""

import json
import os
from typing import List, Dict, Any
from dotenv import load_dotenv
from web3 import Web3
from eth_abi import encode

# Load environment variables
load_dotenv()

import scrape_rethdb_data

# V3 Pool ABI (minimal - only what we need)
V3_POOL_ABI = [
    {
        "inputs": [],
        "name": "slot0",
        "outputs": [
            {"internalType": "uint160", "name": "sqrtPriceX96", "type": "uint160"},
            {"internalType": "int24", "name": "tick", "type": "int24"},
            {"internalType": "uint16", "name": "observationIndex", "type": "uint16"},
            {"internalType": "uint16", "name": "observationCardinality", "type": "uint16"},
            {"internalType": "uint16", "name": "observationCardinalityNext", "type": "uint16"},
            {"internalType": "uint8", "name": "feeProtocol", "type": "uint8"},
            {"internalType": "bool", "name": "unlocked", "type": "bool"}
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "tickSpacing",
        "outputs": [{"internalType": "int24", "name": "", "type": "int24"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [{"internalType": "int24", "name": "tick", "type": "int24"}],
        "name": "ticks",
        "outputs": [
            {"internalType": "uint128", "name": "liquidityGross", "type": "uint128"},
            {"internalType": "int128", "name": "liquidityNet", "type": "int128"},
            {"internalType": "uint256", "name": "feeGrowthOutside0X128", "type": "uint256"},
            {"internalType": "uint256", "name": "feeGrowthOutside1X128", "type": "uint256"},
            {"internalType": "int56", "name": "tickCumulativeOutside", "type": "int56"},
            {"internalType": "uint160", "name": "secondsPerLiquidityOutsideX128", "type": "uint160"},
            {"internalType": "uint32", "name": "secondsOutside", "type": "uint32"},
            {"internalType": "bool", "name": "initialized", "type": "bool"}
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [{"internalType": "int16", "name": "wordPosition", "type": "int16"}],
        "name": "tickBitmap",
        "outputs": [{"internalType": "uint256", "name": "", "type": "uint256"}],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [],
        "name": "liquidity",
        "outputs": [{"internalType": "uint128", "name": "", "type": "uint128"}],
        "stateMutability": "view",
        "type": "function"
    }
]

# V4 StateView ABI (minimal)
V4_STATEVIEW_ABI = [
    {
        "inputs": [{"internalType": "bytes32", "name": "poolId", "type": "bytes32"}],
        "name": "getSlot0",
        "outputs": [
            {"internalType": "uint160", "name": "sqrtPriceX96", "type": "uint160"},
            {"internalType": "int24", "name": "tick", "type": "int24"},
            {"internalType": "uint24", "name": "protocolFee", "type": "uint24"},
            {"internalType": "uint24", "name": "lpFee", "type": "uint24"}
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [
            {"internalType": "bytes32", "name": "poolId", "type": "bytes32"},
            {"internalType": "int24", "name": "tick", "type": "int24"}
        ],
        "name": "getTickLiquidity",
        "outputs": [
            {"internalType": "uint128", "name": "liquidityGross", "type": "uint128"},
            {"internalType": "int128", "name": "liquidityNet", "type": "int128"}
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [
            {"internalType": "bytes32", "name": "poolId", "type": "bytes32"},
            {"internalType": "int16", "name": "wordPosition", "type": "int16"}
        ],
        "name": "getTickBitmap",
        "outputs": [{"internalType": "uint256", "name": "", "type": "uint256"}],
        "stateMutability": "view",
        "type": "function"
    }
]


def tick_to_word_pos(tick: int, tick_spacing: int) -> int:
    """Calculate word position for a tick given tick spacing"""
    compressed = tick // tick_spacing
    return compressed >> 8


def find_nearest_initializable_tick(current_tick: int, tick_spacing: int) -> int:
    """Find the nearest initializable tick (rounded down to tick spacing boundary)"""
    tick_remainder = current_tick % tick_spacing

    if tick_remainder == 0:
        return current_tick
    elif tick_remainder > 0:
        return current_tick - tick_remainder  # Round down
    else:
        return current_tick - (tick_spacing + tick_remainder)  # Round down for negative


def verify_v3_db_vs_rpc(
    w3: Web3,
    db_path: str,
    pool_address: str
) -> Dict[str, Any]:
    """Verify V3 pool data from DB matches RPC"""

    print("\n" + "=" * 80)
    print("V3 DATABASE vs RPC VERIFICATION")
    print("=" * 80)
    print(f"Pool: {pool_address}")
    print(f"DB Path: {db_path}")

    # Create contract instance
    pool = w3.eth.contract(address=Web3.to_checksum_address(pool_address), abi=V3_POOL_ABI)

    # Step 1: Get slot0 from RPC to find current tick
    print("\n--- Step 1: Get Current Tick from RPC ---")
    slot0_rpc = pool.functions.slot0().call()
    current_tick = slot0_rpc[1]  # tick is second element
    tick_spacing = pool.functions.tickSpacing().call()

    print(f"Current tick: {current_tick}")
    print(f"Tick spacing: {tick_spacing}")

    # Step 2: Find nearest initializable tick
    print("\n--- Step 2: Find Nearest Initializable Tick ---")
    nearest_tick = find_nearest_initializable_tick(current_tick, tick_spacing)
    print(f"Nearest initializable tick: {nearest_tick}")

    # Generate test ticks
    test_ticks = [nearest_tick + (tick_spacing * n) for n in range(-5, 6)]
    print(f"Testing {len(test_ticks)} ticks: {test_ticks[0]} to {test_ticks[-1]}")

    # Generate word positions for bitmaps
    word_positions = sorted(set(tick_to_word_pos(t, tick_spacing) for t in test_ticks))
    print(f"Testing {len(word_positions)} bitmap words: {word_positions}")

    # Step 3: Collect data from DB
    print("\n--- Step 3: Collect Data from Reth DB ---")
    pools_input = [{
        "address": pool_address,
        "protocol": "v3",
        "tick_spacing": tick_spacing,
        "slot0_only": False,
    }]

    result_json = scrape_rethdb_data.collect_pools(db_path, pools_input, [])
    db_data = json.loads(result_json)[0]

    print(f"Collected {len(db_data['ticks'])} ticks from DB")
    print(f"Collected {len(db_data['bitmaps'])} bitmaps from DB")

    # Step 4: Compare slot0
    print("\n--- Step 4: Compare Slot0 ---")
    db_slot0 = db_data['slot0']

    sqrt_match = int(db_slot0['sqrt_price_x96'], 0) == slot0_rpc[0]
    tick_match = db_slot0['tick'] == slot0_rpc[1]

    print(f"sqrtPriceX96 match: {sqrt_match}")
    print(f"  DB:  {db_slot0['sqrt_price_x96']}")
    print(f"  RPC: {slot0_rpc[0]}")
    print(f"tick match: {tick_match}")
    print(f"  DB:  {db_slot0['tick']}")
    print(f"  RPC: {slot0_rpc[1]}")

    # Step 5: Compare ticks
    print("\n--- Step 5: Compare Ticks ---")
    tick_matches = 0
    tick_mismatches = 0

    # Create lookup dictionary for DB ticks
    db_ticks_lookup = {t['tick']: t for t in db_data['ticks']}

    for tick in test_ticks:
        rpc_tick = pool.functions.ticks(tick).call()

        if tick in db_ticks_lookup:
            db_tick = db_ticks_lookup[tick]
            gross_match = db_tick['liquidity_gross'] == rpc_tick[0]
            net_match = db_tick['liquidity_net'] == rpc_tick[1]

            if gross_match and net_match:
                tick_matches += 1
            else:
                tick_mismatches += 1
                print(f"  MISMATCH at tick {tick}:")
                print(f"    liquidityGross: DB={db_tick['liquidity_gross']}, RPC={rpc_tick[0]}")
                print(f"    liquidityNet: DB={db_tick['liquidity_net']}, RPC={rpc_tick[1]}")
        else:
            # Tick not in DB - verify it's uninitialized on RPC
            if rpc_tick[0] == 0 and rpc_tick[1] == 0:
                tick_matches += 1
            else:
                tick_mismatches += 1
                print(f"  MISSING tick {tick} with liquidity: gross={rpc_tick[0]}, net={rpc_tick[1]}")

    print(f"Tick comparisons: {tick_matches} matches, {tick_mismatches} mismatches")

    # Step 6: Compare bitmaps
    print("\n--- Step 6: Compare Bitmaps ---")
    bitmap_matches = 0
    bitmap_mismatches = 0

    # Create lookup dictionary for DB bitmaps
    db_bitmaps_lookup = {b['word_pos']: b for b in db_data['bitmaps']}

    for word_pos in word_positions:
        rpc_bitmap = pool.functions.tickBitmap(word_pos).call()

        if word_pos in db_bitmaps_lookup:
            db_bitmap = db_bitmaps_lookup[word_pos]
            bitmap_match = int(db_bitmap['bitmap'], 0) == rpc_bitmap

            if bitmap_match:
                bitmap_matches += 1
            else:
                bitmap_mismatches += 1
                print(f"  MISMATCH at word {word_pos}:")
                print(f"    DB:  {hex(int(db_bitmap['bitmap']))}")
                print(f"    RPC: {hex(rpc_bitmap)}")
        else:
            # Bitmap not in DB - verify it's zero on RPC
            if rpc_bitmap == 0:
                bitmap_matches += 1
            else:
                bitmap_mismatches += 1
                print(f"  MISSING bitmap word {word_pos}: {hex(rpc_bitmap)}")

    print(f"Bitmap comparisons: {bitmap_matches} matches, {bitmap_mismatches} mismatches")

    # Final verdict
    print("\n" + "=" * 80)
    all_match = (sqrt_match and tick_match and
                 tick_mismatches == 0 and bitmap_mismatches == 0)

    if all_match:
        print("✓ VERIFICATION PASSED - All DB data matches RPC!")
    else:
        print("✗ VERIFICATION FAILED - Found mismatches")

    print("=" * 80)

    return {
        "pool": pool_address,
        "protocol": "v3",
        "passed": all_match,
        "slot0_match": sqrt_match and tick_match,
        "tick_matches": tick_matches,
        "tick_mismatches": tick_mismatches,
        "bitmap_matches": bitmap_matches,
        "bitmap_mismatches": bitmap_mismatches,
    }


def verify_v4_db_vs_rpc(
    w3: Web3,
    db_path: str,
    pool_manager: str,
    pool_id: str,
    tick_spacing: int
) -> Dict[str, Any]:
    """Verify V4 pool data from DB matches RPC"""

    print("\n" + "=" * 80)
    print("V4 DATABASE vs RPC VERIFICATION")
    print("=" * 80)
    print(f"Pool Manager: {pool_manager}")
    print(f"Pool ID: {pool_id}")
    print(f"DB Path: {db_path}")

    # V4 StateView contract address
    stateview_address = "0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227"
    stateview = w3.eth.contract(
        address=Web3.to_checksum_address(stateview_address),
        abi=V4_STATEVIEW_ABI
    )

    # Step 1: Get slot0 from RPC to find current tick
    print("\n--- Step 1: Get Current Tick from RPC ---")
    slot0_rpc = stateview.functions.getSlot0(pool_id).call()
    current_tick = slot0_rpc[1]  # tick is second element

    print(f"Current tick: {current_tick}")
    print(f"Tick spacing: {tick_spacing}")

    # Step 2: Find nearest initializable tick
    print("\n--- Step 2: Find Nearest Initializable Tick ---")
    nearest_tick = find_nearest_initializable_tick(current_tick, tick_spacing)
    print(f"Nearest initializable tick: {nearest_tick}")

    # Generate test ticks
    test_ticks = [nearest_tick + (tick_spacing * n) for n in range(-5, 6)]
    print(f"Testing {len(test_ticks)} ticks: {test_ticks[0]} to {test_ticks[-1]}")

    # Generate word positions for bitmaps
    word_positions = sorted(set(tick_to_word_pos(t, tick_spacing) for t in test_ticks))
    print(f"Testing {len(word_positions)} bitmap words: {word_positions}")

    # Step 3: Collect data from DB
    print("\n--- Step 3: Collect Data from Reth DB ---")
    pools_input = [{
        "address": pool_manager,
        "protocol": "v4",
        "tick_spacing": tick_spacing,
        "slot0_only": False,
    }]

    result_json = scrape_rethdb_data.collect_pools(db_path, pools_input, [pool_id])
    db_data = json.loads(result_json)[0]

    print(f"Collected {len(db_data['ticks'])} ticks from DB")
    print(f"Collected {len(db_data['bitmaps'])} bitmaps from DB")

    # Step 4: Compare slot0
    print("\n--- Step 4: Compare Slot0 ---")
    db_slot0 = db_data['slot0']

    sqrt_match = int(db_slot0['sqrt_price_x96'], 0) == slot0_rpc[0]
    tick_match = db_slot0['tick'] == slot0_rpc[1]

    print(f"sqrtPriceX96 match: {sqrt_match}")
    print(f"  DB:  {db_slot0['sqrt_price_x96']}")
    print(f"  RPC: {slot0_rpc[0]}")
    print(f"tick match: {tick_match}")
    print(f"  DB:  {db_slot0['tick']}")
    print(f"  RPC: {slot0_rpc[1]}")

    # Step 5: Compare ticks
    print("\n--- Step 5: Compare Ticks ---")
    tick_matches = 0
    tick_mismatches = 0

    # Create lookup dictionary for DB ticks
    db_ticks_lookup = {t['tick']: t for t in db_data['ticks']}

    for tick in test_ticks:
        rpc_tick = stateview.functions.getTickLiquidity(pool_id, tick).call()

        if tick in db_ticks_lookup:
            db_tick = db_ticks_lookup[tick]
            gross_match = db_tick['liquidity_gross'] == rpc_tick[0]
            net_match = db_tick['liquidity_net'] == rpc_tick[1]

            if gross_match and net_match:
                tick_matches += 1
            else:
                tick_mismatches += 1
                print(f"  MISMATCH at tick {tick}:")
                print(f"    liquidityGross: DB={db_tick['liquidity_gross']}, RPC={rpc_tick[0]}")
                print(f"    liquidityNet: DB={db_tick['liquidity_net']}, RPC={rpc_tick[1]}")
        else:
            # Tick not in DB - verify it's uninitialized on RPC
            if rpc_tick[0] == 0 and rpc_tick[1] == 0:
                tick_matches += 1
            else:
                tick_mismatches += 1
                print(f"  MISSING tick {tick} with liquidity: gross={rpc_tick[0]}, net={rpc_tick[1]}")

    print(f"Tick comparisons: {tick_matches} matches, {tick_mismatches} mismatches")

    # Step 6: Compare bitmaps
    print("\n--- Step 6: Compare Bitmaps ---")
    bitmap_matches = 0
    bitmap_mismatches = 0

    # Create lookup dictionary for DB bitmaps
    db_bitmaps_lookup = {b['word_pos']: b for b in db_data['bitmaps']}

    for word_pos in word_positions:
        rpc_bitmap = stateview.functions.getTickBitmap(pool_id, word_pos).call()

        if word_pos in db_bitmaps_lookup:
            db_bitmap = db_bitmaps_lookup[word_pos]
            bitmap_match = int(db_bitmap['bitmap'], 0) == rpc_bitmap

            if bitmap_match:
                bitmap_matches += 1
            else:
                bitmap_mismatches += 1
                print(f"  MISMATCH at word {word_pos}:")
                print(f"    DB:  {hex(int(db_bitmap['bitmap']))}")
                print(f"    RPC: {hex(rpc_bitmap)}")
        else:
            # Bitmap not in DB - verify it's zero on RPC
            if rpc_bitmap == 0:
                bitmap_matches += 1
            else:
                bitmap_mismatches += 1
                print(f"  MISSING bitmap word {word_pos}: {hex(rpc_bitmap)}")

    print(f"Bitmap comparisons: {bitmap_matches} matches, {bitmap_mismatches} mismatches")

    # Final verdict
    print("\n" + "=" * 80)
    all_match = (sqrt_match and tick_match and
                 tick_mismatches == 0 and bitmap_mismatches == 0)

    if all_match:
        print("✓ VERIFICATION PASSED - All DB data matches RPC!")
    else:
        print("✗ VERIFICATION FAILED - Found mismatches")

    print("=" * 80)

    return {
        "pool": pool_id,
        "protocol": "v4",
        "passed": all_match,
        "slot0_match": sqrt_match and tick_match,
        "tick_matches": tick_matches,
        "tick_mismatches": tick_mismatches,
        "bitmap_matches": bitmap_matches,
        "bitmap_mismatches": bitmap_mismatches,
    }


def test_slot0_only_mode(w3: Web3, db_path: str) -> Dict[str, Any]:
    """Test slot0_only flag - should return only slot0 + liquidity, no ticks/bitmaps"""
    print("\n" + "=" * 80)
    print("SLOT0_ONLY MODE TEST")
    print("=" * 80)

    # Test V3 pool
    v3_pool = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
    print(f"\nTesting V3 Pool: {v3_pool}")
    print(f"DB Path: {db_path}")

    # Get RPC data
    pool = w3.eth.contract(address=w3.to_checksum_address(v3_pool), abi=V3_POOL_ABI)
    slot0_rpc = pool.functions.slot0().call()
    liquidity_rpc = pool.functions.liquidity().call()
    tick_spacing = pool.functions.tickSpacing().call()

    print(f"\n--- RPC Data ---")
    print(f"Tick: {slot0_rpc[1]}")
    print(f"sqrtPriceX96: {slot0_rpc[0]}")
    print(f"Liquidity: {liquidity_rpc}")

    # Collect with slot0_only=True
    print(f"\n--- Collecting with slot0_only=True ---")
    pools = [{
        "address": v3_pool,
        "protocol": "v3",
        "tick_spacing": tick_spacing,
        "slot0_only": True
    }]

    result_json = scrape_rethdb_data.collect_pools(db_path, pools, [])
    db_data = json.loads(result_json)[0]

    # Verify structure
    print(f"✓ Data collected")
    print(f"  Has slot0: {'slot0' in db_data}")
    print(f"  Has liquidity: {'liquidity' in db_data}")
    print(f"  Ticks count: {len(db_data.get('ticks', []))}")
    print(f"  Bitmaps count: {len(db_data.get('bitmaps', []))}")

    # Check that slot0_only mode works correctly
    has_slot0 = 'slot0' in db_data
    has_liquidity = 'liquidity' in db_data
    no_ticks = len(db_data.get('ticks', [])) == 0
    no_bitmaps = len(db_data.get('bitmaps', [])) == 0

    structure_valid = has_slot0 and has_liquidity and no_ticks and no_bitmaps

    if not structure_valid:
        print("\n✗ FAILED: slot0_only mode should return only slot0 + liquidity")
        return {'passed': False, 'mode': 'slot0_only'}

    print(f"\n--- Comparing Values ---")

    # Compare slot0
    db_slot0 = db_data['slot0']
    sqrt_match = int(db_slot0['sqrt_price_x96'], 0) == slot0_rpc[0]
    tick_match = db_slot0['tick'] == slot0_rpc[1]

    print(f"sqrtPriceX96 match: {sqrt_match}")
    print(f"  DB:  {db_slot0['sqrt_price_x96']}")
    print(f"  RPC: {slot0_rpc[0]}")
    print(f"tick match: {tick_match}")
    print(f"  DB:  {db_slot0['tick']}")
    print(f"  RPC: {slot0_rpc[1]}")

    # Compare liquidity
    liquidity_match = db_data['liquidity'] == liquidity_rpc
    print(f"liquidity match: {liquidity_match}")
    print(f"  DB:  {db_data['liquidity']}")
    print(f"  RPC: {liquidity_rpc}")

    all_match = sqrt_match and tick_match and liquidity_match

    print("\n" + "=" * 80)
    if all_match and structure_valid:
        print("✓ SLOT0_ONLY MODE TEST PASSED")
    else:
        print("✗ SLOT0_ONLY MODE TEST FAILED")
    print("=" * 80)

    return {
        'passed': all_match and structure_valid,
        'mode': 'slot0_only',
        'sqrt_match': sqrt_match,
        'tick_match': tick_match,
        'liquidity_match': liquidity_match,
        'structure_valid': structure_valid
    }


def main():
    # Configuration
    db_path = os.getenv("RETH_DB_PATH")
    rpc_url = os.getenv("RPC_URL", "http://localhost:8545")

    if not db_path:
        raise ValueError("RETH_DB_PATH environment variable not set")

    print("=" * 80)
    print("RETH DATABASE vs RPC VERIFICATION TEST (Python)")
    print("=" * 80)
    print(f"RPC URL: {rpc_url}")
    print(f"DB Path: {db_path}")

    # Initialize Web3
    w3 = Web3(Web3.HTTPProvider(rpc_url))

    if not w3.is_connected():
        raise ConnectionError(f"Failed to connect to RPC at {rpc_url}")

    print(f"Connected to chain ID: {w3.eth.chain_id}")

    results = []

    # Test V3 Pool
    v3_pool = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"  # USDC/WETH 0.05%
    result = verify_v3_db_vs_rpc(w3, db_path, v3_pool)
    results.append(result)

    # Test V4 Pool
    v4_pool_manager = "0x000000000004444c5dc75cB358380D2e3dE08A90"
    v4_pool_id = "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d"
    v4_tick_spacing = 60

    result = verify_v4_db_vs_rpc(
        w3,
        db_path,
        v4_pool_manager,
        v4_pool_id,
        v4_tick_spacing
    )
    results.append(result)

    # Test slot0_only mode
    result = test_slot0_only_mode(w3, db_path)
    results.append(result)

    # Summary
    print("\n" + "=" * 80)
    print("FINAL SUMMARY")
    print("=" * 80)

    for result in results:
        status = "✓ PASSED" if result['passed'] else "✗ FAILED"

        # Handle slot0_only test differently
        if result.get('mode') == 'slot0_only':
            print(f"\nSlot0_Only Mode Test: {status}")
            print(f"  Structure valid: {result['structure_valid']}")
            print(f"  sqrtPriceX96 match: {result['sqrt_match']}")
            print(f"  Tick match: {result['tick_match']}")
            print(f"  Liquidity match: {result['liquidity_match']}")
        else:
            print(f"\n{result['protocol'].upper()} Pool: {status}")
            print(f"  Pool: {result['pool']}")
            print(f"  Slot0 match: {result['slot0_match']}")
            print(f"  Tick comparisons: {result['tick_matches']} matches, {result['tick_mismatches']} mismatches")
            print(f"  Bitmap comparisons: {result['bitmap_matches']} matches, {result['bitmap_mismatches']} mismatches")

    all_passed = all(r['passed'] for r in results)
    print("\n" + "=" * 80)
    if all_passed:
        print("✓ ALL TESTS PASSED!")
    else:
        print("✗ SOME TESTS FAILED")
    print("=" * 80)

    return 0 if all_passed else 1


if __name__ == "__main__":
    exit(main())
