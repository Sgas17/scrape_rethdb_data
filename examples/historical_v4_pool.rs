/// Example: Query historical Uniswap V4 pool state
///
/// This demonstrates querying V4 pool state at past block numbers.
/// V4 uses a singleton PoolManager contract with pool IDs instead of
/// individual pool contracts.

use alloy_primitives::{Address, B256};
use eyre::Result;
use scrape_rethdb_data::{collect_pool_data_at_block, PoolInput};
use std::str::FromStr;

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let db_path = std::env::var("RETH_DB_PATH").expect("RETH_DB_PATH must be set");

    // V4 PoolManager address
    let pool_manager =
        Address::from_str("0x000000000004444c5dc75cB358380D2e3dE08A90")?;

    // Pool ID (keccak256 of pool key)
    let pool_id = B256::from_str(
        "0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d",
    )?;

    let tick_spacing = 60;

    // Query state at historical blocks
    let blocks_to_query = vec![
        21000000, // Recent block
        21100000, // More recent
    ];

    println!("Historical V4 Pool State Query");
    println!("==============================");
    println!("Pool Manager: {}", pool_manager);
    println!("Pool ID: 0x{}", hex::encode(pool_id.as_slice()));
    println!("Tick Spacing: {}\n", tick_spacing);

    for block_number in blocks_to_query {
        println!("Querying state at block {}...", block_number);

        let pool_input = PoolInput::new_v4(pool_manager, tick_spacing);

        let results = collect_pool_data_at_block(
            &db_path,
            &[pool_input],
            Some(&[pool_id]),
            block_number,
        )?;

        if results.is_empty() {
            println!("  No data returned\n");
            continue;
        }

        let historical_data = &results[0];
        let pool_data = &historical_data.pool_data;

        // Display slot0 data
        if let Some(slot0) = &pool_data.slot0 {
            println!("  Block: {}", historical_data.block_number);
            println!("  sqrtPriceX96: {}", slot0.sqrt_price_x96);
            println!("  tick: {}", slot0.tick);
            println!("  lpFee: {}", slot0.observation_cardinality_next);
            println!("  Initialized ticks: {}", pool_data.ticks.len());
            println!("  Bitmap words: {}", pool_data.bitmaps.len());

            // Show sample ticks
            if !pool_data.ticks.is_empty() {
                println!("\n  Sample ticks:");
                for tick in pool_data.ticks.iter().take(3) {
                    println!(
                        "    Tick {}: liquidityGross={}, liquidityNet={}",
                        tick.tick, tick.liquidity_gross, tick.liquidity_net
                    );
                }
            }
            println!();
        }
    }

    println!("Success! Historical V4 data retrieved.");

    Ok(())
}
