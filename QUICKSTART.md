# Quick Start Guide

## Using from Rust

### 1. Add to your project

```toml
[dependencies]
scrape_rethdb_data = { path = "../scrape_rethdb_data" }
```

### 2. Basic usage

```rust
use scrape_rethdb_data::{collect_pool_data, PoolInput};

fn main() -> eyre::Result<()> {
    // Define your pools
    let pools = vec![
        PoolInput::new_v3(
            "0xa83326d20b7003bcecf1f4684a2fbb56161e2a8e".parse()?,
            60  // tick spacing
        ),
        PoolInput::new_v2(
            "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".parse()?
        ),
    ];

    // Collect data
    let db_path = "/path/to/reth/db";
    let results = collect_pool_data(db_path, &pools, None)?;

    // Use the data
    for pool in results {
        println!("{:?}", pool);
    }

    Ok(())
}
```

### 3. Run the example

```bash
export RETH_DB_PATH="/path/to/reth/db"
cargo run --example collect_pool_data
```

### 4. Validate decoding correctness

Compare DB reads vs RPC to ensure data accuracy:

```bash
export RETH_DB_PATH="/path/to/reth/db"
export RPC_URL="http://localhost:8545"  # Optional
cargo run --example validate_db_vs_rpc
```

## Using from Python

### 1. Install dependencies

```bash
pip install maturin
```

### 2. Build the Python module

```bash
cd /path/to/scrape_rethdb_data
maturin develop --features python
```

### 3. Use in Python

```python
import json
import scrape_rethdb_data

# Define pools
pools = [
    {
        "address": "0xa83326d20b7003bcecf1f4684a2fbb56161e2a8e",
        "protocol": "v3",
        "tick_spacing": 60,
    },
    {
        "address": "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
        "protocol": "v2",
        "tick_spacing": None,
    },
]

# Collect data
result_json = scrape_rethdb_data.collect_pools("/path/to/reth/db", pools)
results = json.loads(result_json)

# Use the data
for pool in results:
    print(pool)
```

### 4. Run the example

```bash
export RETH_DB_PATH="/path/to/reth/db"
python python_example.py
```

## Integrating with PostgreSQL

### Example: Query pools from database, collect from Reth

```python
import psycopg2
import json
import scrape_rethdb_data

# 1. Connect to your database
conn = psycopg2.connect("postgresql://user:pass@localhost/db")
cursor = conn.cursor()

# 2. Query pools with their metadata
cursor.execute("""
    SELECT address, factory, tick_spacing
    FROM pools
    WHERE active = true
    LIMIT 100
""")

# 3. Build pool list for Rust
pools = []
for row in cursor.fetchall():
    address, factory, tick_spacing = row

    # Infer protocol from factory
    if factory == "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".lower():
        protocol = "v2"
    elif factory == "0x1F98431c8aD98523631AE4a59f267346ea31F984".lower():
        protocol = "v3"
    else:
        continue  # Unknown factory

    pools.append({
        "address": address,
        "protocol": protocol,
        "tick_spacing": tick_spacing,
    })

# 4. Collect data from Reth (FAST!)
result_json = scrape_rethdb_data.collect_pools("/path/to/reth/db", pools)
results = json.loads(result_json)

# 5. Process results
for pool_data in results:
    # Store in database, run analytics, etc.
    print(f"Pool {pool_data['address']}: {len(pool_data.get('ticks', []))} ticks")
```

## Common Patterns

### Parallel batches

```python
import concurrent.futures
import scrape_rethdb_data

def collect_batch(pools_batch):
    return scrape_rethdb_data.collect_pools("/path/to/reth/db", pools_batch)

# Split pools into batches of 100
batches = [all_pools[i:i+100] for i in range(0, len(all_pools), 100)]

# Collect in parallel (opens DB read-only, safe for concurrent access)
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
    results = list(executor.map(collect_batch, batches))
```

### Export to JSON file

```bash
export EXPORT_JSON=1
python python_example.py
# Creates pool_data.json
```

## Troubleshooting

### "Database not found"

Make sure `RETH_DB_PATH` points to the directory containing the MDBX database files (should contain `db.mdbx` and related files).

### "Cannot infer protocol"

For Python, make sure the `protocol` field is one of: `"v2"`, `"v3"`, `"v4"`, `"uniswapv2"`, `"uniswapv3"`, `"uniswapv4"`.

### Missing tick data

The current implementation reads initialized ticks from bitmaps. If a tick exists but its bit isn't set in the bitmap, it won't be returned. This is expected behavior for the efficient storage pattern used by Uniswap.

### V4 pools

For V4 pools, you must provide the `pool_ids` parameter (list of PoolId hashes). The pool address should be the PoolManager singleton address.
