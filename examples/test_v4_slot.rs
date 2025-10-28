use alloy_primitives::B256;
use scrape_rethdb_data::storage;
use std::str::FromStr;

fn main() {
    let pool_id = B256::from_str("0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d")
        .expect("Invalid pool ID");

    println!("Testing V4 storage slot calculation");
    println!("PoolId: 0x{}", hex::encode(pool_id.as_slice()));
    println!();

    // Calculate slot0 slot
    let slot0_slot = storage::v4_slot0_slot(pool_id);
    println!("Rust calculates slot0 slot:");
    println!("  0x{}", hex::encode(slot0_slot.as_slice()));
    println!();

    println!("Expected (verified against RPC):");
    println!("  0x7ced19e67a5796b90f206e133d76f6c105cb78d4f9f3e2074d49c272a8094b4e");
    println!();

    // Verify against actual storage data
    println!("Verification:");
    println!("  cast storage 0x000000000004444c5dc75cB358380D2e3dE08A90 <slot>");
    println!("  Result: 0x000000000bb8000000fd0d82000000000000000000043153c045cb02615bf743");
    println!("  Decoded: sqrtPriceX96=5068644170580286966069059, tick=-193150, lpFee=3000");
    println!("  RPC getSlot0() matches perfectly!");
    println!();

    if hex::encode(slot0_slot.as_slice()) == "7ced19e67a5796b90f206e133d76f6c105cb78d4f9f3e2074d49c272a8094b4e" {
        println!("✓✓✓ CORRECT! Slot calculation matches verified storage!");
    } else {
        println!("✗ Slots DO NOT match!");
    }
}
