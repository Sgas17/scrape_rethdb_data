# Reth Database Scraper - Python Usage Guide

This library allows you to read Uniswap V2, V3, and V4 pool data directly from a Reth database using Python.

## Prerequisites

1. **Reth Node**: You need a synced Reth node with access to its database
2. **Python 3.11+**: The library is built for Python 3.11
3. **uv**: Python package manager (recommended)
4. **Database Permissions**: Read access to the Reth database directory

## Installation

### 1. Build the Rust Library

```bash
cargo build --release --features python
```

This creates `target/release/libscrape_rethdb_data.so`

### 2. Install Python Dependencies

```bash
# Using uv (recommended)
uv venv
source .venv/bin/activate
uv pip install maturin

# Or using pip
pip install maturin
```

### 3. Install the Python Module

```bash
# Using maturin (installs in development mode)
maturin develop --release --features python

# Or manually copy the .so file to your Python site-packages
cp target/release/libscrape_rethdb_data.so \
   ~/.local/lib/python3.11/site-packages/scrape_rethdb_data.cpython-311-x86_64-linux-gnu.so
```

### 4. Set Environment Variables

Create a `.env` file:

```bash
# Path to Reth database
RETH_DB_PATH=/var/lib/docker/volumes/eth-docker_reth-el-data/_data/db

# RPC URL (for validation/testing)
RPC_URL=http://localhost:8545

# V4 StateView contract (optional, for V4 validation)
V4_STATEVIEW_ADDRESS=0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227
```

### 5. Fix Database Permissions (if needed)

If you get permission errors, run:

```bash
sudo ./fix_docker_volumes_access.sh
sudo ./fix_mdbx_final.sh
```

These scripts use ACLs to grant read access without changing file ownership.

## Python API

### Basic Usage

```python
import scrape_rethdb_data
import json
import os

# Set database path
db_path = os.getenv("RETH_DB_PATH")

# Define pools to query
pools = [
    {
        "address": "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
        "protocol": "v2",
        "tick_spacing": None,  # Not needed for V2
    },
    {
        "address": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
        "protocol": "v3",
        "tick_spacing": 10,  # Required for V3
    },
]

# For V4, also provide pool IDs
v4_pool_ids = [
    "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d",
]

# Add V4 pool to the list
pools.append({
    "address": "0x000000000004444c5dc75cB358380D2e3dE08A90",  # PoolManager
    "protocol": "v4",
    "tick_spacing": 60,  # Required for V4
})

# Collect data
result_json = scrape_rethdb_data.collect_pools(db_path, pools, v4_pool_ids)
results = json.loads(result_json)

# Process results
for pool_data in results:
    print(f"Pool: {pool_data['address']}")
    print(f"Protocol: {pool_data['protocol']}")
    # ... access pool data
```

## Data Structures

### Uniswap V2 Pool Output

```python
{
    "address": "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
    "protocol": "uniswapv2",
    "reserves": {
        "reserve0": 24952205359131,          # uint112
        "reserve1": 6341293753918349587147,  # uint112
        "block_timestamp_last": 1761314183,  # uint32
        "raw_data": "0x68fb8587000000000157c3210f93128f32cb000000000000000016b1a356381b"
    }
}
```

### Uniswap V3 Pool Output

```python
{
    "address": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
    "protocol": "uniswapv3",
    "slot0": {
        "sqrt_price_x96": "0x3e3535586cb8adc2ccee2eb0dc9e",  # hex string
        "tick": 193522,                                          # int24
        "unlocked": True,                                        # bool
        "raw_data": "0x00010002d302d3022a02f3f20000000000003e3535586cb8adc2ccee2eb0dc9e"
    },
    "bitmaps": [
        {
            "word_pos": -347,                    # int16
            "bitmap": "0x200000000000000000000000000"  # uint256 as hex
        },
        // ... more bitmap words
    ],
    "ticks": [
        {
            "tick": -887270,                     # int24
            "liquidity_gross": 123456,           # uint128
            "liquidity_net": -123456,            # int128
            "initialized": True,                 # bool
            "raw_data": "0x..."
        },
        // ... more ticks
    ]
}
```

### Uniswap V4 Pool Output

```python
{
    "address": "0x000000000004444c5dc75cB358380D2e3dE08A90",
    "protocol": "uniswapv4",
    "pool_id": "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d",
    "slot0": {
        "sqrt_price_x96": "0x418e3e60570cee5f14dcf",
        "tick": -193611,
        "unlocked": False,
        "raw_data": "0x..."
    },
    "bitmaps": [
        // Same structure as V3
    ],
    "ticks": [
        // Same structure as V3
    ]
}
```

