/// Validation example: Compare DB reads vs RPC calls
///
/// This example reads pool data directly from the reth database and compares
/// it with data fetched via RPC to ensure our decoding logic is correct.

use alloy::{
    primitives::Address,
    providers::ProviderBuilder,
    sol,
};
use eyre::Result;
use scrape_rethdb_data::{collect_pool_data, PoolInput};
use std::env;

// Define contract interfaces using sol! macro
sol! {
    #[sol(rpc)]
    contract IUniswapV2Pair {
        function getReserves() external view returns (
            uint112 reserve0,
            uint112 reserve1,
            uint32 blockTimestampLast
        );
    }

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
    }

    #[sol(rpc)]
    contract IUniswapV4PoolManager {
        function getSlot0(bytes32 poolId) external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Get configuration from environment
    let db_path = env::var("RETH_DB_PATH").expect("RETH_DB_PATH not set");
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());

    println!("🔍 Validation: DB vs RPC");
    println!("DB Path: {}", db_path);
    println!("RPC URL: {}", rpc_url);
    println!();

    // Setup RPC provider
    let provider = ProviderBuilder::new().on_http(rpc_url.parse()?);

    // Test V2 Pool: USDC-WETH
    let v2_pool_address: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".parse()?;
    println!("📊 Testing V2 Pool: {}", v2_pool_address);
    validate_v2_pool(&provider, &db_path, v2_pool_address).await?;
    println!();

    // Test V3 Pool: USDC-WETH 0.05%
    let v3_pool_address: Address = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".parse()?;
    println!("📊 Testing V3 Pool: {}", v3_pool_address);
    validate_v3_pool(&provider, &db_path, v3_pool_address, 10).await?;
    println!();

    // Test V3 Pool: USDC-WETH 0.3%
    let v3_pool_address_30bps: Address = "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8".parse()?;
    println!("📊 Testing V3 Pool (0.3%): {}", v3_pool_address_30bps);
    validate_v3_pool(&provider, &db_path, v3_pool_address_30bps, 60).await?;
    println!();

    // Test V4 Pool (if V4_POOL_MANAGER and V4_POOL_ID are set)
    if let (Ok(pool_manager), Ok(pool_id_hex)) = (
        env::var("V4_POOL_MANAGER"),
        env::var("V4_POOL_ID"),
    ) {
        let pool_manager_address: Address = pool_manager.parse()?;
        let pool_id: alloy_primitives::FixedBytes<32> = pool_id_hex.parse()?;
        println!("📊 Testing V4 Pool");
        println!("  PoolManager: {}", pool_manager_address);
        println!("  PoolId: {}", pool_id_hex);
        validate_v4_pool(&provider, &db_path, pool_manager_address, pool_id, 60).await?;
        println!();
    } else {
        println!("⏭️  Skipping V4 test (V4_POOL_MANAGER and V4_POOL_ID not set)");
        println!();
    }

    println!("✅ All validations passed!");

    Ok(())
}

/// Validate V2 pool data: DB vs RPC
async fn validate_v2_pool(
    provider: &impl alloy::providers::Provider,
    db_path: &str,
    pool_address: Address,
) -> Result<()> {
    // Read from DB
    let pool_input = PoolInput::new_v2(pool_address);
    let db_results = collect_pool_data(db_path, &[pool_input], None)?;
    let db_data = &db_results[0];

    // Read from RPC
    let contract = IUniswapV2Pair::new(pool_address, provider);
    let rpc_result = contract.getReserves().call().await?;

    // Extract DB values
    let db_reserves = db_data.reserves.as_ref().expect("V2 should have reserves");

    // Compare
    println!("  Reserve0:");
    println!("    DB:  {}", db_reserves.reserve0);
    println!("    RPC: {}", rpc_result.reserve0);
    assert_eq!(
        db_reserves.reserve0,
        rpc_result.reserve0.to::<u128>(),
        "Reserve0 mismatch!"
    );

    println!("  Reserve1:");
    println!("    DB:  {}", db_reserves.reserve1);
    println!("    RPC: {}", rpc_result.reserve1);
    assert_eq!(
        db_reserves.reserve1,
        rpc_result.reserve1.to::<u128>(),
        "Reserve1 mismatch!"
    );

    println!("  Timestamp:");
    println!("    DB:  {}", db_reserves.block_timestamp_last);
    println!("    RPC: {}", rpc_result.blockTimestampLast);
    assert_eq!(
        db_reserves.block_timestamp_last, rpc_result.blockTimestampLast,
        "Timestamp mismatch!"
    );

    println!("  ✅ V2 validation passed");

    Ok(())
}

