#!/usr/bin/env python3
"""Test with fresh import"""
import sys
import os

# Force use of new .so file
sys.path.insert(0, '/tmp')
os.environ['PYTHONDONTWRITEBYTECODE'] = '1'

# Import directly from /tmp
import scrape_rethdb_data_new as scrape_rethdb_data
import json

db_path = "/var/lib/docker/volumes/eth-docker_reth-el-data/_data/db"

pools = [{
    "address": "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
    "protocol": "v2",
    "tick_spacing": None,
}]

print("Testing V2 reserve parsing...")
result_json = scrape_rethdb_data.collect_pools(db_path, pools, [])
results = json.loads(result_json)

pool = results[0]
reserves = pool['reserves']

print(f"\nRaw data: {reserves['raw_data']}")
print(f"Reserve0: {reserves['reserve0']}")
print(f"Reserve1: {reserves['reserve1']}")
print(f"Timestamp: {reserves['block_timestamp_last']}")

if reserves['reserve0'] == 0:
    print("\n❌ STILL GETTING ZEROS - Library not updated!")
else:
    print("\n✅ Values decoded successfully!")
