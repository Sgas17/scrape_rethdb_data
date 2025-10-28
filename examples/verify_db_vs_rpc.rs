/// Verify that data collected from Reth DB matches RPC contract calls
///
/// This test:
/// 1. Gets slot0 from both DB and RPC (finds current tick)
/// 2. Generates test ticks around nearest initializable tick
/// 3. Collects tick data from DB using scrape_rethdb_data
/// 4. Calls RPC to get the same tick data
/// 5. Compares DB vs RPC results
/// 6. Does the same for bitmaps
///
/// Run on a machine with both Reth DB access and RPC access

use alloy_primitives::{Address, B256, U256, aliases::I24, Signed};
use alloy::{
    providers::{ProviderBuilder, Provider},
    sol,
};
use eyre::Result;
use scrape_rethdb_data::{collect_pool_data, PoolInput};
use std::str::FromStr;

// Define contract interfaces
sol! {
    #[sol(rpc)]
    contract IUniswapV3Pool {
        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );

        function tickSpacing() external view returns (int24);

        function ticks(int24 tick) external view returns (
            uint128 liquidityGross,
            int128 liquidityNet,
            uint256 feeGrowthOutside0X128,
            uint256 feeGrowthOutside1X128,
            int56 tickCumulativeOutside,
            uint160 secondsPerLiquidityOutsideX128,
            uint32 secondsOutside,
            bool initialized
        );

        function tickBitmap(int16 wordPosition) external view returns (uint256);
    }

    #[sol(rpc)]
    contract IUniswapV4StateView {
        function getSlot0(bytes32 poolId) external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint24 protocolFee,
            uint24 lpFee
        );

        function getTickLiquidity(bytes32 poolId, int24 tick) external view returns (
            uint128 liquidityGross,
            int128 liquidityNet
        );

        function getTickBitmap(bytes32 poolId, int16 wordPosition) external view returns (uint256);
    }
}

/// Calculate word position for a tick given tick spacing
fn tick_to_word_pos(tick: i32, tick_spacing: i32) -> i16 {
    let compressed = tick / tick_spacing;
    (compressed >> 8) as i16
}

