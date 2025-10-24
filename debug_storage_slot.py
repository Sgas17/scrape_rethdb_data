#!/usr/bin/env python3
"""
Debug storage slot calculation - try different encodings
"""

from eth_utils import keccak
import struct


def method1_int256_encoding(word_pos: int, slot: int) -> str:
    """Standard Solidity encoding - what we're currently using"""
    # Encode word_pos as int256 (signed, 32 bytes)
    if word_pos < 0:
        word_pos_bytes = (word_pos & ((1 << 256) - 1)).to_bytes(32, 'big')
    else:
        word_pos_bytes = word_pos.to_bytes(32, 'big')

    slot_bytes = slot.to_bytes(32, 'big')

    data = word_pos_bytes + slot_bytes
    return '0x' + keccak(data).hex()


def method2_int16_raw(word_pos: int, slot: int) -> str:
    """Encode as actual int16 with sign extension"""
    # Convert to int16 (-32768 to 32767)
    word_pos_i16 = struct.pack('>h', word_pos)  # big-endian signed short

    # Sign extend to 32 bytes
    if word_pos < 0:
        word_pos_bytes = b'\xff' * 30 + word_pos_i16
    else:
        word_pos_bytes = b'\x00' * 30 + word_pos_i16

    slot_bytes = slot.to_bytes(32, 'big')

    data = word_pos_bytes + slot_bytes
    return '0x' + keccak(data).hex()


def method3_uint256_unsigned(word_pos: int, slot: int) -> str:
    """Encode word_pos as uint256 (unsigned)"""
    # For -347, this would be a huge positive number
    if word_pos < 0:
        word_pos_uint = (1 << 256) + word_pos
    else:
        word_pos_uint = word_pos

    word_pos_bytes = word_pos_uint.to_bytes(32, 'big')
    slot_bytes = slot.to_bytes(32, 'big')

    data = word_pos_bytes + slot_bytes
    return '0x' + keccak(data).hex()


def method4_abi_packed(word_pos: int, slot: int) -> str:
    """Try keccak256(abi.encodePacked(word_pos, slot)) - NO padding"""
    # This is NOT standard for mappings but let's test
    word_pos_i16 = struct.pack('>h', word_pos)  # 2 bytes
    slot_bytes = slot.to_bytes(1, 'big')  # 1 byte for slot

    data = word_pos_i16 + slot_bytes
    return '0x' + keccak(data).hex()


word_pos = -347
slot = 5

print("Debugging Storage Slot Calculation for word_pos = -347, slot = 5")
print("=" * 80)
print()

print(f"Method 1 (int256 standard): {method1_int256_encoding(word_pos, slot)}")
print(f"Method 2 (int16 with ext):  {method2_int16_raw(word_pos, slot)}")
print(f"Method 3 (uint256):         {method3_uint256_unsigned(word_pos, slot)}")
print(f"Method 4 (packed):          {method4_abi_packed(word_pos, slot)}")
print()

# Show the bytes for each method
print("Breakdown:")
print("-" * 80)

# Method 1
if word_pos < 0:
    word_pos_bytes = (word_pos & ((1 << 256) - 1)).to_bytes(32, 'big')
else:
    word_pos_bytes = word_pos.to_bytes(32, 'big')
print(f"Method 1 key bytes: {word_pos_bytes.hex()}")

# Method 2
word_pos_i16 = struct.pack('>h', word_pos)
if word_pos < 0:
    word_pos_bytes_m2 = b'\xff' * 30 + word_pos_i16
else:
    word_pos_bytes_m2 = b'\x00' * 30 + word_pos_i16
print(f"Method 2 key bytes: {word_pos_bytes_m2.hex()}")

print()
print("Note: -347 as int16 in hex:")
print(f"  Two's complement 16-bit: {struct.pack('>h', word_pos).hex()}")
print(f"  Decimal: {word_pos}")
print(f"  Binary: {bin(word_pos & 0xFFFF)}")

# Let's also test some positive values to see which method makes sense
print()
print("Testing positive word_pos = 100:")
print("-" * 80)
for i, method in enumerate([method1_int256_encoding, method2_int16_raw, method3_uint256_unsigned, method4_abi_packed], 1):
    print(f"Method {i}: {method(100, slot)}")
