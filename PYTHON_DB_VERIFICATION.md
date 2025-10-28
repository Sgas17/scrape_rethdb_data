# Python DB vs RPC Verification Test

This guide explains how to use the Python version of the DB vs RPC verification test.

## Overview

`verify_db_vs_rpc.py` is a Python script that verifies data collected from the Reth database matches data retrieved via RPC calls. This is the Python equivalent of the Rust `examples/verify_db_vs_rpc.rs` test.

## Prerequisites

### 1. Install uv (if not already installed)

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

### 2. Install Dependencies

Using `uv`:

```bash
# Install Python dependencies
uv pip install web3 python-dotenv

# Or install from requirements if you have one
# uv pip install -r requirements.txt
```

### 3. Build Python Bindings

Build the Rust library with Python bindings using `maturin` in the uv environment:

```bash
# Install maturin if not already installed
uv pip install maturin

# Build and install the Python bindings
uv run maturin develop --features python

# Or if maturin is already installed globally:
maturin develop --features python
```

### 4. Environment Variables

Create a `.env` file:

```env
RETH_DB_PATH=/path/to/reth/db
RPC_URL=http://localhost:8545
```

### 5. Requirements

- Access to a Reth database (local file system)
- Access to an Ethereum RPC endpoint (same chain as the DB)
- Both must be at the same block height for accurate comparison

## What It Tests

The script performs comprehensive verification for both V3 and V4 pools:

### For Each Pool:

1. **Slot0 Verification**
   - Retrieves `slot0` from both DB and RPC
   - Compares `sqrtPriceX96` and `tick` values
   - Reports any mismatches

2. **Tick Data Verification**
   - Finds the nearest initializable tick based on tick spacing
   - Generates 11 test ticks around the current tick
   - Retrieves each tick from both DB and RPC
   - Compares `liquidityGross` and `liquidityNet`
   - Handles uninitialized ticks (not in DB but zero on RPC)

3. **Bitmap Verification**
   - Calculates bitmap word positions for the test ticks
   - Retrieves bitmaps from both DB and RPC
   - Compares bitmap values
   - Handles zero bitmaps (not stored in DB)

## Usage

### Basic Usage

Run the verification test using `uv`:

```bash
# Run directly with uv
uv run python verify_db_vs_rpc.py

# Or activate the uv environment and run
uv run verify_db_vs_rpc.py
```

### Alternative: Traditional Python

If you prefer traditional Python execution:

```bash
python verify_db_vs_rpc.py
```

### Example Output

```
================================================================================
RETH DATABASE vs RPC VERIFICATION TEST (Python)
================================================================================
RPC URL: http://localhost:8545
DB Path: /path/to/reth/db
Connected to chain ID: 1

================================================================================
V3 DATABASE vs RPC VERIFICATION
================================================================================
Pool: 0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640
DB Path: /path/to/reth/db

--- Step 1: Get Current Tick from RPC ---
Current tick: -193150
Tick spacing: 10

--- Step 2: Find Nearest Initializable Tick ---
Nearest initializable tick: -193150

--- Step 3: Collect Data from Reth DB ---
Collected 156 ticks from DB
Collected 12 bitmaps from DB

--- Step 4: Compare Slot0 ---
sqrtPriceX96 match: True
  DB:  5068644170580286966069059
  RPC: 5068644170580286966069059
tick match: True
  DB:  -193150
  RPC: -193150

--- Step 5: Compare Ticks ---
Tick comparisons: 11 matches, 0 mismatches

--- Step 6: Compare Bitmaps ---
Bitmap comparisons: 3 matches, 0 mismatches

================================================================================
✓ VERIFICATION PASSED - All DB data matches RPC!
================================================================================
```

## Test Pools

### Default V3 Pool
- **Address**: `0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640`
- **Pair**: USDC/WETH
- **Fee**: 0.05%
- **Tick Spacing**: 10

### Default V4 Pool
- **Pool Manager**: `0x000000000004444c5dc75cB358380D2e3dE08A90`
- **Pool ID**: `0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d`
- **Tick Spacing**: 60

## Customization

To test different pools, edit the `main()` function in the script:

```python
# Test a different V3 pool
v3_pool = "0x<your-pool-address>"
result = verify_v3_db_vs_rpc(w3, db_path, v3_pool)

# Test a different V4 pool
v4_pool_manager = "0x000000000004444c5dc75cB358380D2e3dE08A90"
v4_pool_id = "0x<your-pool-id>"
v4_tick_spacing = 60  # Adjust based on your pool

result = verify_v4_db_vs_rpc(
    w3,
    db_path,
    v4_pool_manager,
    v4_pool_id,
    v4_tick_spacing
)
```