async fn verify_v3_db_vs_rpc(
    provider: &impl Provider,
    db_path: &str,
    pool_address: Address,
) -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("V3 DATABASE vs RPC VERIFICATION");
    println!("{}", "=".repeat(80));
    println!("Pool: {}", pool_address);
    println!("DB Path: {}", db_path);

    let pool = IUniswapV3Pool::new(pool_address, provider);

    // Step 1: Get slot0 from RPC to find current tick
    println!("\n--- Step 1: Get Current Tick from RPC ---");
    let slot0_result = pool.slot0().call().await?;
    let current_tick: i32 = slot0_result.tick.as_i32();
    let tick_spacing: i32 = pool.tickSpacing().call().await?.as_i32();

    println!("Current tick: {}", current_tick);
    println!("Tick spacing: {}", tick_spacing);

    // Step 2: Find nearest initializable tick
    println!("\n--- Step 2: Find Nearest Initializable Tick ---");
    let tick_remainder = current_tick % tick_spacing;
    let nearest_initializable_tick = if tick_remainder == 0 {
        current_tick
    } else if tick_remainder > 0 {
        current_tick - tick_remainder
    } else {
        current_tick - (tick_spacing + tick_remainder)
    };

    println!("Nearest initializable tick: {}", nearest_initializable_tick);

    // Generate test ticks
    let mut test_ticks = Vec::new();
    for n in -5..=5 {
        let tick = nearest_initializable_tick + (tick_spacing * n);
        test_ticks.push(tick);
    }
    println!("Testing {} ticks: [{}, {}]", test_ticks.len(), test_ticks[0], test_ticks[test_ticks.len()-1]);

    // Step 3: Collect from DB
    println!("\n--- Step 3: Collect Data from Database ---");
    let pool_input = PoolInput::new_v3(pool_address, tick_spacing);
    let db_results = collect_pool_data(db_path, &[pool_input], None)?;

    if db_results.is_empty() {
        println!("✗ No data returned from DB!");
        return Ok(());
    }

    let db_data = &db_results[0];

    // Display DB slot0
    if let Some(slot0) = &db_data.slot0 {
        println!("✓ DB Slot0:");
        println!("  sqrtPriceX96: {}", slot0.sqrt_price_x96);
        println!("  tick: {}", slot0.tick);
        println!("  unlocked: {}", slot0.unlocked);
    }

    println!("✓ DB found {} ticks", db_data.ticks.len());
    println!("✓ DB found {} bitmap words", db_data.bitmaps.len());

    // Step 4: Compare slot0
    println!("\n--- Step 4: Compare Slot0 ---");
    if let Some(db_slot0) = &db_data.slot0 {
        let rpc_slot0 = pool.slot0().call().await?;

        // Convert U160 to U256 for comparison
        let rpc_sqrt: U256 = rpc_slot0.sqrtPriceX96.to();
        let sqrt_match = db_slot0.sqrt_price_x96 == rpc_sqrt;
        let tick_match = db_slot0.tick == rpc_slot0.tick.as_i32();

        println!("sqrtPriceX96: {} {}",
            if sqrt_match { "✓" } else { "✗" },
            if sqrt_match { "MATCH" } else { "MISMATCH" }
        );
        println!("  DB:  {}", db_slot0.sqrt_price_x96);
        println!("  RPC: {}", rpc_sqrt);

        println!("tick: {} {}",
            if tick_match { "✓" } else { "✗" },
            if tick_match { "MATCH" } else { "MISMATCH" }
        );
        println!("  DB:  {}", db_slot0.tick);
        println!("  RPC: {}", rpc_slot0.tick);
    }

    // Step 5: Compare specific test ticks
    println!("\n--- Step 5: Compare Test Ticks (DB vs RPC) ---");
    let mut matches = 0;
    let mut mismatches = 0;

    for &tick in &test_ticks {
        // Get from DB
        let db_tick = db_data.ticks.iter().find(|t| t.tick == tick);

        // Get from RPC
        let rpc_tick = pool.ticks(I24::unchecked_from(tick)).call().await?;

        let rpc_initialized = rpc_tick.liquidityGross > 0;
        let db_initialized = db_tick.is_some() && db_tick.unwrap().initialized;

        if rpc_initialized {
            if db_initialized {
                let db_t = db_tick.unwrap();
                // Convert Alloy types to native Rust types
                let rpc_gross: u128 = rpc_tick.liquidityGross;  // Already u128
                let net_bytes = rpc_tick.liquidityNet.to_be_bytes();
                let rpc_net = i128::from_be_bytes(net_bytes[..16].try_into().unwrap());

                let gross_match = db_t.liquidity_gross == rpc_gross;
                let net_match = db_t.liquidity_net == rpc_net;

                if gross_match && net_match {
                    println!("✓ Tick {}: MATCH", tick);
                    println!("    liquidityGross: {}", db_t.liquidity_gross);
                    println!("    liquidityNet: {}", db_t.liquidity_net);
                    matches += 1;
                } else {
                    println!("✗ Tick {}: MISMATCH", tick);
                    println!("    DB  liquidityGross: {}, liquidityNet: {}",
                        db_t.liquidity_gross, db_t.liquidity_net);
                    println!("    RPC liquidityGross: {}, liquidityNet: {}",
                        rpc_gross, rpc_net);
                    mismatches += 1;
                }
            } else {
                println!("✗ Tick {}: RPC says initialized, DB says not", tick);
                mismatches += 1;
            }
        } else {
            if !db_initialized {
                println!("  Tick {}: Not initialized (both agree)", tick);
            } else {
                println!("✗ Tick {}: DB says initialized, RPC says not", tick);
                mismatches += 1;
            }
        }
    }

    // Step 6: Compare bitmaps
    println!("\n--- Step 6: Compare Bitmaps (DB vs RPC) ---");
    let mut word_positions: Vec<i16> = test_ticks.iter()
        .map(|&tick| tick_to_word_pos(tick, tick_spacing))
        .collect();
    word_positions.sort();
    word_positions.dedup();

    println!("Testing {} unique word positions", word_positions.len());

    let mut bitmap_matches = 0;
    let mut bitmap_mismatches = 0;

    for word_pos in word_positions {
        // Get from DB
        let db_bitmap = db_data.bitmaps.iter()
            .find(|b| b.word_pos == word_pos)
            .map(|b| b.bitmap);

        // Get from RPC
        let rpc_bitmap = pool.tickBitmap(word_pos).call().await?;

        let db_value = db_bitmap.unwrap_or(U256::ZERO);

        if db_value == rpc_bitmap {
            if rpc_bitmap > U256::ZERO {
                println!("✓ Word {}: MATCH ({} bits set)", word_pos, rpc_bitmap.count_ones());
            }
            bitmap_matches += 1;
        } else {
            println!("✗ Word {}: MISMATCH", word_pos);
            println!("    DB:  0x{:064x}", db_value);
            println!("    RPC: 0x{:064x}", rpc_bitmap);
            bitmap_mismatches += 1;
        }
    }

    // Summary
    println!("\n{}", "=".repeat(80));
    println!("V3 SUMMARY");
    println!("{}", "=".repeat(80));
    println!("Ticks:   {} matches, {} mismatches", matches, mismatches);
    println!("Bitmaps: {} matches, {} mismatches", bitmap_matches, bitmap_mismatches);

    if mismatches == 0 && bitmap_mismatches == 0 {
        println!("\n✓✓✓ ALL CHECKS PASSED! DB data matches RPC perfectly!");
    } else {
        println!("\n✗ Some mismatches detected - needs investigation");
    }

    Ok(())
}