/// Validate V3 pool data: DB vs RPC
async fn validate_v3_pool(
    provider: &impl alloy::providers::Provider,
    db_path: &str,
    pool_address: Address,
    tick_spacing: i32,
) -> Result<()> {
    // Read from DB
    let pool_input = PoolInput::new_v3(pool_address, tick_spacing);
    let db_results = collect_pool_data(db_path, &[pool_input], None)?;
    let db_data = &db_results[0];

    // Read from RPC
    let contract = IUniswapV3Pool::new(pool_address, provider);
    let rpc_result = contract.slot0().call().await?;

    // Extract DB values
    let db_slot0 = db_data.slot0.as_ref().expect("V3 should have slot0");

    // Compare slot0 fields
    println!("  sqrtPriceX96:");
    println!("    DB:  {}", db_slot0.sqrt_price_x96);
    println!("    RPC: {}", rpc_result.sqrtPriceX96);
    // Convert Uint<160> to U256 for comparison
    let rpc_sqrt_price = alloy_primitives::U256::from(rpc_result.sqrtPriceX96);
    assert_eq!(
        db_slot0.sqrt_price_x96,
        rpc_sqrt_price,
        "sqrtPriceX96 mismatch!"
    );

    println!("  tick:");
    println!("    DB:  {}", db_slot0.tick);
    println!("    RPC: {}", rpc_result.tick);
    // Convert Alloy's Signed<24, 1> to i32 for comparison
    let rpc_tick_i32: i32 = rpc_result.tick.as_i32();
    assert_eq!(db_slot0.tick, rpc_tick_i32, "Tick mismatch!");

    println!("  observationIndex:");
    println!("    DB:  {}", db_slot0.observation_index);
    println!("    RPC: {}", rpc_result.observationIndex);
    assert_eq!(
        db_slot0.observation_index, rpc_result.observationIndex,
        "observationIndex mismatch!"
    );

    println!("  observationCardinality:");
    println!("    DB:  {}", db_slot0.observation_cardinality);
    println!("    RPC: {}", rpc_result.observationCardinality);
    assert_eq!(
        db_slot0.observation_cardinality, rpc_result.observationCardinality,
        "observationCardinality mismatch!"
    );

    println!("  observationCardinalityNext:");
    println!("    DB:  {}", db_slot0.observation_cardinality_next);
    println!("    RPC: {}", rpc_result.observationCardinalityNext);
    assert_eq!(
        db_slot0.observation_cardinality_next, rpc_result.observationCardinalityNext,
        "observationCardinalityNext mismatch!"
    );

    println!("  feeProtocol:");
    println!("    DB:  {}", db_slot0.fee_protocol);
    println!("    RPC: {}", rpc_result.feeProtocol);
    assert_eq!(
        db_slot0.fee_protocol, rpc_result.feeProtocol,
        "feeProtocol mismatch!"
    );

    println!("  unlocked:");
    println!("    DB:  {}", db_slot0.unlocked);
    println!("    RPC: {}", rpc_result.unlocked);
    assert_eq!(
        db_slot0.unlocked, rpc_result.unlocked,
        "unlocked mismatch!"
    );

    // Report bitmap and tick counts
    println!("  Bitmaps found: {}", db_data.bitmaps.len());
    println!("  Ticks found: {}", db_data.ticks.len());

    println!("  ✅ V3 validation passed");

    Ok(())
}

/// Validate V4 pool data: DB vs RPC
async fn validate_v4_pool(
    provider: &impl alloy::providers::Provider,
    db_path: &str,
    pool_manager: Address,
    pool_id: alloy_primitives::FixedBytes<32>,
    tick_spacing: i32,
) -> Result<()> {
    // Read from DB
    let pool_input = PoolInput::new_v4(pool_manager, tick_spacing);
    let db_results = collect_pool_data(
        db_path,
        &[pool_input],
        Some(&[alloy_primitives::B256::from(pool_id)]),
    )?;
    let db_data = &db_results[0];

    // Read from RPC
    let contract = IUniswapV4PoolManager::new(pool_manager, provider);
    let rpc_result = contract.getSlot0(pool_id).call().await?;

    // Extract DB values
    let db_slot0 = db_data.slot0.as_ref().expect("V4 should have slot0");

    // Compare slot0 fields (same as V3)
    println!("  sqrtPriceX96:");
    println!("    DB:  {}", db_slot0.sqrt_price_x96);
    println!("    RPC: {}", rpc_result.sqrtPriceX96);
    let rpc_sqrt_price = alloy_primitives::U256::from(rpc_result.sqrtPriceX96);
    assert_eq!(
        db_slot0.sqrt_price_x96,
        rpc_sqrt_price,
        "sqrtPriceX96 mismatch!"
    );

    println!("  tick:");
    println!("    DB:  {}", db_slot0.tick);
    println!("    RPC: {}", rpc_result.tick);
    let rpc_tick_i32: i32 = rpc_result.tick.as_i32();
    assert_eq!(db_slot0.tick, rpc_tick_i32, "Tick mismatch!");

    println!("  observationIndex:");
    println!("    DB:  {}", db_slot0.observation_index);
    println!("    RPC: {}", rpc_result.observationIndex);
    assert_eq!(
        db_slot0.observation_index, rpc_result.observationIndex,
        "observationIndex mismatch!"
    );

    println!("  observationCardinality:");
    println!("    DB:  {}", db_slot0.observation_cardinality);
    println!("    RPC: {}", rpc_result.observationCardinality);
    assert_eq!(
        db_slot0.observation_cardinality, rpc_result.observationCardinality,
        "observationCardinality mismatch!"
    );

    println!("  observationCardinalityNext:");
    println!("    DB:  {}", db_slot0.observation_cardinality_next);
    println!("    RPC: {}", rpc_result.observationCardinalityNext);
    assert_eq!(
        db_slot0.observation_cardinality_next, rpc_result.observationCardinalityNext,
        "observationCardinalityNext mismatch!"
    );

    println!("  feeProtocol:");
    println!("    DB:  {}", db_slot0.fee_protocol);
    println!("    RPC: {}", rpc_result.feeProtocol);
    assert_eq!(
        db_slot0.fee_protocol, rpc_result.feeProtocol,
        "feeProtocol mismatch!"
    );

    println!("  unlocked:");
    println!("    DB:  {}", db_slot0.unlocked);
    println!("    RPC: {}", rpc_result.unlocked);
    assert_eq!(
        db_slot0.unlocked, rpc_result.unlocked,
        "unlocked mismatch!"
    );

    // Report bitmap and tick counts
    println!("  Bitmaps found: {}", db_data.bitmaps.len());
    println!("  Ticks found: {}", db_data.ticks.len());

    println!("  ✅ V4 validation passed");

    Ok(())
}
