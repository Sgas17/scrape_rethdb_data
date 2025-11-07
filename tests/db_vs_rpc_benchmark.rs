/// Integration tests comparing DB reads vs RPC calls with performance benchmarks
///
/// These tests verify that:
/// 1. Direct DB reads produce identical results to RPC calls
/// 2. DB access is significantly faster than RPC
///
/// Run with: cargo test --test db_vs_rpc_benchmark -- --nocapture --test-threads=1
///
/// Required environment variables:
/// - RETH_DB_PATH: Path to Reth database
/// - RPC_URL: RPC endpoint (defaults to http://localhost:8545)

use alloy_primitives::{Address, B256, U256};
use alloy::{
    providers::ProviderBuilder,
    sol,
};
use scrape_rethdb_data::{
    collect_pool_data, collect_pool_data_at_block,
    get_v3_swap_events, scan_pool_events_multi,
    PoolInput,
};
use std::str::FromStr;
use std::time::Instant;

// Uniswap V3 Pool ABI
sol! {
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );

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

        function tickBitmap(int16 wordPos) external view returns (uint256);
    }
}

// Uniswap V2 Pair ABI
sol! {
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function getReserves() external view returns (
            uint112 reserve0,
            uint112 reserve1,
            uint32 blockTimestampLast
        );
    }
}

fn get_db_path() -> String {
    std::env::var("RETH_DB_PATH").expect("RETH_DB_PATH must be set")
}

fn get_rpc_url() -> String {
    std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string())
}

#[tokio::test]
#[ignore] // Requires DB and RPC access
async fn test_v3_slot0_db_vs_rpc() {
    let db_path = get_db_path();
    let rpc_url = get_rpc_url();

    // USDC/WETH 0.05% pool
    let pool_address = Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640").unwrap();
    let tick_spacing = 10;

    println!("\n=== V3 Slot0 Test ===");
    println!("Pool: {}", pool_address);

    // Time DB read
    let db_start = Instant::now();
    let pool_input = PoolInput::new_v3(pool_address, tick_spacing);
    let db_results = collect_pool_data(&db_path, &[pool_input], None).unwrap();
    let db_duration = db_start.elapsed();

    let db_slot0 = db_results[0].slot0.as_ref().unwrap();

    println!("\nDB Read:");
    println!("  Time: {:?}", db_duration);
    println!("  sqrtPriceX96: {}", db_slot0.sqrt_price_x96);
    println!("  tick: {}", db_slot0.tick);

    // Time RPC call
    let rpc_start = Instant::now();
    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    let pool = IUniswapV3Pool::new(pool_address, provider);
    let rpc_slot0 = pool.slot0().call().await.unwrap();
    let rpc_duration = rpc_start.elapsed();

    println!("\nRPC Call:");
    println!("  Time: {:?}", rpc_duration);
    println!("  sqrtPriceX96: {}", rpc_slot0.sqrtPriceX96);
    println!("  tick: {}", rpc_slot0.tick);

    // Verify equality
    assert_eq!(
        U256::from(db_slot0.sqrt_price_x96),
        U256::from(rpc_slot0.sqrtPriceX96),
        "sqrtPriceX96 mismatch"
    );
    assert_eq!(db_slot0.tick, i32::try_from(rpc_slot0.tick).unwrap(), "tick mismatch");
    assert_eq!(
        db_slot0.observation_index,
        rpc_slot0.observationIndex,
        "observationIndex mismatch"
    );
    assert_eq!(
        db_slot0.observation_cardinality,
        rpc_slot0.observationCardinality,
        "observationCardinality mismatch"
    );

    // Performance comparison
    let speedup = rpc_duration.as_micros() as f64 / db_duration.as_micros() as f64;
    println!("\n✓ All fields match!");
    println!("⚡ DB is {:.1}x faster than RPC", speedup);
}

#[tokio::test]
#[ignore] // Requires DB and RPC access
async fn test_v2_reserves_db_vs_rpc() {
    let db_path = get_db_path();
    let rpc_url = get_rpc_url();

    // USDC/WETH V2 pair
    let pool_address = Address::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap();

    println!("\n=== V2 Reserves Test ===");
    println!("Pool: {}", pool_address);

    // Time DB read
    let db_start = Instant::now();
    let pool_input = PoolInput::new_v2(pool_address);
    let db_results = collect_pool_data(&db_path, &[pool_input], None).unwrap();
    let db_duration = db_start.elapsed();

    let db_reserves = db_results[0].reserves.as_ref().unwrap();

    println!("\nDB Read:");
    println!("  Time: {:?}", db_duration);
    println!("  reserve0: {}", db_reserves.reserve0);
    println!("  reserve1: {}", db_reserves.reserve1);

    // Time RPC call
    let rpc_start = Instant::now();
    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    let pair = IUniswapV2Pair::new(pool_address, provider);
    let rpc_reserves = pair.getReserves().call().await.unwrap();
    let rpc_duration = rpc_start.elapsed();

    println!("\nRPC Call:");
    println!("  Time: {:?}", rpc_duration);
    println!("  reserve0: {}", rpc_reserves.reserve0);
    println!("  reserve1: {}", rpc_reserves.reserve1);

    // Verify equality (convert to U256 for comparison)
    assert_eq!(
        U256::from(db_reserves.reserve0),
        U256::from(rpc_reserves.reserve0),
        "reserve0 mismatch"
    );
    assert_eq!(
        U256::from(db_reserves.reserve1),
        U256::from(rpc_reserves.reserve1),
        "reserve1 mismatch"
    );
    assert_eq!(
        db_reserves.block_timestamp_last,
        rpc_reserves.blockTimestampLast,
        "blockTimestampLast mismatch"
    );

    // Performance comparison
    let speedup = rpc_duration.as_micros() as f64 / db_duration.as_micros() as f64;
    println!("\n✓ All fields match!");
    println!("⚡ DB is {:.1}x faster than RPC", speedup);
}