## Complete Examples

### Example 1: Get V2 Reserves for Multiple Pools

```python
import scrape_rethdb_data
import json
import os

db_path = os.getenv("RETH_DB_PATH")

# List of V2 pools
v2_pools = [
    {
        "address": "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",  # USDC/WETH
        "protocol": "v2",
        "tick_spacing": None,
    },
    {
        "address": "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852",  # WETH/USDT
        "protocol": "v2",
        "tick_spacing": None,
    },
    {
        "address": "0xd3d2E2692501A5c9Ca623199D38826e513033a17",  # UNI/WETH
        "protocol": "v2",
        "tick_spacing": None,
    },
]

# Collect reserves
result_json = scrape_rethdb_data.collect_pools(db_path, v2_pools, [])
results = json.loads(result_json)

# Print reserves for each pool
for pool in results:
    reserves = pool['reserves']
    print(f"\nPool: {pool['address']}")
    print(f"  Reserve0: {reserves['reserve0']}")
    print(f"  Reserve1: {reserves['reserve1']}")
    print(f"  Timestamp: {reserves['block_timestamp_last']}")
```

### Example 2: Get V3 Tick Data for Multiple Pools

```python
import scrape_rethdb_data
import json
import os

db_path = os.getenv("RETH_DB_PATH")

# List of V3 pools with their tick spacings
v3_pools = [
    {
        "address": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",  # USDC/WETH 0.05%
        "protocol": "v3",
        "tick_spacing": 10,
    },
    {
        "address": "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8",  # USDC/WETH 0.3%
        "protocol": "v3",
        "tick_spacing": 60,
    },
    {
        "address": "0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36",  # WETH/USDT 0.3%
        "protocol": "v3",
        "tick_spacing": 60,
    },
]

# Collect data
result_json = scrape_rethdb_data.collect_pools(db_path, v3_pools, [])
results = json.loads(result_json)

# Process tick data
for pool in results:
    print(f"\nPool: {pool['address']}")
    print(f"  Current tick: {pool['slot0']['tick']}")
    print(f"  Price: {pool['slot0']['sqrt_price_x96']}")
    print(f"  Total ticks: {len(pool['ticks'])}")
    print(f"  Bitmap words: {len(pool['bitmaps'])}")

    # Print first few initialized ticks
    print(f"  First 5 ticks:")
    for tick in pool['ticks'][:5]:
        print(f"    Tick {tick['tick']}: liquidity_gross={tick['liquidity_gross']}")
```

### Example 3: Get V4 Pool Data

```python
import scrape_rethdb_data
import json
import os

db_path = os.getenv("RETH_DB_PATH")

# V4 PoolManager address (singleton)
pool_manager = "0x000000000004444c5dc75cB358380D2e3dE08A90"

# List of V4 pool IDs you want to query
v4_pool_ids = [
    "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d",
    # Add more pool IDs here...
]

# Create pool entries (one for each pool ID)
v4_pools = []
for pool_id in v4_pool_ids:
    v4_pools.append({
        "address": pool_manager,
        "protocol": "v4",
        "tick_spacing": 60,  # This should match the pool's actual tick spacing
    })

# Collect data
result_json = scrape_rethdb_data.collect_pools(db_path, v4_pools, v4_pool_ids)
results = json.loads(result_json)

# Process results
for pool in results:
    print(f"\nV4 Pool ID: {pool['pool_id']}")
    print(f"  Current tick: {pool['slot0']['tick']}")
    print(f"  Price: {pool['slot0']['sqrt_price_x96']}")
    print(f"  Total ticks: {len(pool['ticks'])}")
    print(f"  Bitmap words: {len(pool['bitmaps'])}")
```

### Example 4: Mixed Protocol Query

