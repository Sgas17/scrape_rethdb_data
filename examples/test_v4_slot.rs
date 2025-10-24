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

    println!("Expected from Python:");
    println!("  0xbaaa5b5d3df4de7195a399b0e3e864e6ef2e9771be84e3b828cc32a43c58d300");
    println!();

    if hex::encode(slot0_slot.as_slice()) == "baaa5b5d3df4de7195a399b0e3e864e6ef2e9771be84e3b828cc32a43c58d300" {
        println!("✓ Slots match!");
    } else {
        println!("✗ Slots DO NOT match!");
    }
}