async fn verify_v4_db_vs_rpc(
    provider: &impl Provider,
    db_path: &str,
    pool_manager: Address,
    pool_id: B256,
    stateview_address: Address,
    tick_spacing: i32,
) -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("V4 DATABASE vs RPC VERIFICATION");
    println!("{}", "=".repeat(80));
    println!("PoolManager: {}", pool_manager);
    println!("PoolId: 0x{}", hex::encode(pool_id.as_slice()));
    println!("DB Path: {}", db_path);

    let stateview = IUniswapV4StateView::new(stateview_address, provider);

    // Step 1: Get slot0 from RPC
    println!("\n--- Step 1: Get Current Tick from RPC ---");
    let slot0_result = stateview.getSlot0(pool_id).call().await?;
    let current_tick: i32 = slot0_result.tick.as_i32();

    println!("Current tick: {}", current_tick);
    println!("Tick spacing: {}", tick_spacing);

    // Step 2: Find nearest initializable tick
    println!("\n--- Step 2: Find Nearest Initializable Tick ---");
    let tick_remainder = current_tick % tick_spacing;
    let nearest_initializable_tick = if tick_remainder == 0 {
        current_tick
    } else if tick_remainder > 0 {
        current_tick - tick_remainder
    } else {
        current_tick - (tick_spacing + tick_remainder)
    };

    println!("Nearest initializable tick: {}", nearest_initializable_tick);

    // Generate test ticks
    let mut test_ticks = Vec::new();
    for n in -5..=5 {
        let tick = nearest_initializable_tick + (tick_spacing * n);
        test_ticks.push(tick);
    }
    println!("Testing {} ticks: [{}, {}]", test_ticks.len(), test_ticks[0], test_ticks[test_ticks.len()-1]);

    // Step 3: Collect from DB
    println!("\n--- Step 3: Collect Data from Database ---");
    let pool_input = PoolInput::new_v4(pool_manager, tick_spacing);
    let db_results = collect_pool_data(db_path, &[pool_input], Some(&[pool_id]))?;

    if db_results.is_empty() {
        println!("✗ No data returned from DB!");
        return Ok(());
    }

    let db_data = &db_results[0];

    // Display DB slot0
    if let Some(slot0) = &db_data.slot0 {
        println!("✓ DB Slot0:");
        println!("  sqrtPriceX96: {}", slot0.sqrt_price_x96);
        println!("  tick: {}", slot0.tick);
    }

    println!("✓ DB found {} ticks", db_data.ticks.len());
    println!("✓ DB found {} bitmap words", db_data.bitmaps.len());

    // Step 4: Compare slot0
    println!("\n--- Step 4: Compare Slot0 ---");
    if let Some(db_slot0) = &db_data.slot0 {
        let rpc_slot0 = stateview.getSlot0(pool_id).call().await?;

        // Convert U160 to U256 for comparison
        let rpc_sqrt: U256 = rpc_slot0.sqrtPriceX96.to();
        let sqrt_match = db_slot0.sqrt_price_x96 == rpc_sqrt;
        let tick_match = db_slot0.tick == rpc_slot0.tick.as_i32();

        println!("sqrtPriceX96: {} {}",
            if sqrt_match { "✓" } else { "✗" },
            if sqrt_match { "MATCH" } else { "MISMATCH" }
        );
        println!("  DB:  {}", db_slot0.sqrt_price_x96);
        println!("  RPC: {}", rpc_sqrt);

        println!("tick: {} {}",
            if tick_match { "✓" } else { "✗" },
            if tick_match { "MATCH" } else { "MISMATCH" }
        );
        println!("  DB:  {}", db_slot0.tick);
        println!("  RPC: {}", rpc_slot0.tick);
    }

    // Step 5: Compare specific test ticks
    println!("\n--- Step 5: Compare Test Ticks (DB vs RPC) ---");
    let mut matches = 0;
    let mut mismatches = 0;

    for &tick in &test_ticks {
        // Get from DB
        let db_tick = db_data.ticks.iter().find(|t| t.tick == tick);

        // Get from RPC
        let rpc_tick = stateview.getTickLiquidity(pool_id, I24::unchecked_from(tick)).call().await?;

        let rpc_initialized = rpc_tick.liquidityGross > 0;
        let db_initialized = db_tick.is_some() && db_tick.unwrap().initialized;

        if rpc_initialized {
            if db_initialized {
                let db_t = db_tick.unwrap();
                // Convert Alloy types to native Rust types
                let rpc_gross: u128 = rpc_tick.liquidityGross;  // Already u128
                let net_bytes = rpc_tick.liquidityNet.to_be_bytes();
                let rpc_net = i128::from_be_bytes(net_bytes[..16].try_into().unwrap());

                let gross_match = db_t.liquidity_gross == rpc_gross;
                let net_match = db_t.liquidity_net == rpc_net;

                if gross_match && net_match {
                    println!("✓ Tick {}: MATCH", tick);
                    println!("    liquidityGross: {}", db_t.liquidity_gross);
                    println!("    liquidityNet: {}", db_t.liquidity_net);
                    matches += 1;
                } else {
                    println!("✗ Tick {}: MISMATCH", tick);
                    println!("    DB  liquidityGross: {}, liquidityNet: {}",
                        db_t.liquidity_gross, db_t.liquidity_net);
                    println!("    RPC liquidityGross: {}, liquidityNet: {}",
                        rpc_gross, rpc_net);
                    mismatches += 1;
                }
            } else {
                println!("✗ Tick {}: RPC says initialized, DB says not", tick);
                mismatches += 1;
            }
        } else {
            if !db_initialized {
                println!("  Tick {}: Not initialized (both agree)", tick);
            } else {
                println!("✗ Tick {}: DB says initialized, RPC says not", tick);
                mismatches += 1;
            }
        }
    }

    // Step 6: Compare bitmaps
    println!("\n--- Step 6: Compare Bitmaps (DB vs RPC) ---");
    let mut word_positions: Vec<i16> = test_ticks.iter()
        .map(|&tick| tick_to_word_pos(tick, tick_spacing))
        .collect();
    word_positions.sort();
    word_positions.dedup();

    println!("Testing {} unique word positions", word_positions.len());

    let mut bitmap_matches = 0;
    let mut bitmap_mismatches = 0;

    for word_pos in word_positions {
        // Get from DB
        let db_bitmap = db_data.bitmaps.iter()
            .find(|b| b.word_pos == word_pos)
            .map(|b| b.bitmap);

        // Get from RPC
        let rpc_bitmap = stateview.getTickBitmap(pool_id, word_pos).call().await?;

        let db_value = db_bitmap.unwrap_or(U256::ZERO);

        if db_value == rpc_bitmap {
            if rpc_bitmap > U256::ZERO {
                println!("✓ Word {}: MATCH ({} bits set)", word_pos, rpc_bitmap.count_ones());
            }
            bitmap_matches += 1;
        } else {
            println!("✗ Word {}: MISMATCH", word_pos);
            println!("    DB:  0x{:064x}", db_value);
            println!("    RPC: 0x{:064x}", rpc_bitmap);
            bitmap_mismatches += 1;
        }
    }

    // Summary
    println!("\n{}", "=".repeat(80));
    println!("V4 SUMMARY");
    println!("{}", "=".repeat(80));
    println!("Ticks:   {} matches, {} mismatches", matches, mismatches);
    println!("Bitmaps: {} matches, {} mismatches", bitmap_matches, bitmap_mismatches);

    if mismatches == 0 && bitmap_mismatches == 0 {
        println!("\n✓✓✓ ALL CHECKS PASSED! DB data matches RPC perfectly!");
    } else {
        println!("\n✗ Some mismatches detected - needs investigation");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8545".to_string());
    let db_path = std::env::var("RETH_DB_PATH")
        .expect("RETH_DB_PATH must be set");

    println!("Connecting to RPC: {}", rpc_url);
    println!("Using DB: {}", db_path);
    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);

    // Test V3 pool (USDC/WETH 0.05%)
    println!("\n{}", "=".repeat(80));
    println!("STARTING V3 VERIFICATION");
    println!("{}", "=".repeat(80));

    let v3_pool = Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640")?;
    verify_v3_db_vs_rpc(&provider, &db_path, v3_pool).await?;

    // Test V4 pool
    println!("\n{}", "=".repeat(80));
    println!("STARTING V4 VERIFICATION");
    println!("{}", "=".repeat(80));

    let v4_pool_manager = Address::from_str("0x000000000004444c5dc75cB358380D2e3dE08A90")?;
    let v4_pool_id = B256::from_str("0xdce6394339af00981949f5f3baf27e3610c76326a700af57e4b3e3ae4977f78d")?;
    let v4_stateview = Address::from_str("0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227")?;
    let v4_tick_spacing = 60;

    verify_v4_db_vs_rpc(&provider, &db_path, v4_pool_manager, v4_pool_id, v4_stateview, v4_tick_spacing).await?;

    println!("\n{}", "=".repeat(80));
    println!("ALL VERIFICATIONS COMPLETE");
    println!("{}", "=".repeat(80));

    Ok(())
}
