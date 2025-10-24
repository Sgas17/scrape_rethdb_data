#!/usr/bin/env python3
"""
Test and compare storage slot calculations between Python and what we expect
"""

from eth_utils import keccak
from eth_abi import encode


def calculate_mapping_slot_solidity_style(key: int, base_slot: int, key_type: str = "int256") -> str:
    """
    Calculate storage slot the way Solidity does it.

    Solidity always encodes mapping keys as full 32-byte values (uint256/int256).
    Even if the key type is int16, it's still encoded as int256 for hashing.
    """
    if key_type in ["int16", "int24", "int256"]:
        # Encode as signed integer (int256) - this handles negative values
        encoded_key = encode(['int256'], [key])
    else:
        # Encode as unsigned integer (uint256)
        encoded_key = encode(['uint256'], [key])

    # Encode slot as uint256
    encoded_slot = encode(['uint256'], [base_slot])

    # Concatenate and hash
    data = encoded_key + encoded_slot
    storage_slot = keccak(data)

    return '0x' + storage_slot.hex()


# Test for word position -347 at slot 5
word_pos = -347
mapping_slot = 5

print("Testing Storage Slot Calculation for tickBitmap")
print("=" * 70)
print(f"Word position: {word_pos}")
print(f"Mapping slot: {mapping_slot}")
print()

# Method 1: Treating key as int256 (what Solidity actually does)
slot_int256 = calculate_mapping_slot_solidity_style(word_pos, mapping_slot, "int256")
print(f"Method 1 (int256 key): {slot_int256}")

# Method 2: Let's also check what we get if we incorrectly use the raw value
# This is what our Rust code might be doing wrong
print()
print("Breakdown of Method 1 (correct Solidity way):")
print("-" * 70)
encoded_key = encode(['int256'], [word_pos])
encoded_slot = encode(['uint256'], [mapping_slot])
print(f"Key as int256 (32 bytes): {encoded_key.hex()}")
print(f"Slot as uint256 (32 bytes): {encoded_slot.hex()}")
print(f"Concatenated (64 bytes): {(encoded_key + encoded_slot).hex()}")
print(f"keccak256: {slot_int256}")
