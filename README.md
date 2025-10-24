# scrape_rethdb_data

High-performance library for collecting Uniswap pool data (V2, V3, and V4) directly from the Reth database, bypassing RPC overhead.

## Features

- **Direct Database Access**: Read pool state directly from Reth's MDBX database
- **Multi-Protocol Support**: Works with UniswapV2, UniswapV3, and UniswapV4 pools
- **Comprehensive Data Collection**:
  - V2: Reserve data (reserve0, reserve1, blockTimestampLast)
  - V3/V4: Slot0 data, tick data, and tick bitmaps
- **Dynamic Pool Lists**: Accept pools as input parameters (no hardcoding needed)
- **Tick Math**: Automatic calculation of word positions based on tickSpacing
- **Dual Interface**: Use from Rust or Python

## Architecture

The library is built on the `reth_bitmap_benchmark` pattern and extends it to:

1. **Accept dynamic pool configurations** via function parameters
2. **Support multiple protocols** (V2/V3/V4) with protocol-specific readers
3. **Calculate tick ranges** based on each pool's tickSpacing
4. **Efficiently query** all relevant storage slots for each pool type

## Usage

### From Rust

```rust
use scrape_rethdb_data::{collect_pool_data, PoolInput};

// Define pools
let pools = vec![
    PoolInput::new_v3(
        "0xa83326d20b7003bcecf1f4684a2fbb56161e2a8e".parse()?,
        60,  // tick spacing
    ),
    PoolInput::new_v2(
        "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".parse()?,
    ),
];

// Collect data
let db_path = "/path/to/reth/db";
let results = collect_pool_data(db_path, &pools, None)?;

// Process results
for pool_data in results {
    match pool_data.protocol {
        Protocol::UniswapV2 => {
            println!("Reserves: {:?}", pool_data.reserves);
        }
        Protocol::UniswapV3 | Protocol::UniswapV4 => {
            println!("Current tick: {}", pool_data.slot0.unwrap().tick);
            println!("Initialized ticks: {}", pool_data.ticks.len());
            println!("Bitmap words: {}", pool_data.bitmaps.len());
        }
    }
}
```

### From Python

First, build and install the Python module:

```bash
# Install maturin
pip install maturin

# Build and install in development mode
maturin develop --features python

# Or build a wheel for distribution
maturin build --features python --release
pip install target/wheels/scrape_rethdb_data-*.whl
```

Then use it in Python:

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

# Process results
for pool in results:
    print(f"Pool: {pool['address']}")
    if pool['protocol'] == 'uniswapv2':
        print(f"  Reserves: {pool['reserves']}")
    else:
        print(f"  Tick: {pool['slot0']['tick']}")
        print(f"  Ticks: {len(pool['ticks'])}")
```

## Storage Layout Reference

### UniswapV3 Pools

```solidity
// Slot 0: Slot0 struct (sqrtPriceX96, tick, etc.)
// Slot 4: mapping(int24 => Tick) ticks
// Slot 5: mapping(int16 => uint256) tickBitmap
```

### UniswapV4 Pools

V4 uses a singleton PoolManager pattern with nested mappings:

```solidity
// Slot 0: mapping(PoolId => Pool.State) pools
// Within each pool:
//   Offset 0: Slot0
//   Offset 4: mapping(int24 => Tick) ticks
//   Offset 5: mapping(int16 => uint256) tickBitmap
```

### UniswapV2 Pools

```solidity
// Slot 8: Packed reserves (reserve0 | reserve1 | blockTimestampLast)
```

## Tick Math

The library automatically calculates which bitmap words to query based on tickSpacing:

- **Word Position**: `(tick / tickSpacing) >> 8`
- **Bit Position**: `(tick / tickSpacing) % 256`
- **Full Range**: MIN_TICK (-887272) to MAX_TICK (887272)

For a pool with tickSpacing=60, this generates word positions from approximately -58 to +57.

## Performance

Direct database access is **orders of magnitude faster** than RPC:

- **RPC**: ~100-500ms per pool (network + node processing)
- **Direct DB**: ~1-10ms per pool (local disk I/O only)

For batches of 1000+ pools, this translates to minutes vs. hours.

## Building

### Rust Library

```bash
cargo build --release
```

### Python Module

```bash
# Development build
maturin develop --features python

# Release build
maturin build --features python --release
```

### Run Example

```bash
# Set database path
export RETH_DB_PATH="/path/to/reth/db"

# Rust example
cargo run --example collect_pool_data

# Python example (after installing module)
python python_example.py
```

## Testing

```bash
# Run tests
cargo test

# Run with specific feature
cargo test --features python
```

### Validation: DB vs RPC

To verify the correctness of our decoding logic, run the validation example that compares DB reads against RPC calls:

```bash
# Set database and RPC endpoint
export RETH_DB_PATH="/path/to/reth/db"
export RPC_URL="http://localhost:8545"  # Optional, defaults to localhost:8545

# Run validation
cargo run --example validate_db_vs_rpc
```

This will:
- Read pool data directly from the database using our decoders
- Fetch the same data via RPC using Alloy's type-safe contract calls
- Compare all fields and assert they match
- Report any discrepancies

The validation tests:
- **V2 Reserves**: reserve0, reserve1, blockTimestampLast
- **V3 Slot0**: sqrtPriceX96, tick, observationIndex, observationCardinality, observationCardinalityNext, feeProtocol, unlocked

## Module Structure

```
src/
├── lib.rs           # Main API
├── types.rs         # Data structures
├── storage.rs       # Storage slot calculations
├── tick_math.rs     # Tick/bitmap math utilities
├── readers.rs       # Protocol-specific readers
└── python.rs        # Python bindings (optional)
```

## Data Output

Each `PoolOutput` contains:

```rust
pub struct PoolOutput {
    pub address: Address,
    pub protocol: Protocol,
    pub reserves: Option<Reserves>,        // V2 only
    pub slot0: Option<Slot0>,              // V3/V4 only
    pub ticks: Vec<Tick>,                  // V3/V4 only
    pub bitmaps: Vec<Bitmap>,              // V3/V4 only
}
```

All data is serializable to JSON for easy integration with other tools.

## Integration with Python Analytics

The Python interface is designed to integrate seamlessly with data pipelines:

1. **Query pools from PostgreSQL** to get addresses, protocols, and tick spacings
2. **Pass to Rust library** for high-performance data collection
3. **Receive JSON output** for analysis or storage
4. **Insert into database** or pass to analytics tools

Example workflow:

```python
import psycopg2
import json
import scrape_rethdb_data

# 1. Query pools from database
conn = psycopg2.connect("postgresql://...")
cursor = conn.execute("""
    SELECT address, factory, tick_spacing
    FROM pools
    WHERE active = true
""")

# 2. Build pool list
pools = []
for row in cursor:
    protocol = infer_protocol(row['factory'])
    pools.append({
        "address": row['address'],
        "protocol": protocol,
        "tick_spacing": row['tick_spacing'],
    })

# 3. Collect data from Reth
results_json = scrape_rethdb_data.collect_pools(db_path, pools)
results = json.loads(results_json)

# 4. Process and store results
# ... your analytics code ...
```

## License

MIT OR Apache-2.0

## Contributing

Contributions welcome! Areas for improvement:

- [ ] Complete tick data parsing (currently simplified)
- [ ] Add block number parameter for historical queries
- [ ] Optimize bitmap scanning for sparse pools
- [ ] Add caching layer for repeated queries
- [ ] Support for other DEX protocols (SushiSwap, Curve, etc.)
