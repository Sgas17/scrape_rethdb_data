# V4 Slot Calculation Issue - RESOLVED

## Summary

The V4 slot calculation in [src/storage.rs](src/storage.rs) was **CORRECT** all along. The test in [examples/test_v4_slot.rs](examples/test_v4_slot.rs) had an incorrect expected value that was causing confusion.

## The Issue

The test was expecting:
```
0xbaaa5b5d3df4de7195a399b0e3e864e6ef2e9771be84e3b828cc32a43c58d300
```

But the Rust code calculated:
```
0x7ced19e67a5796b90f206e133d76f6c105cb78d4f9f3e2074d49c272a8094b4e
```

## The Investigation

### Step 1: Verified Python calculation
Used Python with `eth_abi` and `web3` to independently calculate the slot:
```python
from eth_abi import encode
from web3 import Web3

pool_id = '0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d'
pool_id_bytes = bytes.fromhex(pool_id[2:])
pools_slot = 6

# Standard Solidity mapping encoding: keccak256(abi.encode(key, slot))
encoded = encode(['bytes32', 'uint256'], [pool_id_bytes, pools_slot])
base_slot = Web3.keccak(encoded)
# Result: 0x7ced19e67a5796b90f206e133d76f6c105cb78d4f9f3e2074d49c272a8094b4e
```

**Python matches Rust!** ✓

### Step 2: Verified against actual storage
Used `cast storage` to check what's actually in the blockchain:

```bash
# Check the "expected" slot - EMPTY!
cast storage 0x000000000004444c5dc75cB358380D2e3dE08A90 \
  0xbaaa5b5d3df4de7195a399b0e3e864e6ef2e9771be84e3b828cc32a43c58d300 \
  --rpc-url http://100.104.193.35:8545
# Result: 0x0000000000000000000000000000000000000000000000000000000000000000

# Check the Rust-calculated slot - HAS DATA!
cast storage 0x000000000004444c5dc75cB358380D2e3dE08A90 \
  0x7ced19e67a5796b90f206e133d76f6c105cb78d4f9f3e2074d49c272a8094b4e \
  --rpc-url http://100.104.193.35:8545
# Result: 0x000000000bb8000000fd0d82000000000000000000043153c045cb02615bf743
```

**Rust-calculated slot has real data!** ✓

### Step 3: Decoded the storage data
```python
data = '0x000000000bb8000000fd0d82000000000000000000043153c045cb02615bf743'

# V4 Slot0 (packed storage, right-to-left):
# - uint160 sqrtPriceX96 (last 20 bytes)
# - int24 tick (3 bytes)
# - uint24 protocolFee (3 bytes)
# - uint24 lpFee (3 bytes)

# Decoded:
#   sqrtPriceX96: 5068644170580286966069059
#   tick: -193150
#   protocolFee: 0
#   lpFee: 3000
```

### Step 4: Verified against RPC
Called `getSlot0(poolId)` via StateView contract:
```python
w3.eth.call({
    'to': '0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227',  # StateView
    'data': '0x' + selector + encode(['bytes32'], [pool_id_bytes]).hex()
})

# Result:
#   sqrtPriceX96: 5068644170580286966069059  ✓ MATCH
#   tick: -193150                             ✓ MATCH
#   protocolFee: 0                            ✓ MATCH
#   lpFee: 3000                               ✓ MATCH
```

**Perfect match with DB storage!** ✓✓✓

## The Correct Formula

For V4 pools, the storage slot calculation is:

```rust
// 1. Calculate base slot for the pool
// For mapping(PoolId => Pool.State) _pools at slot 6:
let encoded = (pool_id, U256::from(6)).abi_encode();
let base_slot = keccak256(&encoded);

// 2. For slot0 (offset 0 in Pool.State struct):
let slot0_slot = base_slot + 0;

// 3. For ticks mapping (offset 4):
let ticks_mapping_slot = base_slot + 4;
let tick_slot = keccak256(abi_encode(tick, ticks_mapping_slot));

// 4. For tickBitmap mapping (offset 5):
let bitmap_mapping_slot = base_slot + 5;
let bitmap_slot = keccak256(abi_encode(word_pos, bitmap_mapping_slot));
```

## Implementation Status

✅ **Rust calculation in [src/storage.rs](src/storage.rs:93-96) is CORRECT**

✅ **Test fixed in [examples/test_v4_slot.rs](examples/test_v4_slot.rs) with verified expected value**

✅ **V4 data collection verified working** (slot0, bitmaps, ticks all decode correctly)

## Where the Wrong Expected Value Came From

The incorrect expected value `0xbaaa5b5...` appears to have been from:
- A typo in documentation
- An older/incorrect calculation method
- Or possibly calculated for a different pool/slot combination

It was **NOT** based on actual blockchain storage or correct Solidity ABI encoding.

## Conclusion

**The Rust code was correct from the beginning.** The issue was purely in the test's expected value. All V4 functionality is working properly:

- Slot calculation ✓
- Storage reading ✓
- Data decoding ✓
- RPC verification ✓

No changes needed to production code, only the test file was updated.
