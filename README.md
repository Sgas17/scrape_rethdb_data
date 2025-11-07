# scrape_rethdb_data

High-performance library for collecting Uniswap pool data (V2, V3, and V4) directly from the Reth database, bypassing RPC overhead.

## Features

- **Direct Database Access**: Read pool state directly from Reth's MDBX database
- **Multi-Protocol Support**: Works with UniswapV2, UniswapV3, and UniswapV4 pools
- **Comprehensive Data Collection**:
  - V2: Reserve data (reserve0, reserve1, blockTimestampLast)
  - V3/V4: Slot0 data, tick data, and tick bitmaps
- **Historical Queries**: Query pool state at any past block number using Reth's changesets
- **Event Scanning**: Efficiently scan for events with bloom filter optimization
  - Single-address scanning
  - Multi-address scanning (optimized ~N times faster for N addresses)
  - Built-in V3 event filters (Swap, Mint, Burn)
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

### Historical Queries

Query pool state at specific block numbers for backtesting and analysis:

```rust
use scrape_rethdb_data::{collect_pool_data_at_block, PoolInput};

let pool = PoolInput::new_v3(pool_address, 60);
let block_number = 18000000; // August 2023

let results = collect_pool_data_at_block(
    db_path,
    &[pool],
    None,
    block_number
)?;

println!("Pool state at block {}: {:?}", block_number, results[0]);
```

### Event Scanning

Efficiently scan for events using bloom filter optimization:

```rust
use scrape_rethdb_data::{get_v3_swap_events, scan_pool_events_multi};

// Scan single pool for swap events
let result = get_v3_swap_events(
    db_path,
    pool_address,
    from_block,
    to_block
)?;

println!("Found {} swap events", result.logs.len());
println!("Skipped {} blocks via bloom filter", result.blocks_skipped_by_bloom);

// Scan multiple pools (OPTIMIZED - scans each block only once!)
let pool_addresses = vec![addr1, addr2, addr3];
let results = scan_pool_events_multi(
    db_path,
    &pool_addresses,
    from_block,
    to_block,
    None  // Optional topic filters
)?;

// Results contain events for each address
for (i, result) in results.iter().enumerate() {
    println!("Pool {}: {} events", i, result.logs.len());
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

#### Historical Queries (Python)

```python
import json
import scrape_rethdb_data

# Query pool state at a specific block
pools = [{"address": "0x...", "protocol": "v3", "tick_spacing": 60}]
block_number = 18000000

result_json = scrape_rethdb_data.collect_pools_at_block(
    "/path/to/reth/db",
    pools,
    block_number,
    None  # v4_pool_ids (optional)
)

results = json.loads(result_json)
print(f"Pool state at block {results[0]['block_number']}")
print(f"Tick: {results[0]['pool_data']['slot0']['tick']}")
```

#### Event Scanning (Python)

```python
import json
import scrape_rethdb_data

# Scan for swap events from a single pool
result_json = scrape_rethdb_data.get_swap_events(
    "/path/to/reth/db",
    "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",  # USDC/WETH
    20000000,  # from_block
    20100000   # to_block
)

result = json.loads(result_json)
print(f"Found {len(result['logs'])} swap events")
print(f"Bloom filter skipped {result['blocks_skipped_by_bloom']} blocks")

# Scan multiple pools (OPTIMIZED)
pool_addresses = [
    "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
    "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8",
]

results_json = scrape_rethdb_data.scan_events_multi(
    "/path/to/reth/db",
    pool_addresses,
    20000000,
    20100000,
    None  # topics (optional)
)

results = json.loads(results_json)
for i, result in enumerate(results):
    print(f"Pool {i}: {len(result['logs'])} events")
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

### Pool State Queries

- **RPC**: ~100-500ms per pool (network + node processing)
- **Direct DB**: ~1-10ms per pool (local disk I/O only)

For batches of 1000+ pools, this translates to minutes vs. hours.

### Historical Queries

- Uses Reth's changesets for efficient historical state reconstruction
- ~5-20ms per pool per block (depends on state history)
- Much faster than archive node RPC calls

### Event Scanning

- **Bloom Filter Optimization**: Skips 80-95% of blocks (depending on pool activity)
- **Multi-Address Scanning**: Scans N addresses in same time as 1 address
  - Single-address scan: M block reads for M blocks
  - Multi-address scan: M block reads for N addresses (N times faster!)
- **Direct DB Access**: No RPC rate limits or network overhead

