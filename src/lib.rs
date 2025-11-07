pub mod contracts;
pub mod decoding;
pub mod events;
pub mod historical;
pub mod readers;
pub mod storage;
pub mod tick_math;
pub mod types;

#[cfg(feature = "python")]
pub mod python;

use alloy_primitives::{Address, B256};
use eyre::{eyre, Result};
use reth_db::{database::Database, open_db_read_only};
use std::path::Path;

use types::BlockNumber;

pub use events::{EventLog, EventScanResult};
pub use types::{Bitmap, HistoricalPoolOutput, PoolInput, PoolOutput, Protocol, Reserves, Slot0, Tick};

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

/// Collect historical pool data at a specific block number
///
/// # Arguments
/// * `db_path` - Path to the reth database directory
/// * `pools` - List of pool configurations to collect data from
/// * `v4_pool_ids` - Optional list of pool IDs for V4 pools
/// * `block_number` - Block number to query state at
///
/// # Returns
/// Vector of `HistoricalPoolOutput` containing data at the specified block
pub fn collect_pool_data_at_block(
    db_path: impl AsRef<Path>,
    pools: &[PoolInput],
    v4_pool_ids: Option<&[B256]>,
    block_number: BlockNumber,
) -> Result<Vec<HistoricalPoolOutput>> {
    let db = open_db_read_only(db_path.as_ref(), Default::default())?;
    let tx = db.tx()?;

    let mut results = Vec::new();
    let mut v4_pool_id_idx = 0;

    for pool in pools {
        let pool_data = match pool.protocol {
            Protocol::UniswapV2 => {
                historical::read_v2_pool_at_block(&tx, pool, block_number)?
            }
            Protocol::UniswapV3 => {
                historical::read_v3_pool_at_block(&tx, pool, block_number)?
            }
            Protocol::UniswapV4 => {
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

                historical::read_v4_pool_at_block(&tx, pool, pool_id, block_number)?
            }
        };

        results.push(HistoricalPoolOutput {
            pool_data,
            block_number,
        });
    }

    Ok(results)
}

/// Scan for events from a pool address
///
/// # Arguments
/// * `db_path` - Path to the reth database directory
/// * `pool_address` - Address of the pool to scan for events
/// * `from_block` - Starting block number (inclusive)
/// * `to_block` - Ending block number (inclusive)
/// * `topics` - Optional topic filters (e.g., event signatures)
///
/// # Returns
/// `EventScanResult` containing all matching logs and statistics
pub fn scan_pool_events(
    db_path: impl AsRef<Path>,
    pool_address: Address,
    from_block: BlockNumber,
    to_block: BlockNumber,
    topics: Option<Vec<B256>>,
) -> Result<EventScanResult> {
    let db = open_db_read_only(db_path.as_ref(), Default::default())?;
    let tx = db.tx()?;

    events::scan_events(&tx, pool_address, from_block, to_block, topics)
}

/// Get V3 Swap events for a pool
pub fn get_v3_swap_events(
    db_path: impl AsRef<Path>,
    pool_address: Address,
    from_block: BlockNumber,
    to_block: BlockNumber,
) -> Result<EventScanResult> {
    let db = open_db_read_only(db_path.as_ref(), Default::default())?;
    let tx = db.tx()?;

    events::get_v3_swap_events(&tx, pool_address, from_block, to_block)
}

/// Get V3 Mint events for a pool
pub fn get_v3_mint_events(
    db_path: impl AsRef<Path>,
    pool_address: Address,
    from_block: BlockNumber,
    to_block: BlockNumber,
) -> Result<EventScanResult> {
    let db = open_db_read_only(db_path.as_ref(), Default::default())?;
    let tx = db.tx()?;

    events::get_v3_mint_events(&tx, pool_address, from_block, to_block)
}

/// Get V3 Burn events for a pool
pub fn get_v3_burn_events(
    db_path: impl AsRef<Path>,
    pool_address: Address,
    from_block: BlockNumber,
    to_block: BlockNumber,
) -> Result<EventScanResult> {
    let db = open_db_read_only(db_path.as_ref(), Default::default())?;
    let tx = db.tx()?;

    events::get_v3_burn_events(&tx, pool_address, from_block, to_block)
}

/// Scan for events from multiple pool addresses - OPTIMIZED
///
/// This is significantly more efficient than calling `scan_pool_events` multiple times
/// because it scans each block only once and checks bloom filters for all addresses
/// at once.
///
/// # Performance
/// If you have N addresses and M blocks:
/// - Multiple calls to `scan_pool_events`: N * M block reads
/// - This function: M block reads (N times faster!)
///
/// # Arguments
/// * `db_path` - Path to the reth database directory
/// * `pool_addresses` - List of pool addresses to scan for events
/// * `from_block` - Starting block number (inclusive)
/// * `to_block` - Ending block number (inclusive)
/// * `topics` - Optional topic filters (e.g., event signatures)
///
/// # Returns
/// Vector of `EventScanResult`, one for each address in the same order
pub fn scan_pool_events_multi(
    db_path: impl AsRef<Path>,
    pool_addresses: &[Address],
    from_block: BlockNumber,
    to_block: BlockNumber,
    topics: Option<Vec<B256>>,
) -> Result<Vec<EventScanResult>> {
    let db = open_db_read_only(db_path.as_ref(), Default::default())?;
    let tx = db.tx()?;

    events::scan_events_multi_address(&tx, pool_addresses, from_block, to_block, topics)
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
