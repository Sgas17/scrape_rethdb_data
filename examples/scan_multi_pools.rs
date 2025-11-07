/// Example: Scan multiple pools for events - OPTIMIZED
///
/// This example demonstrates the optimized multi-address event scanning.
/// Instead of scanning each pool separately (N * M block reads for N pools and M blocks),
/// this scans all pools at once (M block reads), making it ~N times faster!
///
/// Perfect for:
/// - Monitoring many pools simultaneously
/// - Collecting market-wide data
/// - Building event indexes for multiple addresses

use alloy_primitives::Address;
use eyre::Result;
use scrape_rethdb_data::scan_pool_events_multi;
use std::str::FromStr;

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let db_path = std::env::var("RETH_DB_PATH").expect("RETH_DB_PATH must be set");

    // Example: Multiple popular Uniswap V3 pools
    let pools = vec![
        "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640", // USDC/WETH 0.05%
        "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8", // USDC/WETH 0.3%
        "0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36", // WETH/USDT 0.3%
    ];

    let pool_addresses: Result<Vec<Address>, _> =
        pools.iter().map(|addr| Address::from_str(addr)).collect();
    let pool_addresses = pool_addresses?;

    // Scan 10,000 blocks
    let from_block = 20000000;
    let to_block = 20010000;

    println!("Multi-Pool Event Scanning (OPTIMIZED)");
    println!("======================================");
    println!("Number of pools: {}", pool_addresses.len());
    println!("Block range: {} to {}", from_block, to_block);
    println!("Blocks to scan: {}\n", to_block - from_block + 1);

    println!("Scanning all pools simultaneously...");
    let results =
        scan_pool_events_multi(&db_path, &pool_addresses, from_block, to_block, None)?;

    println!("\nResults:");
    println!("--------");

    let mut total_events = 0;
    for (i, result) in results.iter().enumerate() {
        println!("\nPool {}: {}", i + 1, pools[i]);
        println!("  Events found: {}", result.logs.len());
        println!("  Blocks scanned: {}", result.blocks_scanned);
        println!(
            "  Blocks skipped by bloom filter: {} ({:.1}%)",
            result.blocks_skipped_by_bloom,
            (result.blocks_skipped_by_bloom as f64 / result.blocks_scanned as f64) * 100.0
        );
        total_events += result.logs.len();
    }

    println!("\n========");
    println!("Total events found across all pools: {}", total_events);
    println!("\nPerformance Note:");
    println!(
        "This optimized scan reads each block ONCE instead of {} times!",
        pool_addresses.len()
    );
    println!("That's ~{}x faster than scanning pools separately.", pool_addresses.len());

    Ok(())
}
