#!/usr/bin/env python3
"""Test to understand the actual byte layout"""

# Raw hex from your output
raw_hex = "0x68fa676700000000015913f4500ad6dda7530000000000000000169a51a962f2"

# Remove 0x prefix
raw_hex = raw_hex[2:]
raw_bytes = bytes.fromhex(raw_hex)

print(f"Total bytes: {len(raw_bytes)}")
print(f"Hex: {raw_hex}")
print()

# Expected from RPC:
# Reserve0: 24852050830066
# Reserve1: 6365564567618318083923
# Timestamp: 1761240935

print("Expected values from RPC:")
print(f"  Reserve0: 24852050830066 = 0x{24852050830066:x}")
print(f"  Reserve1: 6365564567618318083923 = 0x{6365564567618318083923:x}")
print(f"  Timestamp: 1761240935 = 0x{1761240935:x}")
print()

# Try parsing as: reserve0 (14 bytes) | reserve1 (14 bytes) | timestamp (4 bytes)
reserve0_bytes = raw_bytes[0:14]
reserve1_bytes = raw_bytes[14:28]
timestamp_bytes = raw_bytes[28:32]

reserve0 = int.from_bytes(reserve0_bytes, 'big')
reserve1 = int.from_bytes(reserve1_bytes, 'big')
timestamp = int.from_bytes(timestamp_bytes, 'big')

print("Parsed as LEFT to RIGHT (reserve0|reserve1|timestamp):")
print(f"  Reserve0: {reserve0} = 0x{reserve0:x}")
print(f"  Reserve1: {reserve1} = 0x{reserve1:x}")
print(f"  Timestamp: {timestamp} = 0x{timestamp:x}")
print()

# Try parsing as RIGHT to LEFT: timestamp (4 bytes) | reserve1 (14 bytes) | reserve0 (14 bytes)
timestamp_bytes2 = raw_bytes[0:4]
reserve1_bytes2 = raw_bytes[4:18]
reserve0_bytes2 = raw_bytes[18:32]

timestamp2 = int.from_bytes(timestamp_bytes2, 'big')
reserve1_2 = int.from_bytes(reserve1_bytes2, 'big')
reserve0_2 = int.from_bytes(reserve0_bytes2, 'big')

print("Parsed as RIGHT to LEFT (timestamp|reserve1|reserve0):")
print(f"  Reserve0: {reserve0_2} = 0x{reserve0_2:x}")
print(f"  Reserve1: {reserve1_2} = 0x{reserve1_2:x}")
print(f"  Timestamp: {timestamp2} = 0x{timestamp2:x}")
