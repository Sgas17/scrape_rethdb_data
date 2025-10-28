#!/usr/bin/env python3
"""Test slot0_only mode for fast liquidity filtering"""

import json
import os
import time
from dotenv import load_dotenv

load_dotenv('/home/sam-sullivan/scrape_rethdb_data/.env')

import scrape_rethdb_data

db_path = os.getenv("RETH_DB_PATH")

# Test pools
v3_pool = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"  # USDC/WETH
v4_pool_manager = "0x000000000004444c5dc75cB358380D2e3dE08A90"
v4_pool_id = "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d"

print("="*80)
print("Testing slot0_only Mode - Fast Liquidity Filtering")
print("="*80)
print()

# Test 1: V3 with slot0_only=False (full data)
print("Test 1: V3 Pool - Full Data (slot0_only=False)")
print("-"*80)

pools_full = [{
    "address": v3_pool,
    "protocol": "v3",
    "tick_spacing": 10,
    "slot0_only": False,
}]

start = time.time()
result_json = scrape_rethdb_data.collect_pools(db_path, pools_full, [])
results = json.loads(result_json)
elapsed_full = time.time() - start

pool_data = results[0]
print(f"Time: {elapsed_full*1000:.2f} ms")
print(f"Slot0 tick: {pool_data['slot0']['tick']}")
print(f"Liquidity: {pool_data.get('liquidity', 'N/A')}")
print(f"Bitmaps collected: {len(pool_data['bitmaps'])}")
print(f"Ticks collected: {len(pool_data['ticks'])}")
print()

# Test 2: V3 with slot0_only=True (fast mode)
print("Test 2: V3 Pool - Slot0 Only (slot0_only=True)")
print("-"*80)

pools_fast = [{
    "address": v3_pool,
    "protocol": "v3",
    "tick_spacing": 10,
    "slot0_only": True,
}]

start = time.time()
result_json = scrape_rethdb_data.collect_pools(db_path, pools_fast, [])
results = json.loads(result_json)
elapsed_fast = time.time() - start

pool_data = results[0]
print(f"Time: {elapsed_fast*1000:.2f} ms")
print(f"Slot0 tick: {pool_data['slot0']['tick']}")
print(f"Liquidity: {pool_data.get('liquidity', 'N/A')}")
print(f"Bitmaps collected: {len(pool_data['bitmaps'])}")
print(f"Ticks collected: {len(pool_data['ticks'])}")
print()

speedup = elapsed_full / elapsed_fast
print(f"Speedup: {speedup:.1f}x faster")
print()

# Test 3: V4 with slot0_only=True
print("Test 3: V4 Pool - Slot0 Only (slot0_only=True)")
print("-"*80)

pools_v4 = [{
    "address": v4_pool_manager,
    "protocol": "v4",
    "tick_spacing": 60,
    "slot0_only": True,
}]
v4_pool_ids = [v4_pool_id]

start = time.time()
result_json = scrape_rethdb_data.collect_pools(db_path, pools_v4, v4_pool_ids)
results = json.loads(result_json)
elapsed_v4 = time.time() - start

pool_data = results[0]
print(f"Time: {elapsed_v4*1000:.2f} ms")
print(f"Pool ID: {pool_data['pool_id']}")
print(f"Slot0 tick: {pool_data['slot0']['tick']}")
print(f"Liquidity: {pool_data.get('liquidity', 'N/A')}")
print(f"Bitmaps collected: {len(pool_data['bitmaps'])}")
print(f"Ticks collected: {len(pool_data['ticks'])}")
print()

# Test 4: Batch collection with slot0_only
print("Test 4: Batch Collection - Multiple Pools with slot0_only=True")
print("-"*80)

pools_batch = [
    {
        "address": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",  # USDC/WETH 0.05%
        "protocol": "v3",
        "tick_spacing": 10,
        "slot0_only": True,
    },
    {
        "address": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8",  # USDC/WETH 0.3%
        "protocol": "v3",
        "tick_spacing": 60,
        "slot0_only": True,
    },
    {
        "address": "0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36",  # WETH/USDT 0.3%
        "protocol": "v3",
        "tick_spacing": 60,
        "slot0_only": True,
    },
]

start = time.time()
result_json = scrape_rethdb_data.collect_pools(db_path, pools_batch, [])
results = json.loads(result_json)
elapsed_batch = time.time() - start

print(f"Time: {elapsed_batch*1000:.2f} ms for {len(results)} pools")
print(f"Average: {elapsed_batch*1000/len(results):.2f} ms per pool")
print()

for i, pool in enumerate(results, 1):
    print(f"Pool {i}: {pool['address']}")
    print(f"  Tick: {pool['slot0']['tick']}")
    print(f"  Liquidity: {pool.get('liquidity', 'N/A')}")
    print()

print("="*80)
print("Summary")
print("="*80)
print(f"slot0_only mode is {speedup:.1f}x faster than full collection")
print(f"Use slot0_only=True for liquidity filtering before full collection")
