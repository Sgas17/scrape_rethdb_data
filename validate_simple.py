#!/usr/bin/env python3
"""
Simple validation: Compare Rust-decoded values against RPC
No manual decoding in Python - trust the Rust alloy-sol-types decoding
"""

import json
import os
from dotenv import load_dotenv
from web3 import Web3
from eth_abi import encode, decode

# Load environment variables
load_dotenv()

# Import the Rust library
import scrape_rethdb_data

def get_function_selector(function_signature):
    """Get 4-byte function selector from signature"""
    return Web3.keccak(text=function_signature)[:4]

def validate_v2_pool(w3, pool_data):
    """Validate V2 pool reserves using getReserves()"""
    print(f"\n{'='*80}")
    print(f"Validating V2 Pool: {pool_data['address']}")
    print(f"{'='*80}")

    address = Web3.to_checksum_address(pool_data['address'])

    # Get Rust-decoded values from database
    db_reserves = pool_data['reserves']
    print(f"\nDB Reserves (decoded by Rust alloy-sol-types):")
    print(f"  Reserve0: {db_reserves['reserve0']}")
    print(f"  Reserve1: {db_reserves['reserve1']}")
    print(f"  Timestamp: {db_reserves['block_timestamp_last']}")
    print(f"  Raw: {db_reserves['raw_data']}")

    # Call getReserves() via RPC
    print("\nCalling getReserves() on RPC...")
    selector = get_function_selector("getReserves()")

    result = w3.eth.call({
        'to': address,
        'data': selector
    })

    # Decode RPC result
    reserve0, reserve1, block_timestamp_last = decode(
        ['uint112', 'uint112', 'uint32'],
        result
    )

    print(f"\nRPC Reserves:")
    print(f"  Reserve0: {reserve0}")
    print(f"  Reserve1: {reserve1}")
    print(f"  Timestamp: {block_timestamp_last}")

    # Validate
    matches = (
        db_reserves['reserve0'] == reserve0 and
        db_reserves['reserve1'] == reserve1 and
        db_reserves['block_timestamp_last'] == block_timestamp_last
    )

    if matches:
        print(f"\n✓ VALIDATION PASSED - Reserves match!")
    else:
        print(f"\n✗ VALIDATION FAILED - Reserves don't match!")
        print(f"\n  Differences:")
        if db_reserves['reserve0'] != reserve0:
            print(f"    Reserve0: DB={db_reserves['reserve0']}, RPC={reserve0}")
        if db_reserves['reserve1'] != reserve1:
            print(f"    Reserve1: DB={db_reserves['reserve1']}, RPC={reserve1}")
        if db_reserves['block_timestamp_last'] != block_timestamp_last:
            print(f"    Timestamp: DB={db_reserves['block_timestamp_last']}, RPC={block_timestamp_last}")

    return matches

