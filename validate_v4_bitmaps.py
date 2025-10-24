#!/usr/bin/env python3
"""Validate V4 pool bitmaps and ticks against RPC"""

import json
import os
from dotenv import load_dotenv
from web3 import Web3
from eth_abi import encode, decode

load_dotenv('/home/sam-sullivan/scrape_rethdb_data/.env')

import scrape_rethdb_data

def get_function_selector(function_signature):
    return Web3.keccak(text=function_signature)[:4]

# Setup
rpc_url = os.getenv("RPC_URL", "http://localhost:8545")
w3 = Web3(Web3.HTTPProvider(rpc_url))

db_path = os.getenv("RETH_DB_PATH")
pool_manager = "0x000000000004444c5dc75cB358380D2e3dE08A90"
pool_id = "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d"

# StateView contract for querying V4
stateview_address = Web3.to_checksum_address(os.getenv("V4_STATEVIEW_ADDRESS"))

print("="*80)
print("V4 Pool Bitmap and Tick Validation")
print("="*80)
print(f"PoolManager: {pool_manager}")
print(f"PoolId: {pool_id}")
print(f"StateView: {stateview_address}")
print()

# Collect from database
pools = [{
    "address": pool_manager,
    "protocol": "v4",
    "tick_spacing": 60,
}]
v4_pool_ids = [pool_id]

print("Collecting data from database...")
result_json = scrape_rethdb_data.collect_pools(db_path, pools, v4_pool_ids)
results = json.loads(result_json)
pool_data = results[0]

print(f"✓ DB found {len(pool_data['bitmaps'])} bitmap words")
print(f"✓ DB found {len(pool_data['ticks'])} initialized ticks")
print()

# Validate bitmaps
print("="*80)
print("Validating Bitmaps")
print("="*80)

pool_id_bytes = bytes.fromhex(pool_id[2:])
all_bitmaps_match = True

num_to_check = min(10, len(pool_data['bitmaps']))
print(f"Checking first {num_to_check} bitmap words...\n")

for i, bitmap in enumerate(pool_data['bitmaps'][:num_to_check]):
    word_pos = bitmap['word_pos']
    db_bitmap_hex = bitmap['bitmap']
    db_bitmap_int = int(db_bitmap_hex, 16)

    # Query RPC via StateView getTickBitmap(poolId, wordPos)
    selector = get_function_selector("getTickBitmap(bytes32,int16)")
    data = selector + encode(['bytes32', 'int16'], [pool_id_bytes, word_pos])

    result = w3.eth.call({
        'to': stateview_address,
        'data': data
    })
    rpc_bitmap = decode(['uint256'], result)[0]

    match = "✓" if db_bitmap_int == rpc_bitmap else "✗"
    print(f"{match} Word {word_pos}:")
    print(f"  DB:  {db_bitmap_hex}")
    print(f"  RPC: 0x{rpc_bitmap:064x}")

    if db_bitmap_int != rpc_bitmap:
        print(f"  DB has {bin(db_bitmap_int).count('1')} bits set")
        print(f"  RPC has {bin(rpc_bitmap).count('1')} bits set")
        all_bitmaps_match = False
    print()

if all_bitmaps_match:
    print(f"✓✓✓ All {num_to_check} bitmap words match!\n")
else:
    print(f"✗ Some bitmap words don't match\n")

# Validate ticks
print("="*80)
print("Validating Ticks")
print("="*80)

all_ticks_match = True

num_to_check = min(15, len(pool_data['ticks']))
print(f"Checking first {num_to_check} ticks...\n")

for i, tick_data in enumerate(pool_data['ticks'][:num_to_check]):
    tick = tick_data['tick']
    db_initialized = tick_data['initialized']

    # Query RPC via StateView getTickLiquidity(poolId, tick)
    # Returns (liquidityGross, liquidityNet)
    # If liquidityGross > 0, tick is initialized
    selector = get_function_selector("getTickLiquidity(bytes32,int24)")
    data = selector + encode(['bytes32', 'int24'], [pool_id_bytes, tick])

    result = w3.eth.call({
        'to': stateview_address,
        'data': data
    })

    liquidity_gross, liquidity_net = decode(['uint128', 'int128'], result)
    rpc_initialized = liquidity_gross > 0

    match = "✓" if db_initialized == rpc_initialized else "✗"
    print(f"{match} Tick {tick}: DB={db_initialized}, RPC={rpc_initialized}")

    if db_initialized != rpc_initialized:
        all_ticks_match = False

print()

if all_ticks_match:
    print(f"✓✓✓ All {num_to_check} ticks match!\n")
else:
    print(f"✗ Some ticks don't match\n")

# Final summary
print("="*80)
print("SUMMARY")
print("="*80)

if all_bitmaps_match and all_ticks_match:
    print("✓✓✓ ALL VALIDATIONS PASSED!")
    print(f"  - {len(pool_data['bitmaps'])} bitmap words")
    print(f"  - {len(pool_data['ticks'])} initialized ticks")
else:
    print("✗ SOME VALIDATIONS FAILED")
    if not all_bitmaps_match:
        print("  - Bitmap mismatches detected")
    if not all_ticks_match:
        print("  - Tick mismatches detected")
