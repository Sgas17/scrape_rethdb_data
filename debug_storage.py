#!/usr/bin/env python3
"""Debug storage parsing - compare raw storage to function calls"""

from web3 import Web3
from eth_abi import decode
import os
from dotenv import load_dotenv

load_dotenv()

rpc_url = os.getenv("RPC_URL", "http://localhost:8545")
w3 = Web3(Web3.HTTPProvider(rpc_url))

# V2 Pool
v2_pool = Web3.to_checksum_address("0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc")

print("=" * 80)
print("V2 USDC/ETH Pool Debug")
print("=" * 80)

# Method 1: getReserves() call
selector = Web3.keccak(text="getReserves()")[:4]
result = w3.eth.call({'to': v2_pool, 'data': selector})
reserve0, reserve1, timestamp = decode(['uint112', 'uint112', 'uint32'], result)

print(f"\ngetReserves() call:")
print(f"  Reserve0: {reserve0}")
print(f"  Reserve1: {reserve1}")
print(f"  Timestamp: {timestamp}")

# Method 2: Direct storage read at slot 8
storage_slot_8 = w3.eth.get_storage_at(v2_pool, 8)
print(f"\nRaw storage at slot 8:")
print(f"  Hex: {storage_slot_8.hex()}")
print(f"  Length: {len(storage_slot_8)} bytes")

# Parse the storage manually (packed: reserve0 | reserve1 | timestamp)
storage_bytes = storage_slot_8
# Last 4 bytes = timestamp
ts = int.from_bytes(storage_bytes[-4:], 'big')
# Next 14 bytes = reserve1 (112 bits)
r1 = int.from_bytes(storage_bytes[-18:-4], 'big')
# First 14 bytes = reserve0 (112 bits)
r0 = int.from_bytes(storage_bytes[:-18], 'big')

print(f"\nParsed from storage:")
print(f"  Reserve0: {r0}")
print(f"  Reserve1: {r1}")
print(f"  Timestamp: {ts}")

print("\n" + "=" * 80)
print("V3 USDC/ETH Pool Debug")
print("=" * 80)

v3_pool = Web3.to_checksum_address("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")

# slot0() call
selector = Web3.keccak(text="slot0()")[:4]
result = w3.eth.call({'to': v3_pool, 'data': selector})
sqrt_price, tick, *_ = decode(['uint160', 'int24', 'uint16', 'uint16', 'uint16', 'uint8', 'bool'], result)

print(f"\nslot0() call:")
print(f"  SqrtPriceX96: {hex(sqrt_price)}")
print(f"  Tick: {tick}")

# Check a specific tick
test_tick = -887270
selector = Web3.keccak(text="ticks(int24)")[:4]
from eth_abi import encode
data = selector + encode(['int24'], [test_tick])
result = w3.eth.call({'to': v3_pool, 'data': data})

print(f"\nticks({test_tick}) call:")
print(f"  Result hex: {result.hex()}")
print(f"  Result length: {len(result)} bytes")
print(f"  Is initialized: {int.from_bytes(result, 'big') != 0}")

# Check if result has actual tick data structure
if len(result) >= 32:
    # Try to decode the tick struct fields
    print(f"\nFirst 32 bytes (likely liquidityGross): {int.from_bytes(result[:32], 'big')}")
