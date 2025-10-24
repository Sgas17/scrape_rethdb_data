#!/usr/bin/env python3
"""
Example of using scrape_rethdb_data from Python

Installation:
  1. Install maturin: uv add --dev maturin
  2. Build and install the Python module: uv run maturin develop --features python
  3. Set RETH_DB_PATH in .env file
  4. Run this script: uv run python python_example.py

Or build a wheel:
  uv run maturin build --features python --release
  pip install target/wheels/scrape_rethdb_data-*.whl
"""

import json
import os
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Import the Rust library
import scrape_rethdb_data

def main():
    print("=" * 80)
    print("Uniswap Pool Data Collection - Python Example")
    print("=" * 80)

    # Get database path from environment
    db_path = os.getenv("RETH_DB_PATH", "/path/to/reth/db")
    print(f"\nDatabase path: {db_path}")

    # Define pools to collect data from
    pools = [
        # UniswapV3 pools with different tick spacings
        {
            "address": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
            "protocol": "v3",
            "tick_spacing": 10,
        },
        # UniswapV2 pool
        {
            "address": "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
            "protocol": "v2",
            "tick_spacing": None,
        },
	# UniswapV4 pool - requires pool_id to be passed separately
	{
	    "address": "0x000000000004444c5dc75cB358380D2e3dE08A90",
	    "protocol": "v4",
	    "tick_spacing": 60,
	},
    ]

    # V4 pool IDs (must be in same order as V4 pools in the list above)
    v4_pool_ids = [
        "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d",
    ]

    print(f"\nCollecting data for {len(pools)} pools...\n")

    try:
        # Collect data (returns JSON string)
        result_json = scrape_rethdb_data.collect_pools(db_path, pools, v4_pool_ids)

        # Parse JSON
        results = json.loads(result_json)

        # Display results
        for idx, pool_data in enumerate(results):
            print(f"Pool {idx + 1} ({pool_data['address']}):")
            print(f"  Protocol: {pool_data['protocol']}")

            # Display pool_id for V4 pools
            if pool_data.get('pool_id'):
                print(f"  Pool ID: {pool_data['pool_id']}")

            protocol = pool_data['protocol']

            if protocol == 'uniswapv2':
                if pool_data.get('reserves'):
                    reserves = pool_data['reserves']
                    print(f"  Reserve0: {reserves['reserve0']}")
                    print(f"  Reserve1: {reserves['reserve1']}")
                    print(f"  Block Timestamp: {reserves['block_timestamp_last']}")

            elif protocol in ['uniswapv3', 'uniswapv4']:
                if pool_data.get('slot0'):
                    slot0 = pool_data['slot0']
                    print(f"  Current Tick: {slot0['tick']}")
                    print(f"  Sqrt Price X96: {slot0['sqrt_price_x96']}")
                    print(f"  Unlocked: {slot0['unlocked']}")

                ticks = pool_data.get('ticks', [])
                bitmaps = pool_data.get('bitmaps', [])

                print(f"  Initialized Ticks: {len(ticks)}")
                print(f"  Bitmap Words: {len(bitmaps)}")

                # Show sample of ticks
                if ticks:
                    print("  Sample ticks:")
                    for tick in ticks[:5]:
                        print(f"    Tick {tick['tick']}: initialized={tick['initialized']}")

                # Show sample of bitmaps
                if bitmaps:
                    print("  Sample bitmaps:")
                    for bitmap in bitmaps[:3]:
                        print(f"    Word {bitmap['word_pos']}: bitmap={bitmap['bitmap']}")

            print()

        print("=" * 80)
        print("Collection complete!")
        print("=" * 80)

        # Optionally export to file
        if os.getenv("EXPORT_JSON"):
            with open("pool_data.json", "w") as f:
                json.dump(results, f, indent=2)
            print("\nData exported to pool_data.json")

    except Exception as e:
        import traceback
        print(f"Error: {e}")
        traceback.print_exc()
        return 1

    return 0


if __name__ == "__main__":
    exit(main())
