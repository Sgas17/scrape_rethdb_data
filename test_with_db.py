#!/usr/bin/env python3
"""Quick test to verify database access"""

import json
import os
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Import the Rust library
import scrape_rethdb_data

# Get database path
db_path = os.getenv("RETH_DB_PATH", "/path/to/reth/db")
print(f"Database path: {db_path}")
print(f"Testing database access...\n")

# Just test with one simple V2 pool
pools = [
    {
        "address": "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
        "protocol": "v2",
        "tick_spacing": None,
    },
]

try:
    result_json = scrape_rethdb_data.collect_pools(db_path, pools)
    results = json.loads(result_json)

    print("✓ SUCCESS! Database access working!")
    print(f"\nCollected data for pool: {results[0]['address']}")
    print(f"Protocol: {results[0]['protocol']}")

    if results[0].get('reserves'):
        reserves = results[0]['reserves']
        print(f"Reserve0: {reserves['reserve0']}")
        print(f"Reserve1: {reserves['reserve1']}")

except Exception as e:
    import traceback
    print(f"✗ ERROR: {e}")
    traceback.print_exc()
