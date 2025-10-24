use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use alloy::sol;
use std::str::FromStr;

// Define the Uniswap V2 Pair interface using sol! macro
sol! {
    #[sol(rpc)]
    contract IUniswapV2Pair {
        function getReserves() external view returns (
            uint112 reserve0,
            uint112 reserve1,
            uint32 blockTimestampLast
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Your local RPC endpoint
    let rpc_url = "http://100.104.193.35:8545";

    // USDC-WETH pair on Ethereum mainnet (most liquid V2 pool)
    let pool_address = Address::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc")?;

    println!("Testing Alloy sol! macro with Uniswap V2 getReserves()");
    println!("========================================================\n");
    println!("RPC: {}", rpc_url);
    println!("Pool: {} (USDC-WETH V2)", pool_address);
    println!();

    // Create provider
    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);

    // Create contract instance using the auto-generated bindings
    let pool = IUniswapV2Pair::new(pool_address, provider);

    // Call getReserves() - Alloy handles all encoding/decoding automatically!
    println!("Calling getReserves()...");
    let result = pool.getReserves().call().await?;

    // Extract values from the auto-generated struct
    // Alloy uses Uint types, need to convert to u128
    let reserve0 = result.reserve0.to::<u128>();
    let reserve1 = result.reserve1.to::<u128>();
    let timestamp = result.blockTimestampLast;

    println!("\n✅ Successfully decoded reserves using Alloy sol! macro:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Reserve0 (USDC): {}", reserve0);
    println!("  Reserve1 (WETH): {}", reserve1);
    println!("  Block Timestamp: {}", timestamp);
    println!();
    println!("  Formatted:");
    println!("    USDC: {:.2}", reserve0 as f64 / 1e6);
    println!("    WETH: {:.6}", reserve1 as f64 / 1e18);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