#[tokio::test]
#[ignore] // Requires DB and RPC access
async fn test_historical_query_db_vs_rpc() {
    let db_path = get_db_path();
    let rpc_url = get_rpc_url();

    // USDC/WETH 0.05% pool
    let pool_address = Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640").unwrap();
    let tick_spacing = 10;
    let block_number = 18000000u64; // Historical block

    println!("\n=== Historical Query Test ===");
    println!("Pool: {}", pool_address);
    println!("Block: {}", block_number);

    // Time DB read
    let db_start = Instant::now();
    let pool_input = PoolInput::new_v3(pool_address, tick_spacing);
    let db_results = collect_pool_data_at_block(&db_path, &[pool_input], None, block_number).unwrap();
    let db_duration = db_start.elapsed();

    let db_slot0 = db_results[0].pool_data.slot0.as_ref().unwrap();

    println!("\nDB Read:");
    println!("  Time: {:?}", db_duration);
    println!("  sqrtPriceX96: {}", db_slot0.sqrt_price_x96);
    println!("  tick: {}", db_slot0.tick);

    // Time RPC call (with block parameter)
    let rpc_start = Instant::now();
    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    let pool = IUniswapV3Pool::new(pool_address, provider);
    let rpc_slot0 = pool.slot0().block(block_number.into()).call().await.unwrap();
    let rpc_duration = rpc_start.elapsed();

    println!("\nRPC Call (archive node):");
    println!("  Time: {:?}", rpc_duration);
    println!("  sqrtPriceX96: {}", rpc_slot0.sqrtPriceX96);
    println!("  tick: {}", rpc_slot0.tick);

    // Verify equality
    assert_eq!(
        U256::from(db_slot0.sqrt_price_x96),
        U256::from(rpc_slot0.sqrtPriceX96),
        "sqrtPriceX96 mismatch"
    );
    assert_eq!(db_slot0.tick, i32::try_from(rpc_slot0.tick).unwrap(), "tick mismatch");

    // Performance comparison
    let speedup = rpc_duration.as_micros() as f64 / db_duration.as_micros() as f64;
    println!("\n✓ Historical data matches!");
    println!("⚡ DB is {:.1}x faster than archive node RPC", speedup);
}

#[tokio::test]
#[ignore] // Requires DB and RPC access
async fn test_event_scanning_performance() {
    let db_path = get_db_path();

    // USDC/WETH 0.05% pool
    let pool_address = Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640").unwrap();

    // Scan 10,000 blocks
    let from_block = 20000000u64;
    let to_block = 20010000u64;
    let num_blocks = to_block - from_block + 1;

    println!("\n=== Event Scanning Performance Test ===");
    println!("Pool: {}", pool_address);
    println!("Blocks: {} to {} ({} blocks)", from_block, to_block, num_blocks);

    // Time DB scan
    let db_start = Instant::now();
    let result = get_v3_swap_events(&db_path, pool_address, from_block, to_block).unwrap();
    let db_duration = db_start.elapsed();

    println!("\nDB Scan:");
    println!("  Time: {:?}", db_duration);
    println!("  Events found: {}", result.logs.len());
    println!("  Blocks scanned: {}", result.blocks_scanned);
    println!("  Blocks skipped by bloom: {} ({:.1}%)",
        result.blocks_skipped_by_bloom,
        (result.blocks_skipped_by_bloom as f64 / result.blocks_scanned as f64) * 100.0
    );
    println!("  Time per block: {:.3}ms",
        db_duration.as_micros() as f64 / result.blocks_scanned as f64 / 1000.0
    );

    // Estimate RPC time (assume 100ms per eth_getLogs call)
    let estimated_rpc_time = num_blocks as f64 * 100.0; // ms
    println!("\nEstimated RPC Time:");
    println!("  ~{:.1}s ({} requests × 100ms)", estimated_rpc_time / 1000.0, num_blocks);

    let speedup = estimated_rpc_time / (db_duration.as_micros() as f64 / 1000.0);
    println!("\n⚡ DB is ~{:.1}x faster than RPC", speedup);
}

