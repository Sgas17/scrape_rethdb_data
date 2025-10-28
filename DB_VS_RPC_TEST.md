# Database vs RPC Verification Test

## Purpose

This test verifies that data collected directly from the Reth database matches data from RPC contract calls, proving that:
1. Our slot calculations are correct
2. Our data decoding is correct
3. The database reading logic works properly

## Test: examples/verify_db_vs_rpc.rs

### What It Does

1. **Gets current tick from RPC** - Calls slot0 on the pool contract
2. **Finds nearest initializable tick** - Rounds to valid tick spacing boundary
3. **Generates test ticks** - 11 ticks around current price: `nearestTick +- tickSpacing * n`
4. **Collects from DB** - Uses `scrape_rethdb_data::collect_pool_data()`
5. **Collects from RPC** - Calls pool contract methods
6. **Compares results** - Checks if DB and RPC data match exactly

### Tests

- ✓ V3 pools (direct contract calls)
- ✓ V4 pools (via StateView contract)
- ✓ Slot0 (sqrtPriceX96, tick)
- ✓ Ticks (liquidityGross, liquidityNet)
- ✓ TickBitmaps (word positions and values)

## Requirements

**Must run on a machine with BOTH**:
- Reth database access
- RPC access (can be same node or different)

## Usage

```bash
# Set environment variables
export RETH_DB_PATH=/path/to/reth/mainnet/db
export RPC_URL=http://localhost:8545

# Run the test
cargo run --release --example verify_db_vs_rpc
```

## Example Output

```
================================================================================
V3 DATABASE vs RPC VERIFICATION
================================================================================
Pool: 0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640
DB Path: /mnt/data/reth/mainnet/db

--- Step 1: Get Current Tick from RPC ---
Current tick: 193383
Tick spacing: 10

--- Step 2: Find Nearest Initializable Tick ---
Nearest initializable tick: 193380

--- Step 3: Collect Data from Database ---
✓ DB Slot0:
  sqrtPriceX96: 1252685640355712706855697920
  tick: 193380
  unlocked: true
✓ DB found 5 ticks
✓ DB found 2 bitmap words

--- Step 4: Compare Slot0 ---
sqrtPriceX96: ✓ MATCH
  DB:  1252685640355712706855697920
  RPC: 1252685640355712706855697920
tick: ✓ MATCH
  DB:  193380
  RPC: 193380

--- Step 5: Compare Test Ticks (DB vs RPC) ---
✓ Tick 193360: MATCH
    liquidityGross: 18523456789
    liquidityNet: -5234567
✓ Tick 193370: MATCH
    liquidityGross: 92341234567
    liquidityNet: 12345678
  Tick 193340: Not initialized (both agree)
  Tick 193350: Not initialized (both agree)
...

--- Step 6: Compare Bitmaps (DB vs RPC) ---
✓ Word 754: MATCH (3 bits set)
✓ Word 755: MATCH (2 bits set)

================================================================================
V3 SUMMARY
================================================================================
Ticks:   5 matches, 0 mismatches
Bitmaps: 2 matches, 0 mismatches

✓✓✓ ALL CHECKS PASSED! DB data matches RPC perfectly!
```

## What Gets Verified

### For Each Pool:

**Slot0**:
- sqrtPriceX96: 160-bit value
- tick: Current tick (int24)

**Ticks** (for each initialized tick near current price):
- liquidityGross: u128
- liquidityNet: i128 (signed!)

**Bitmaps** (for word positions containing test ticks):
- Full 256-bit bitmap value
- Number of bits set

## Success Criteria

✓ **ALL** slot0 fields match
✓ **ALL** initialized tick values match
✓ **ALL** bitmap values match
✓ DB and RPC agree on which ticks are initialized

If all these pass → **Database reading is provably correct!**

## Test Pools

### V3 Pool
- **Address**: `0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640`
- **Pair**: USDC/WETH 0.05%
- **Tick Spacing**: 10

### V4 Pool
- **PoolManager**: `0x000000000004444c5dc75cB358380D2e3dE08A90`
- **PoolId**: `0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d`
- **StateView**: `0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227`
- **Tick Spacing**: 60

## Troubleshooting

### "RETH_DB_PATH must be set"
```bash
export RETH_DB_PATH=/mnt/data/reth/mainnet/db
# Or wherever your Reth database is located
```

### "Connection refused"
```bash
export RPC_URL=http://YOUR_NODE_IP:8545
# Make sure RPC is accessible
```

### "No data returned from DB"
- Check that RETH_DB_PATH points to correct location
- Verify database is not corrupted: `ls -lh $RETH_DB_PATH/data.mdb`
- Make sure you have read permissions

### "Mismatches detected"
This would indicate a bug! Investigate:
1. Check which specific values don't match
2. Verify with `cast storage` manually
3. Check if database is fully synced

## Performance Notes

This test is **not** optimized for speed - it's for correctness verification.

For performance testing (speed comparison), use:
- `~/dynamicWhitelist/test_db_vs_rpc_performance.py`

## Related Tests

- `examples/verify_slot_calculations.rs` - Verifies slot calculations only (no DB needed)
- `examples/validate_db_vs_rpc.rs` - Older validation example
- `~/dynamicWhitelist/test_db_vs_rpc_performance.py` - Performance benchmarking

## Next Steps

Once this test passes:
1. ✓ Slot calculations verified
2. ✓ DB reading verified
3. ✓ Data decoding verified
4. → Ready for production use!
5. → Run performance tests
6. → Integrate into dynamicWhitelist
