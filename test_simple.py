#!/usr/bin/env python3
import json
import scrape_rethdb_data

# Test with just V3 and V2 pools (no V4)
pools = [
    {
        "address": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
        "protocol": "v3",
        "tick_spacing": 10,
    },
    {
        "address": "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
        "protocol": "v2",
        "tick_spacing": None,
    },
]

print("Testing with V2 and V3 pools only...")
print(f"Pool 1: {pools[0]}")
print(f"Pool 2: {pools[1]}")

try:
    result_json = scrape_rethdb_data.collect_pools("/path/to/reth/db", pools)
    print("Success!")
    print(result_json)
except Exception as e:
    import traceback
    print(f"Error: {e}")
    traceback.print_exc()