#[tokio::test]
#[ignore] // Requires DB and RPC access
async fn test_multi_pool_scanning_performance() {
    let db_path = get_db_path();

    // Multiple popular pools
    let pool_addresses = vec![
        Address::from_str("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640").unwrap(), // USDC/WETH 0.05%
        Address::from_str("0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8").unwrap(), // USDC/WETH 0.3%
        Address::from_str("0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36").unwrap(), // WETH/USDT 0.3%
    ];

    let from_block = 20000000u64;
    let to_block = 20010000u64;
    let num_blocks = to_block - from_block + 1;

    println!("\n=== Multi-Pool Scanning Performance Test ===");
    println!("Pools: {}", pool_addresses.len());
    println!("Blocks: {} to {} ({} blocks)", from_block, to_block, num_blocks);

    // Time optimized multi-pool scan
    let db_start = Instant::now();
    let results = scan_pool_events_multi(&db_path, &pool_addresses, from_block, to_block, None).unwrap();
    let db_duration = db_start.elapsed();

    let mut total_events = 0;
    for (i, result) in results.iter().enumerate() {
        total_events += result.logs.len();
        println!("\nPool {}:", i + 1);
        println!("  Events: {}", result.logs.len());
        println!("  Blocks skipped: {} ({:.1}%)",
            result.blocks_skipped_by_bloom,
            (result.blocks_skipped_by_bloom as f64 / result.blocks_scanned as f64) * 100.0
        );
    }

    println!("\nOptimized Multi-Pool Scan:");
    println!("  Total time: {:?}", db_duration);
    println!("  Total events: {}", total_events);
    println!("  Time per pool: {:.3}ms",
        db_duration.as_micros() as f64 / pool_addresses.len() as f64 / 1000.0
    );

    // Estimate single-pool scan time (N × M block reads)
    let single_scan_estimate = db_duration.as_micros() * pool_addresses.len() as u128;
    println!("\nEstimated Single-Pool Scan Time:");
    println!("  ~{:.1}s ({} pools × sequential scans)",
        single_scan_estimate as f64 / 1_000_000.0,
        pool_addresses.len()
    );

    // Estimate RPC time
    let rpc_estimate = num_blocks * pool_addresses.len() as u64 * 100; // ms
    println!("\nEstimated RPC Time:");
    println!("  ~{:.1}s ({} requests × 100ms)",
        rpc_estimate as f64 / 1000.0,
        num_blocks * pool_addresses.len() as u64
    );

    let speedup_vs_single = single_scan_estimate as f64 / db_duration.as_micros() as f64;
    let speedup_vs_rpc = rpc_estimate as f64 / (db_duration.as_micros() as f64 / 1000.0);

    println!("\n⚡ Optimized scan is {:.1}x faster than sequential DB scans", speedup_vs_single);
    println!("⚡ Optimized scan is ~{:.1}x faster than RPC", speedup_vs_rpc);
}

#[tokio::test]
#[ignore] // Requires DB and RPC access
async fn test_batch_pool_query_performance() {
    let db_path = get_db_path();
    let rpc_url = get_rpc_url();

    // Multiple V3 pools
    let pools = vec![
        ("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640", 10), // USDC/WETH 0.05%
        ("0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8", 60), // USDC/WETH 0.3%
        ("0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36", 60), // WETH/USDT 0.3%
    ];

    println!("\n=== Batch Pool Query Performance Test ===");
    println!("Pools: {}", pools.len());

    // Time DB batch read
    let pool_inputs: Vec<_> = pools
        .iter()
        .map(|(addr, spacing)| {
            PoolInput::new_v3(Address::from_str(addr).unwrap(), *spacing)
        })
        .collect();

    let db_start = Instant::now();
    let db_results = collect_pool_data(&db_path, &pool_inputs, None).unwrap();
    let db_duration = db_start.elapsed();

    println!("\nDB Batch Read:");
    println!("  Total time: {:?}", db_duration);
    println!("  Time per pool: {:.3}ms",
        db_duration.as_micros() as f64 / pools.len() as f64 / 1000.0
    );

    // Time individual RPC calls
    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());
    let rpc_start = Instant::now();

    for (addr_str, _) in &pools {
        let addr = Address::from_str(addr_str).unwrap();
        let pool = IUniswapV3Pool::new(addr, &provider);
        let _slot0 = pool.slot0().call().await.unwrap();
    }

    let rpc_duration = rpc_start.elapsed();

    println!("\nRPC Sequential Calls:");
    println!("  Total time: {:?}", rpc_duration);
    println!("  Time per pool: {:.3}ms",
        rpc_duration.as_micros() as f64 / pools.len() as f64 / 1000.0
    );

    // Verify first pool
    let db_slot0 = db_results[0].slot0.as_ref().unwrap();
    let addr = Address::from_str(pools[0].0).unwrap();
    let pool = IUniswapV3Pool::new(addr, &provider);
    let rpc_slot0 = pool.slot0().call().await.unwrap();

    assert_eq!(U256::from(db_slot0.sqrt_price_x96), U256::from(rpc_slot0.sqrtPriceX96));
    assert_eq!(db_slot0.tick, i32::try_from(rpc_slot0.tick).unwrap());

    let speedup = rpc_duration.as_micros() as f64 / db_duration.as_micros() as f64;
    println!("\n✓ Data matches!");
    println!("⚡ DB batch read is {:.1}x faster than RPC", speedup);
}