## How It Works

### 1. DB Data Collection

Uses the Python bindings to call the Rust library:

```python
pools_input = [{
    "address": pool_address,
    "protocol": "v3",  # or "v4"
    "tick_spacing": tick_spacing,
    "slot0_only": False,
}]

result_json = scrape_rethdb_data.collect_pools(db_path, pools_input, pool_ids)
db_data = json.loads(result_json)[0]
```

### 2. RPC Data Collection

Uses Web3.py to call contract functions:

**For V3 Pools** (direct pool contract):
```python
pool = w3.eth.contract(address=pool_address, abi=V3_POOL_ABI)
slot0 = pool.functions.slot0().call()
tick_data = pool.functions.ticks(tick).call()
bitmap = pool.functions.tickBitmap(word_pos).call()
```

**For V4 Pools** (via StateView contract):
```python
stateview = w3.eth.contract(address=STATEVIEW_ADDRESS, abi=V4_STATEVIEW_ABI)
slot0 = stateview.functions.getSlot0(pool_id).call()
tick_data = stateview.functions.getTickLiquidity(pool_id, tick).call()
bitmap = stateview.functions.getTickBitmap(pool_id, word_pos).call()
```

### 3. Data Comparison

- Creates lookup dictionaries from DB data for efficient comparison
- Iterates through test ticks/bitmaps and compares values
- Handles cases where data is missing from DB (should be zero on RPC)
- Reports any mismatches with detailed information

## Understanding Results

### Success Indicators

- `✓ VERIFICATION PASSED` - All data matches perfectly
- High match counts for ticks and bitmaps
- `slot0_match: True`

### Failure Indicators

- `✗ VERIFICATION FAILED` - Found mismatches
- Mismatch counts > 0
- Detailed mismatch reports showing which values differ

### Common Issues

1. **Block Height Mismatch**
   - DB and RPC are at different blocks
   - Solution: Ensure both sources are synchronized

2. **Missing Data**
   - Tick shows liquidity on RPC but not in DB
   - May indicate the tick was initialized very recently

3. **Network Issues**
   - RPC connection failures
   - Solution: Check `RPC_URL` is correct and accessible

## Exit Codes

- `0` - All tests passed
- `1` - Some tests failed

## Comparison with Rust Version

| Feature | Python Version | Rust Version |
|---------|---------------|--------------|
| Performance | Slower (Python overhead) | Faster (native) |
| Dependencies | Web3.py, dotenv | Alloy, eyre |
| Package Manager | uv | cargo |
| Ease of Use | More familiar to Python devs | More familiar to Rust devs |
| RPC Calls | Web3.py | Alloy providers |
| DB Access | Python bindings → Rust | Direct Rust |

Both versions perform identical verification logic and should produce the same results.

## Building with uv - Complete Workflow

Here's the complete workflow for setting up and running with `uv`:

```bash
# 1. Install dependencies
uv pip install web3 python-dotenv maturin

# 2. Build the Python bindings
uv run maturin develop --features python

# 3. Run the verification test
uv run python verify_db_vs_rpc.py
```

## Troubleshooting

### Import Error: `scrape_rethdb_data`

Rebuild the Python bindings:
```bash
uv run maturin develop --features python
```

### Connection Error

Check your RPC URL:
```bash
curl -X POST $RPC_URL -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'
```

### DB Path Error

Verify the path exists:
```bash
ls -la $RETH_DB_PATH
```

### uv Command Not Found

Install uv:
```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Then restart your shell or run:
```bash
source $HOME/.cargo/env
```

## Next Steps

After successful verification:

1. **Integration Testing**: Use this in CI/CD pipelines with `uv`
2. **Monitoring**: Run periodically to ensure DB consistency
3. **Custom Pools**: Test with your own pool addresses
4. **Performance**: Compare Python vs Rust execution times

## Related Documentation

- [DB_VS_RPC_TEST.md](DB_VS_RPC_TEST.md) - Rust version documentation
- [SLOT_VERIFICATION_GUIDE.md](SLOT_VERIFICATION_GUIDE.md) - Slot calculation verification
- [V4_SLOT_CALCULATION_FIX.md](V4_SLOT_CALCULATION_FIX.md) - V4 slot calculation details