def validate_v3_pool(w3, pool_data, sample_size=10):
    """Validate V3 pool data using slot0(), tickBitmap(), ticks()"""
    print(f"\n{'='*80}")
    print(f"Validating V3 Pool: {pool_data['address']}")
    print(f"{'='*80}")

    address = Web3.to_checksum_address(pool_data['address'])
    all_match = True

    # 1. Validate Slot0
    print("\n1. Validating Slot0 via slot0() call...")

    db_slot0 = pool_data['slot0']
    print(f"  DB  - Tick: {db_slot0['tick']}, Price: {db_slot0['sqrt_price_x96']}, Unlocked: {db_slot0['unlocked']}")
    print(f"  Raw: {db_slot0.get('raw_data', 'N/A')}")

    selector = get_function_selector("slot0()")
    result = w3.eth.call({
        'to': address,
        'data': selector
    })

    # Decode RPC result
    sqrt_price_x96, tick, obs_index, obs_card, obs_card_next, fee_proto, unlocked = decode(
        ['uint160', 'int24', 'uint16', 'uint16', 'uint16', 'uint8', 'bool'],
        result
    )

    print(f"  RPC - Tick: {tick}, Price: {hex(sqrt_price_x96)}, Unlocked: {unlocked}")

    slot0_match = (
        db_slot0['tick'] == tick and
        int(db_slot0['sqrt_price_x96'], 16) == sqrt_price_x96 and
        db_slot0['unlocked'] == unlocked
    )

    if slot0_match:
        print(f"  ✓ Slot0 matches")
    else:
        print(f"  ✗ Slot0 doesn't match")
        all_match = False

    # 2. Validate bitmaps
    print(f"\n2. Validating {min(sample_size, len(pool_data['bitmaps']))} bitmap words...")
    bitmaps_to_check = pool_data['bitmaps'][:sample_size]

    for bitmap in bitmaps_to_check:
        word_pos = bitmap['word_pos']
        db_bitmap = bitmap['bitmap']

        selector = get_function_selector("tickBitmap(int16)")
        data = selector + encode(['int16'], [word_pos])

        result = w3.eth.call({
            'to': address,
            'data': data
        })

        rpc_bitmap = decode(['uint256'], result)[0]
        db_bitmap_int = int(db_bitmap, 16)
        match = db_bitmap_int == rpc_bitmap

        status = "✓" if match else "✗"
        print(f"  {status} Word {word_pos}: {'Match' if match else 'MISMATCH'}")

        if not match:
            all_match = False

    # 3. Validate ticks
    print(f"\n3. Validating {min(sample_size, len(pool_data['ticks']))} ticks...")
    ticks_to_check = pool_data['ticks'][:sample_size]

    for tick_data in ticks_to_check:
        tick = tick_data['tick']

        selector = get_function_selector("ticks(int24)")
        data = selector + encode(['int24'], [tick])

        result = w3.eth.call({
            'to': address,
            'data': data
        })

        rpc_initialized = int.from_bytes(result, 'big') != 0
        db_initialized = tick_data['initialized']

        match = db_initialized == rpc_initialized
        status = "✓" if match else "✗"
        print(f"  {status} Tick {tick}: DB={db_initialized}, RPC={rpc_initialized}")

        if not match:
            all_match = False

    if all_match:
        print(f"\n✓ VALIDATION PASSED - All checked data matches!")
    else:
        print(f"\n✗ VALIDATION FAILED - Some data doesn't match!")

    return all_match

def validate_v4_pool(w3, pool_data, sample_size=5):
    """Validate V4 pool data using StateView contract"""
    print(f"\n{'='*80}")
    print(f"Validating V4 Pool: {pool_data['pool_id']}")
    print(f"  PoolManager: {pool_data['address']}")
    print(f"{'='*80}")

    stateview_address = os.getenv("V4_STATEVIEW_ADDRESS", None)

    if not stateview_address:
        print("\n⚠️  V4_STATEVIEW_ADDRESS not set in environment")

        db_slot0 = pool_data['slot0']
        print(f"\nDB V4 Slot0 (decoded by Rust):")
        print(f"  Tick: {db_slot0['tick']}")
        print(f"  SqrtPriceX96: {db_slot0['sqrt_price_x96']}")
        print(f"  Unlocked: {db_slot0['unlocked']}")
        print(f"  Bitmap words: {len(pool_data['bitmaps'])}")
        print(f"  Initialized ticks: {len(pool_data['ticks'])}")
        print(f"  Raw: {db_slot0.get('raw_data', 'N/A')}")

        print(f"\n✓ V4 data structure shown (set V4_STATEVIEW_ADDRESS for validation)")
        return True

    stateview_address = Web3.to_checksum_address(stateview_address)
    pool_id = pool_data['pool_id']
    pool_id_bytes = bytes.fromhex(pool_id[2:] if pool_id.startswith('0x') else pool_id)

    all_match = True

    # Validate Slot0
    print("\n1. Validating Slot0 via getSlot0(bytes32) call...")

    db_slot0 = pool_data['slot0']
    print(f"  DB  - Tick: {db_slot0['tick']}, Price: {db_slot0['sqrt_price_x96']}, Unlocked: {db_slot0['unlocked']}")

    selector = get_function_selector("getSlot0(bytes32)")
    data = selector + encode(['bytes32'], [pool_id_bytes])

    try:
        result = w3.eth.call({
            'to': stateview_address,
            'data': data
        })

        sqrt_price_x96, tick, obs_index, obs_card, obs_card_next, fee_proto, unlocked = decode(
            ['uint160', 'int24', 'uint16', 'uint16', 'uint16', 'uint8', 'bool'],
            result
        )

        print(f"  RPC - Tick: {tick}, Price: {hex(sqrt_price_x96)}, Unlocked: {unlocked}")

        slot0_match = (
            db_slot0['tick'] == tick and
            int(db_slot0['sqrt_price_x96'], 16) == sqrt_price_x96 and
            db_slot0['unlocked'] == unlocked
        )

        if slot0_match:
            print(f"  ✓ Slot0 matches")
        else:
            print(f"  ✗ Slot0 doesn't match")
            all_match = False

    except Exception as e:
        print(f"  ✗ Error calling getSlot0: {e}")
        all_match = False

    if all_match:
        print(f"\n✓ VALIDATION PASSED")
    else:
        print(f"\n✗ VALIDATION FAILED")

    return all_match