Example: Scanning 3 pools for 100,000 blocks:
- **RPC**: 300,000 requests × 100ms = 8.3 hours
- **Direct DB (single)**: 300,000 block reads × 1ms = 5 minutes
- **Direct DB (multi)**: 100,000 block reads × 1ms = 1.7 minutes

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

### Run Examples

```bash
# Set database path
export RETH_DB_PATH="/path/to/reth/db"

# Current pool state
cargo run --example collect_pool_data

# Historical pool state
cargo run --example historical_pool_state
cargo run --example historical_v4_pool

# Event scanning
cargo run --example scan_swap_events
cargo run --example scan_multi_pools

# V4 pools (live data)
cargo run --example test_v4_live

# V4 slot calculation verification
cargo run --example test_v4_slot

# Python example (after installing module)
python python_example.py
```

## Testing

```bash
# Run unit tests
cargo test

# Run with specific feature
cargo test --features python
```

### Performance Benchmarks: DB vs RPC

We provide comprehensive integration tests that verify correctness and measure performance:

```bash
# Set required environment variables
export RETH_DB_PATH="/path/to/reth/db"
export RPC_URL="http://localhost:8545"  # Optional, defaults to localhost:8545

# Run all benchmark tests (requires DB and RPC access)
cargo test --test db_vs_rpc_benchmark -- --nocapture --test-threads=1 --ignored

# Run specific benchmark
cargo test --test db_vs_rpc_benchmark test_v3_slot0_db_vs_rpc -- --nocapture --ignored
```

**Available Benchmark Tests:**

1. **`test_v3_slot0_db_vs_rpc`** - Compare V3 slot0 reads
2. **`test_v2_reserves_db_vs_rpc`** - Compare V2 reserve reads
3. **`test_historical_query_db_vs_rpc`** - Compare historical queries (requires archive node)
4. **`test_event_scanning_performance`** - Measure event scanning with bloom filter stats
5. **`test_multi_pool_scanning_performance`** - Benchmark optimized multi-pool scanning
6. **`test_batch_pool_query_performance`** - Compare batch queries vs sequential RPC

Each test:
- Verifies that DB reads match RPC responses exactly
- Measures execution time for both methods
- Reports speedup factor (typically 10-100x for current state, 5-50x for historical)
- Shows bloom filter efficiency for event scans

**Example Output:**
```
=== V3 Slot0 Test ===
Pool: 0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640

DB Read:
  Time: 1.234ms
  sqrtPriceX96: 1234567890...
  tick: 123456

RPC Call:
  Time: 98.765ms
  sqrtPriceX96: 1234567890...
  tick: 123456

✓ All fields match!
⚡ DB is 80.0x faster than RPC
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
- **V4 Slot0**: Same as V3, but tests the PoolManager singleton pattern

For V4 testing, set additional environment variables:
```bash
export V4_POOL_MANAGER="0x..." # V4 PoolManager contract address
export V4_POOL_ID="0x..."      # Pool ID (bytes32)
```

## Module Structure

```
src/
├── lib.rs           # Main API
├── types.rs         # Data structures (BlockNumber, PoolInput, PoolOutput, etc.)
├── storage.rs       # Storage slot calculations (V2/V3/V4)
├── tick_math.rs     # Tick/bitmap math utilities
├── readers.rs       # Protocol-specific current state readers
├── historical.rs    # Historical state queries via changesets
├── events.rs        # Event log scanning with bloom filters
├── decoding.rs      # Storage value decoders
├── contracts.rs     # Solidity type definitions
└── python.rs        # Python bindings (optional)

examples/
├── collect_pool_data.rs      # Basic pool data collection
├── historical_pool_state.rs  # Historical V3 pool queries
├── historical_v4_pool.rs     # Historical V4 pool queries
├── scan_swap_events.rs       # Single pool event scanning
├── scan_multi_pools.rs       # Multi-pool optimized scanning
├── test_v4_live.rs          # V4 live data test
└── test_v4_slot.rs          # V4 slot calculation verification
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

- [x] ~~Add block number parameter for historical queries~~ (✓ Implemented)
- [x] ~~Event log scanning~~ (✓ Implemented with bloom filter optimization)
- [ ] Add caching layer for repeated queries
- [ ] Support for other DEX protocols (SushiSwap, Curve, etc.)
- [ ] Parallel processing for large batch queries
- [ ] Streaming API for real-time updates
