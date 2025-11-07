/// Example: Scan for Uniswap V3 Swap events
///
/// This example demonstrates how to efficiently scan for Swap events
/// from Uniswap V3 pools using direct database access.
///
/// Features:
/// - Bloom filter optimization (skips irrelevant blocks)
/// - Direct DB access (faster than RPC)
/// - Event decoding with transaction metadata

use alloy_primitives::Address;
use eyre::Result;
use scrape_rethdb_data::get_v3_swap_events;
use std::str::FromStr;

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let db_path = std::env::var("RETH_DB_PATH").expect("RETH_DB_PATH must be set");

    // Example: USDC/WETH 0.05% pool on Uniswap V3
    let pool_address =
        Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")?;

    // Scan 10,000 blocks for Swap events
    let from_block = 20000000;
    let to_block = 20010000;

    println!("Scanning for Uniswap V3 Swap Events");
    println!("===================================");
    println!("Pool: {}", pool_address);
    println!("Block range: {} to {}", from_block, to_block);
    println!("Blocks to scan: {}\n", to_block - from_block + 1);

    println!("Scanning...");
    let result = get_v3_swap_events(&db_path, pool_address, from_block, to_block)?;

    println!("\nResults:");
    println!("--------");
    println!("Swap events found: {}", result.logs.len());
    println!("Blocks scanned: {}", result.blocks_scanned);
    println!(
        "Blocks skipped by bloom filter: {} ({:.1}%)",
        result.blocks_skipped_by_bloom,
        (result.blocks_skipped_by_bloom as f64 / result.blocks_scanned as f64) * 100.0
    );
    println!();

    // Show first few swap events
    if !result.logs.is_empty() {
        println!("Sample swap events:");
        for (i, event) in result.logs.iter().take(5).enumerate() {
            println!(
                "  {}. Block: {}, TxIndex: {}, Topics: {}",
                i + 1,
                event.block_number,
                event.transaction_index,
                event.log.data.topics().len()
            );
        }
    }

    println!("\nSuccess! Event scan complete.");

    Ok(())
}
