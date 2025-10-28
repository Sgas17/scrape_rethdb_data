use alloy_primitives::{Address, B256};
use eyre::Result;
use scrape_rethdb_data::{PoolInput, collect_pool_data};
use std::str::FromStr;

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let db_path = std::env::var("RETH_DB_PATH")
        .expect("RETH_DB_PATH must be set");

    // Test pool from validate_v4_bitmaps.py
    let pool_manager = Address::from_str("0x000000000004444c5dc75cB358380D2e3dE08A90")?;
    let pool_id = B256::from_str("0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d")?;
    let tick_spacing = 60;

    println!("Testing V4 Pool Data Collection");
    println!("================================");
    println!("Pool Manager: {}", pool_manager);
    println!("Pool ID: 0x{}", hex::encode(pool_id.as_slice()));
    println!("Tick Spacing: {}", tick_spacing);
    println!();

    // Create pool input
    let pool_input = PoolInput::new_v4(pool_manager, pool_id, tick_spacing);

    // Collect data
    println!("Collecting data from database...");
    let results = collect_pool_data(&db_path, &[pool_input], None)?;

    if results.is_empty() {
        println!("✗ No data returned!");
        return Ok(());
    }

    let pool_data = &results[0];

    // Display slot0 data
    if let Some(slot0) = &pool_data.slot0 {
        println!("\n✓ Slot0 Data:");
        println!("  sqrtPriceX96: {}", slot0.sqrt_price_x96);
        println!("  tick: {}", slot0.tick);
        println!("  observationIndex: {}", slot0.observation_index);
        println!("  observationCardinality: {}", slot0.observation_cardinality);
        println!("  observationCardinalityNext: {}", slot0.observation_cardinality_next);
        println!("  feeProtocol: {}", slot0.fee_protocol);
        println!("  unlocked: {}", slot0.unlocked);
    } else {
        println!("✗ No slot0 data");
    }

    // Display bitmap summary
    let bitmaps = &pool_data.bitmaps;
    println!("\n✓ Bitmaps: {} words", bitmaps.len());
    if !bitmaps.is_empty() {
        println!("  Sample bitmap words:");
        for bitmap in bitmaps.iter().take(5) {
            let bits_set = bitmap.bitmap.count_ones();
            println!("    Word {}: {} bits set (0x{:016x})",
                bitmap.word_pos, bits_set, bitmap.bitmap);
        }
    }

    // Display tick summary
    let ticks = &pool_data.ticks;
    println!("\n✓ Ticks: {} initialized", ticks.len());
    if !ticks.is_empty() {
        println!("  Sample ticks:");
        for tick in ticks.iter().take(5) {
            println!("    Tick {}: liquidityGross={}, liquidityNet={}, initialized={}",
                tick.tick,
                tick.liquidity_gross,
                tick.liquidity_net,
                tick.initialized
            );
        }
    }

    println!("\n✓✓✓ SUCCESS! V4 data collection working!");

    Ok(())
}