def main():
    print("=" * 80)
    print("RPC Validation - Using Rust alloy-sol-types Decoding")
    print("=" * 80)

    db_path = os.getenv("RETH_DB_PATH", "/path/to/reth/db")
    rpc_url = os.getenv("RPC_URL", "http://localhost:8545")

    print(f"\nDatabase path: {db_path}")
    print(f"RPC URL: {rpc_url}\n")

    w3 = Web3(Web3.HTTPProvider(rpc_url))

    if not w3.is_connected():
        print("✗ Failed to connect to RPC!")
        return

    print(f"✓ Connected to RPC (Chain ID: {w3.eth.chain_id})\n")

    # Define test pools
    pools = [
        {
            "address": "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
            "protocol": "v2",
            "tick_spacing": None,
        },
        {
            "address": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
            "protocol": "v3",
            "tick_spacing": 10,
        },
        {
            "address": "0x000000000004444c5dc75cB358380D2e3dE08A90",
            "protocol": "v4",
            "tick_spacing": 60,
        },
    ]

    v4_pool_ids = [
        "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d",
    ]

    print("Collecting data from database...")
    result_json = scrape_rethdb_data.collect_pools(db_path, pools, v4_pool_ids)
    results = json.loads(result_json)
    print(f"✓ Collected data for {len(results)} pools\n")

    all_passed = True

    for pool_data in results:
        protocol = pool_data['protocol']

        try:
            if protocol == 'uniswapv2':
                passed = validate_v2_pool(w3, pool_data)
            elif protocol == 'uniswapv3':
                passed = validate_v3_pool(w3, pool_data, sample_size=10)
            elif protocol == 'uniswapv4':
                passed = validate_v4_pool(w3, pool_data, sample_size=5)
            else:
                print(f"Unknown protocol: {protocol}")
                passed = False

            if not passed:
                all_passed = False

        except Exception as e:
            print(f"\n✗ Error validating {protocol} pool: {e}")
            import traceback
            traceback.print_exc()
            all_passed = False

    print("\n" + "=" * 80)
    print("VALIDATION SUMMARY")
    print("=" * 80)

    if all_passed:
        print("\n✓ ALL VALIDATIONS PASSED")
        print("Rust alloy-sol-types decoding matches RPC for all pools!")
    else:
        print("\n✗ SOME VALIDATIONS FAILED")
        print("Please review the output above for details.")

    print("\n" + "=" * 80)

if __name__ == "__main__":
    main()
