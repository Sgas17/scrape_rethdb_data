use eyre::Result;
use scrape_rethdb_data::{collect_pool_data, PoolInput};

fn main() -> Result<()> {
    println!("=".repeat(80));
    println!("Uniswap Pool Data Collection Example");
    println!("=".repeat(80));

    // Get database path from environment
    let db_path = std::env::var("RETH_DB_PATH")
        .unwrap_or_else(|_| "/path/to/reth/db".to_string());

    println!("\nDatabase path: {}", db_path);

    // Define pools to collect data from
    let pools = vec![
        // UniswapV3 pool examples with different tick spacings
        PoolInput::new_v3(
            "0xa83326d20b7003bcecf1f4684a2fbb56161e2a8e".parse()?,
            60,
        ),
        PoolInput::new_v3(
            "0x7736b5006d90d5d5c0ee8148f1ea07ef82ab1677".parse()?,
            10,
        ),
        // UniswapV2 pool example
        PoolInput::new_v2(
            "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".parse()?,
        ),
    ];

    println!("\nCollecting data for {} pools...\n", pools.len());

    // Collect data
    let results = collect_pool_data(&db_path, &pools, None)?;

    // Display results
    for (idx, result) in results.iter().enumerate() {
        println!("Pool {} ({:?}):", idx + 1, result.address);
        println!("  Protocol: {:?}", result.protocol);

        match result.protocol {
            scrape_rethdb_data::Protocol::UniswapV2 => {
                if let Some(reserves) = &result.reserves {
                    println!("  Reserve0: {}", reserves.reserve0);
                    println!("  Reserve1: {}", reserves.reserve1);
                    println!("  Block Timestamp: {}", reserves.block_timestamp_last);
                }
            }
            scrape_rethdb_data::Protocol::UniswapV3
            | scrape_rethdb_data::Protocol::UniswapV4 => {
                if let Some(slot0) = &result.slot0 {
                    println!("  Current Tick: {}", slot0.tick);
                    println!("  Sqrt Price X96: {}", slot0.sqrt_price_x96);
                    println!("  Unlocked: {}", slot0.unlocked);
                }
                println!("  Initialized Ticks: {}", result.ticks.len());
                println!("  Bitmap Words: {}", result.bitmaps.len());

                // Show sample of ticks
                if !result.ticks.is_empty() {
                    println!("  Sample ticks:");
                    for tick in result.ticks.iter().take(5) {
                        println!("    Tick {}: initialized={}", tick.tick, tick.initialized);
                    }
                }

                // Show sample of bitmaps
                if !result.bitmaps.is_empty() {
                    println!("  Sample bitmaps:");
                    for bitmap in result.bitmaps.iter().take(3) {
                        println!("    Word {}: bitmap={:x}", bitmap.word_pos, bitmap.bitmap);
                    }
                }
            }
        }
        println!();
    }

    println!("=".repeat(80));
    println!("Collection complete!");
    println!("=".repeat(80));

    // Optionally, export to JSON
    if std::env::var("EXPORT_JSON").is_ok() {
        let json = serde_json::to_string_pretty(&results)?;
        std::fs::write("pool_data.json", json)?;
        println!("\nData exported to pool_data.json");
    }

    Ok(())
}
