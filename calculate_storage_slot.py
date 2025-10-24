#!/usr/bin/env python3
"""
Calculate Solidity mapping storage slots

For a mapping at storage slot N:
  storage_slot = keccak256(abi.encode(key, slot))

Where:
  - key is the mapping key (padded to 32 bytes)
  - slot is the base storage slot number (padded to 32 bytes)
"""

from eth_hash.auto import keccak
import sys


def int_to_bytes32(value: int, signed: bool = False) -> bytes:
    """Convert an integer to 32-byte representation (big-endian)"""
    if signed and value < 0:
        # Two's complement for negative numbers
        value = (1 << 256) + value
    return value.to_bytes(32, byteorder='big', signed=False)


def calculate_mapping_slot(key: int, base_slot: int, key_signed: bool = False) -> str:
    """
    Calculate the storage slot for a mapping value.

    Args:
        key: The mapping key
        base_slot: The base storage slot of the mapping
        key_signed: Whether the key should be treated as a signed integer

    Returns:
        The storage slot as a hex string
    """
    # Encode key (32 bytes) + slot (32 bytes)
    key_bytes = int_to_bytes32(key, signed=key_signed)
    slot_bytes = int_to_bytes32(base_slot, signed=False)

    # Concatenate: key || slot
    data = key_bytes + slot_bytes

    # Hash with keccak256
    storage_slot = keccak(data)

    return '0x' + storage_slot.hex()


def main():
    # Example: tickBitmap mapping at word -347
    # For UniswapV3, tickBitmap is typically at storage slot 5

    word_pos = -347
    mapping_slot = 5

    print("Solidity Mapping Storage Slot Calculator")
    print("=" * 50)
    print()

    # Calculate for signed key (word position can be negative)
    storage_slot = calculate_mapping_slot(word_pos, mapping_slot, key_signed=True)

    print(f"Mapping: tickBitmap")
    print(f"Base storage slot: {mapping_slot}")
    print(f"Key (word position): {word_pos}")
    print(f"Key as bytes32 (signed): {int_to_bytes32(word_pos, signed=True).hex()}")
    print(f"Slot as bytes32: {int_to_bytes32(mapping_slot).hex()}")
    print()
    print(f"Calculated storage slot: {storage_slot}")
    print()

    # Show the calculation step by step
    print("Calculation breakdown:")
    print("-" * 50)
    key_bytes = int_to_bytes32(word_pos, signed=True)
    slot_bytes = int_to_bytes32(mapping_slot)
    concatenated = key_bytes + slot_bytes

    print(f"1. Key bytes (32 bytes):  {key_bytes.hex()}")
    print(f"2. Slot bytes (32 bytes): {slot_bytes.hex()}")
    print(f"3. Concatenated (64 bytes):")
    print(f"   {concatenated.hex()}")
    print(f"4. keccak256(concatenated): {storage_slot}")
    print()

    # Additional examples
    print("\nAdditional examples:")
    print("-" * 50)

    # Positive word position
    word_pos_positive = 100
    slot_positive = calculate_mapping_slot(word_pos_positive, mapping_slot, key_signed=True)
    print(f"Word position +100: {slot_positive}")

    # Another negative word position
    word_pos_neg = -1
    slot_neg = calculate_mapping_slot(word_pos_neg, mapping_slot, key_signed=True)
    print(f"Word position -1:   {slot_neg}")

    # For V3 ticks mapping (slot 4)
    tick = -887272  # MIN_TICK
    tick_slot = calculate_mapping_slot(tick, 4, key_signed=True)
    print(f"\nTick -887272 (slot 4): {tick_slot}")

    # For V3 tick 0
    tick_zero = 0
    tick_zero_slot = calculate_mapping_slot(tick_zero, 4, key_signed=True)
    print(f"Tick 0 (slot 4):       {tick_zero_slot}")


if __name__ == "__main__":
    # Allow command-line usage
    if len(sys.argv) >= 3:
        key = int(sys.argv[1])
        slot = int(sys.argv[2])
        signed = len(sys.argv) > 3 and sys.argv[3].lower() in ('true', 'yes', '1', 'signed')

        result = calculate_mapping_slot(key, slot, key_signed=signed)
        print(f"Key: {key}, Slot: {slot}, Signed: {signed}")
        print(f"Storage slot: {result}")
    else:
        main()
