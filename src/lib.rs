pub mod contracts;
pub mod decoding;
pub mod readers;
pub mod storage;
pub mod tick_math;
pub mod types;

#[cfg(feature = "python")]
pub mod python;

use alloy_primitives::B256;
use eyre::{eyre, Result};
use reth_db::{database::Database, open_db_read_only};
use std::path::Path;

pub use types::{Bitmap, PoolInput, PoolOutput, Protocol, Reserves, Slot0, Tick};

/// Main function to collect pool data from reth database
///
/// # Arguments
/// * `db_path` - Path to the reth database directory
/// * `pools` - List of pool configurations to collect data from
/// * `v4_pool_ids` - Optional list of pool IDs for V4 pools (must match order of V4 pools in `pools`)
///
/// # Returns
/// Vector of `PoolOutput` containing collected data for each pool
///
/// # Example
/// ```no_run
/// use scrape_rethdb_data::{collect_pool_data, PoolInput, Protocol};
/// use alloy_primitives::Address;
///
/// let pools = vec![
///     PoolInput {
///         address: "0x1234...".parse().unwrap(),
///         protocol: Protocol::UniswapV3,
///         tick_spacing: Some(60),
///     },
/// ];
///
/// let results = collect_pool_data("/path/to/reth/db", &pools, None).unwrap();
/// ```
pub fn collect_pool_data(
    db_path: impl AsRef<Path>,
    pools: &[PoolInput],
    v4_pool_ids: Option<&[B256]>,
) -> Result<Vec<PoolOutput>> {
    // Open database read-only
    let db = open_db_read_only(db_path.as_ref(), Default::default())?;

    let tx = db.tx()?;

    let mut results = Vec::new();
    let mut v4_pool_id_idx = 0;

    for pool in pools {
        match pool.protocol {
            Protocol::UniswapV2 => {
                let output = readers::read_v2_pool(&tx, pool)?;
                results.push(output);
            }
            Protocol::UniswapV3 => {
                let output = readers::read_v3_pool(&tx, pool)?;
                results.push(output);
            }
            Protocol::UniswapV4 => {
                // V4 requires a pool ID
                let pool_ids = v4_pool_ids.ok_or_else(|| {
                    eyre!("V4 pools require pool_ids parameter")
                })?;

                if v4_pool_id_idx >= pool_ids.len() {
                    return Err(eyre!(
                        "Not enough pool IDs provided for V4 pools (need at least {})",
                        v4_pool_id_idx + 1
                    ));
                }

                let pool_id = pool_ids[v4_pool_id_idx];
                v4_pool_id_idx += 1;

                let output = readers::read_v4_pool(&tx, pool, pool_id)?;
                results.push(output);
            }
        }
    }

    Ok(results)
}

/// Collect data from a single pool
pub fn collect_single_pool(
    db_path: impl AsRef<Path>,
    pool: &PoolInput,
    v4_pool_id: Option<B256>,
) -> Result<PoolOutput> {
    let v4_pool_ids = v4_pool_id.map(|id| vec![id]);
    let results = collect_pool_data(
        db_path,
        &[pool.clone()],
        v4_pool_ids.as_deref(),
    )?;

    results.into_iter().next().ok_or_else(|| eyre!("No results returned"))
}

/// Helper to collect data from multiple V3 pools efficiently
pub fn collect_v3_pools(
    db_path: impl AsRef<Path>,
    pools: &[PoolInput],
) -> Result<Vec<PoolOutput>> {
    // Verify all pools are V3
    for pool in pools {
        if pool.protocol != Protocol::UniswapV3 {
            return Err(eyre!("All pools must be UniswapV3"));
        }
    }

    collect_pool_data(db_path, pools, None)
}

/// Helper to collect data from multiple V2 pools efficiently
pub fn collect_v2_pools(
    db_path: impl AsRef<Path>,
    pools: &[PoolInput],
) -> Result<Vec<PoolOutput>> {
    // Verify all pools are V2
    for pool in pools {
        if pool.protocol != Protocol::UniswapV2 {
            return Err(eyre!("All pools must be UniswapV2"));
        }
    }

    collect_pool_data(db_path, pools, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_input_creation() {
        let addr = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();

        let v2_pool = PoolInput::new_v2(addr);
        assert_eq!(v2_pool.protocol, Protocol::UniswapV2);
        assert_eq!(v2_pool.tick_spacing, None);

        let v3_pool = PoolInput::new_v3(addr, 60);
        assert_eq!(v3_pool.protocol, Protocol::UniswapV3);
        assert_eq!(v3_pool.tick_spacing, Some(60));
    }
}
