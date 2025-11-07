/// Example: Query historical pool state at specific block numbers
///
/// This example demonstrates how to query pool state as it was at past block numbers,
/// which is useful for:
/// - Backtesting trading strategies
/// - Analyzing historical liquidity distributions
/// - Reconstructing past market conditions

use alloy_primitives::{Address, B256};
use eyre::Result;
use scrape_rethdb_data::{collect_pool_data_at_block, PoolInput};
use std::str::FromStr;

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let db_path = std::env::var("RETH_DB_PATH").expect("RETH_DB_PATH must be set");

    // Example: USDC/WETH 0.05% pool on Uniswap V3
    let pool_address =
        Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")?;
    let tick_spacing = 10;

    // Query state at multiple historical blocks
    let blocks_to_query = vec![
        18000000, // ~August 2023
        19000000, // ~January 2024
        20000000, // ~May 2024
    ];

    println!("Historical Pool State Query");
    println!("===========================");
    println!("Pool: {}", pool_address);
    println!("Protocol: Uniswap V3");
    println!("Tick Spacing: {}\n", tick_spacing);

    for block_number in blocks_to_query {
        println!("Querying state at block {}...", block_number);

        let pool_input = PoolInput::new_v3(pool_address, tick_spacing);

        let results =
            collect_pool_data_at_block(&db_path, &[pool_input], None, block_number)?;

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
            println!("  Initialized ticks: {}", pool_data.ticks.len());
            println!("  Bitmap words: {}", pool_data.bitmaps.len());
            println!();
        }
    }

    println!("Success! Historical data retrieved.");

    Ok(())
}