```python
import scrape_rethdb_data
import json
import os

db_path = os.getenv("RETH_DB_PATH")

# Query V2, V3, and V4 pools in a single call
pools = [
    # V2 pools
    {
        "address": "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
        "protocol": "v2",
        "tick_spacing": None,
    },
    # V3 pools
    {
        "address": "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
        "protocol": "v3",
        "tick_spacing": 10,
    },
    # V4 pools
    {
        "address": "0x000000000004444c5dc75cB358380D2e3dE08A90",
        "protocol": "v4",
        "tick_spacing": 60,
    },
]

# V4 pool IDs (must match the order of V4 pools in the list above)
v4_pool_ids = [
    "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d",
]

# Collect all data in one call
result_json = scrape_rethdb_data.collect_pools(db_path, pools, v4_pool_ids)
results = json.loads(result_json)

# Process by protocol
for pool in results:
    if pool['protocol'] == 'uniswapv2':
        print(f"V2 Pool {pool['address']}: reserves={pool['reserves']['reserve0']}, {pool['reserves']['reserve1']}")
    elif pool['protocol'] == 'uniswapv3':
        print(f"V3 Pool {pool['address']}: tick={pool['slot0']['tick']}, ticks={len(pool['ticks'])}")
    elif pool['protocol'] == 'uniswapv4':
        print(f"V4 Pool {pool['pool_id']}: tick={pool['slot0']['tick']}, ticks={len(pool['ticks'])}")
```

## Understanding Tick Bitmaps

Tick bitmaps are used in V3 and V4 to efficiently track which ticks are initialized (have liquidity).

### Bitmap Structure

- Each bitmap word is a `uint256` (256 bits)
- Each bit represents whether a tick at a specific offset is initialized
- Word position (`word_pos`) determines which range of ticks the bitmap covers

### Converting Bitmap to Tick Numbers

```python
def extract_ticks_from_bitmap(word_pos, bitmap_value, tick_spacing):
    """Extract initialized tick numbers from a bitmap word"""
    ticks = []

    # Each word covers 256 ticks
    base_tick = word_pos * 256 * tick_spacing

    # Check each bit
    for i in range(256):
        if bitmap_value & (1 << i):
            tick = base_tick + (i * tick_spacing)
            ticks.append(tick)

    return ticks

# Example usage
bitmap = pool['bitmaps'][0]
word_pos = bitmap['word_pos']
bitmap_value = int(bitmap['bitmap'], 16)
tick_spacing = 10

ticks = extract_ticks_from_bitmap(word_pos, bitmap_value, tick_spacing)
print(f"Initialized ticks in word {word_pos}: {ticks}")
```

Note: The library already extracts all initialized ticks for you in the `ticks` array, so you typically don't need to process bitmaps manually.

## Important Notes

### Storage Slot Details

The library correctly handles storage slot calculations for all protocols:

- **V2**: Reserves at slot 8 (packed: timestamp | reserve1 | reserve0)
- **V3**:
  - Slot0 at slot 0
  - Ticks mapping at slot 5
  - TickBitmap mapping at slot 6
- **V4**:
  - `_pools` mapping at slot 6 (accounts for inherited storage)
  - Pool state struct offsets from base slot

### Tick Spacing

Tick spacing varies by pool fee tier:
- **V3**:
  - 0.01% fee: tick_spacing = 1
  - 0.05% fee: tick_spacing = 10
  - 0.3% fee: tick_spacing = 60
  - 1% fee: tick_spacing = 200
- **V4**: Configurable per pool

### Performance Considerations

- Reading from the database is very fast (microseconds per query)
- The library uses exact slot matching to ensure correctness
- Bitmap generation scans a range based on tick spacing

### Database Access

- The database must be opened in read-only mode
- Multiple processes can read simultaneously
- Write access is not needed (read-only operations)

## Troubleshooting

### Permission Denied Errors

If you get `Permission denied` errors:

1. Check that the database path is correct
2. Run the permission fix scripts:
   ```bash
   sudo ./fix_docker_volumes_access.sh
   sudo ./fix_mdbx_final.sh
   ```
3. Verify you can read the files:
   ```bash
   ls -la /var/lib/docker/volumes/eth-docker_reth-el-data/_data/db
   ```

### Module Not Found

If Python can't find the module:

1. Check the .so file is in the right location
2. Make sure the Python version matches (3.11)
3. Try reinstalling with maturin:
   ```bash
   maturin develop --release --features python
   ```

### Empty Results

If you get empty results:

1. Verify the pool addresses are correct and checksum-formatted
2. Check that the pools exist on mainnet (not testnet)
3. Ensure the Reth database is synced to a recent block
4. For V4, make sure the pool ID is correct

### Data Mismatch with RPC

Small differences in tick/price between DB and RPC are expected because:
- Pool state changes with every swap
- DB and RPC may be at slightly different blocks
- Solution: Query both at the same time for comparison

## Testing and Validation

Run the validation scripts to verify correctness:

```bash
# Validate V2 reserves
python validate_simple.py

# Validate V3 bitmaps and ticks
python debug_bitmap.py

# Validate V4 data
python validate_v4_bitmaps.py
```

All validation scripts compare database values against RPC calls to ensure accuracy.
