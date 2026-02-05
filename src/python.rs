//! Python bindings for scrape_rethdb_data
//!
//! Build with: maturin develop --features python

use pyo3::prelude::*;
use pyo3::types::PyList;
use std::str::FromStr;

use crate::{
    collect_pool_data, collect_pool_data_at_block, scan_pool_events, scan_pool_events_multi,
    get_v3_swap_events, get_v3_mint_events, get_v3_burn_events, PoolInput, Protocol,
};
use alloy_primitives::{Address, B256};

/// Python wrapper for PoolInput
#[pyclass]
#[derive(Clone)]
struct PyPoolInput {
    #[pyo3(get, set)]
    address: String,
    #[pyo3(get, set)]
    protocol: String,
    #[pyo3(get, set)]
    tick_spacing: Option<i32>,
}

#[pymethods]
impl PyPoolInput {
    #[new]
    #[pyo3(signature = (address, protocol, tick_spacing=None))]
    fn new(address: String, protocol: String, tick_spacing: Option<i32>) -> Self {
        Self {
            address,
            protocol,
            tick_spacing,
        }
    }
}

/// Collect pool data from reth database
///
/// Parameters:
/// - db_path (str): Path to reth database directory
/// - pools (List[PyPoolInput]): List of pool configurations
/// - v4_pool_ids (Optional[List[str]]): List of pool IDs for V4 pools (hex strings)
///
/// Returns:
/// - str: JSON string containing collected pool data
///
/// Example:
/// ```python
/// from scrape_rethdb_data import collect_pools
///
/// pools = [
///     {"address": "0x...", "protocol": "v3", "tick_spacing": 60},
///     {"address": "0x...", "protocol": "v2", "tick_spacing": None},
/// ]
///
/// result_json = collect_pools("/path/to/reth/db", pools)
/// import json
/// data = json.loads(result_json)
/// ```
#[pyfunction]
#[pyo3(signature = (db_path, pools, v4_pool_ids=None))]
fn collect_pools(
    db_path: String,
    pools: &Bound<'_, PyList>,
    v4_pool_ids: Option<Vec<String>>,
) -> PyResult<String> {
    // Convert Python pools to Rust PoolInput
    let rust_pools: Result<Vec<PoolInput>, PyErr> = pools
        .iter()
        .map(|item| {
            let dict = item.downcast::<pyo3::types::PyDict>()?;

            let address_str: String = dict
                .get_item("address")?
                .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Missing 'address'"))?
                .extract()?;

            let protocol_str: String = dict
                .get_item("protocol")?
                .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Missing 'protocol'"))?
                .extract()?;

            let tick_spacing: Option<i32> = match dict.get_item("tick_spacing")? {
                Some(v) => {
                    if v.is_none() {
                        None
                    } else {
                        Some(v.extract()?)
                    }
                }
                None => None,
            };

            let slot0_only: bool = match dict.get_item("slot0_only")? {
                Some(v) => v.extract().unwrap_or(false),
                None => false,
            };

            let address = Address::from_str(&address_str)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address: {}", e)))?;

            let protocol = match protocol_str.to_lowercase().as_str() {
                "v2" | "uniswapv2" => Protocol::UniswapV2,
                "v3" | "uniswapv3" => Protocol::UniswapV3,
                "v4" | "uniswapv4" => Protocol::UniswapV4,
                _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Invalid protocol: {}", protocol_str)
                )),
            };

            Ok(PoolInput {
                address,
                protocol,
                tick_spacing,
                slot0_only,
            })
        })
        .collect();

    let rust_pools = rust_pools?;

    // Convert V4 pool IDs if provided
    let rust_v4_pool_ids: Option<Result<Vec<B256>, PyErr>> = v4_pool_ids.map(|ids| {
        ids.iter()
            .map(|id_str| {
                B256::from_str(id_str).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid pool ID: {}",
                        e
                    ))
                })
            })
            .collect()
    });

    let rust_v4_pool_ids = rust_v4_pool_ids.transpose()?;

    // Call Rust function
    let results = collect_pool_data(&db_path, &rust_pools, rust_v4_pool_ids.as_deref())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Collection failed: {}", e)))?;

    // Serialize to JSON
    let json = serde_json::to_string(&results)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization failed: {}", e)))?;

    Ok(json)
}

/// Collect pool data at a specific block number
///
/// Parameters:
/// - db_path (str): Path to reth database directory
/// - pools (List[dict]): List of pool configurations
/// - v4_pool_ids (Optional[List[str]]): List of pool IDs for V4 pools (hex strings)
/// - block_number (int): Block number to query state at
///
/// Returns:
/// - str: JSON string containing historical pool data
///
/// Example:
/// ```python
/// from scrape_rethdb_data import collect_pools_at_block
///
/// pools = [{"address": "0x...", "protocol": "v3", "tick_spacing": 60}]
/// result_json = collect_pools_at_block("/path/to/reth/db", pools, 12345678, None)
/// ```
#[pyfunction]
#[pyo3(signature = (db_path, pools, block_number, v4_pool_ids=None))]
fn collect_pools_at_block(
    db_path: String,
    pools: &Bound<'_, PyList>,
    block_number: u64,
    v4_pool_ids: Option<Vec<String>>,
) -> PyResult<String> {
    // Convert Python pools to Rust PoolInput (same logic as collect_pools)
    let rust_pools: Result<Vec<PoolInput>, PyErr> = pools
        .iter()
        .map(|item| {
            let dict = item.downcast::<pyo3::types::PyDict>()?;

            let address_str: String = dict
                .get_item("address")?
                .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Missing 'address'"))?
                .extract()?;

            let protocol_str: String = dict
                .get_item("protocol")?
                .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Missing 'protocol'"))?
                .extract()?;

            let tick_spacing: Option<i32> = match dict.get_item("tick_spacing")? {
                Some(v) => {
                    if v.is_none() {
                        None
                    } else {
                        Some(v.extract()?)
                    }
                }
                None => None,
            };

            let address = Address::from_str(&address_str)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address: {}", e)))?;

            let protocol = match protocol_str.to_lowercase().as_str() {
                "v2" | "uniswapv2" => Protocol::UniswapV2,
                "v3" | "uniswapv3" => Protocol::UniswapV3,
                "v4" | "uniswapv4" => Protocol::UniswapV4,
                _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Invalid protocol: {}", protocol_str)
                )),
            };

            Ok(PoolInput {
                address,
                protocol,
                tick_spacing,
                slot0_only: false,
            })
        })
        .collect();

    let rust_pools = rust_pools?;

    // Convert V4 pool IDs if provided
    let rust_v4_pool_ids: Option<Result<Vec<B256>, PyErr>> = v4_pool_ids.map(|ids| {
        ids.iter()
            .map(|id_str| {
                B256::from_str(id_str).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid pool ID: {}",
                        e
                    ))
                })
            })
            .collect()
    });

    let rust_v4_pool_ids = rust_v4_pool_ids.transpose()?;

    // Call Rust function
    let results = collect_pool_data_at_block(
        &db_path,
        &rust_pools,
        rust_v4_pool_ids.as_deref(),
        block_number,
    )
    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Collection failed: {}", e)))?;

    // Serialize to JSON
    let json = serde_json::to_string(&results)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization failed: {}", e)))?;

    Ok(json)
}

/// Scan for events from a pool address
///
/// Parameters:
/// - db_path (str): Path to reth database directory
/// - pool_address (str): Address of the pool to scan (hex string)
/// - from_block (int): Starting block number (inclusive)
/// - to_block (int): Ending block number (inclusive)
/// - topics (Optional[List[str]]): Optional topic filters (hex strings)
///
/// Returns:
/// - str: JSON string containing event scan results
///
/// Example:
/// ```python
/// from scrape_rethdb_data import scan_events
///
/// result_json = scan_events(
///     "/path/to/reth/db",
///     "0x...",
///     12000000,
///     12100000,
///     None  # No topic filter
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (db_path, pool_address, from_block, to_block, topics=None))]
fn scan_events(
    db_path: String,
    pool_address: String,
    from_block: u64,
    to_block: u64,
    topics: Option<Vec<String>>,
) -> PyResult<String> {
    let address = Address::from_str(&pool_address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address: {}", e)))?;

    let rust_topics: Option<Result<Vec<B256>, PyErr>> = topics.map(|ts| {
        ts.iter()
            .map(|t| {
                B256::from_str(t).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid topic: {}", e))
                })
            })
            .collect()
    });

    let rust_topics = rust_topics.transpose()?;

    let result = scan_pool_events(&db_path, address, from_block, to_block, rust_topics)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Scan failed: {}", e)))?;

    let json = serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization failed: {}", e)))?;

    Ok(json)
}

/// Scan for events from multiple pool addresses - OPTIMIZED
///
/// This is much more efficient than calling scan_events multiple times
/// because it scans each block only once.
///
/// Parameters:
/// - db_path (str): Path to reth database directory
/// - pool_addresses (List[str]): List of pool addresses to scan (hex strings)
/// - from_block (int): Starting block number (inclusive)
/// - to_block (int): Ending block number (inclusive)
/// - topics (Optional[List[str]]): Optional topic filters (hex strings)
///
/// Returns:
/// - str: JSON string containing list of event scan results (one per address)
///
/// Example:
/// ```python
/// from scrape_rethdb_data import scan_events_multi
///
/// result_json = scan_events_multi(
///     "/path/to/reth/db",
///     ["0x...", "0x..."],
///     12000000,
///     12100000,
///     None
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (db_path, pool_addresses, from_block, to_block, topics=None))]
fn scan_events_multi(
    db_path: String,
    pool_addresses: Vec<String>,
    from_block: u64,
    to_block: u64,
    topics: Option<Vec<String>>,
) -> PyResult<String> {
    let addresses: Result<Vec<Address>, PyErr> = pool_addresses
        .iter()
        .map(|addr_str| {
            Address::from_str(addr_str).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address: {}", e))
            })
        })
        .collect();

    let addresses = addresses?;

    let rust_topics: Option<Result<Vec<B256>, PyErr>> = topics.map(|ts| {
        ts.iter()
            .map(|t| {
                B256::from_str(t).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid topic: {}", e))
                })
            })
            .collect()
    });

    let rust_topics = rust_topics.transpose()?;

    let results = scan_pool_events_multi(&db_path, &addresses, from_block, to_block, rust_topics)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Scan failed: {}", e)))?;

    let json = serde_json::to_string(&results)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization failed: {}", e)))?;

    Ok(json)
}

/// Get V3 Swap events for a pool
///
/// Parameters:
/// - db_path (str): Path to reth database directory
/// - pool_address (str): Address of the pool (hex string)
/// - from_block (int): Starting block number (inclusive)
/// - to_block (int): Ending block number (inclusive)
///
/// Returns:
/// - str: JSON string containing swap event scan results
#[pyfunction]
fn get_swap_events(
    db_path: String,
    pool_address: String,
    from_block: u64,
    to_block: u64,
) -> PyResult<String> {
    let address = Address::from_str(&pool_address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address: {}", e)))?;

    let result = get_v3_swap_events(&db_path, address, from_block, to_block)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Scan failed: {}", e)))?;

    let json = serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization failed: {}", e)))?;

    Ok(json)
}

/// Get V3 Mint events for a pool
///
/// Parameters:
/// - db_path (str): Path to reth database directory
/// - pool_address (str): Address of the pool (hex string)
/// - from_block (int): Starting block number (inclusive)
/// - to_block (int): Ending block number (inclusive)
///
/// Returns:
/// - str: JSON string containing mint event scan results
#[pyfunction]
fn get_mint_events(
    db_path: String,
    pool_address: String,
    from_block: u64,
    to_block: u64,
) -> PyResult<String> {
    let address = Address::from_str(&pool_address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address: {}", e)))?;

    let result = get_v3_mint_events(&db_path, address, from_block, to_block)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Scan failed: {}", e)))?;

    let json = serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization failed: {}", e)))?;

    Ok(json)
}

/// Get V3 Burn events for a pool
///
/// Parameters:
/// - db_path (str): Path to reth database directory
/// - pool_address (str): Address of the pool (hex string)
/// - from_block (int): Starting block number (inclusive)
/// - to_block (int): Ending block number (inclusive)
///
/// Returns:
/// - str: JSON string containing burn event scan results
#[pyfunction]
fn get_burn_events(
    db_path: String,
    pool_address: String,
    from_block: u64,
    to_block: u64,
) -> PyResult<String> {
    let address = Address::from_str(&pool_address)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address: {}", e)))?;

    let result = get_v3_burn_events(&db_path, address, from_block, to_block)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Scan failed: {}", e)))?;

    let json = serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization failed: {}", e)))?;

    Ok(json)
}

/// Python module
#[pymodule]
fn scrape_rethdb_data(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(collect_pools, m)?)?;
    m.add_function(wrap_pyfunction!(collect_pools_at_block, m)?)?;
    m.add_function(wrap_pyfunction!(scan_events, m)?)?;
    m.add_function(wrap_pyfunction!(scan_events_multi, m)?)?;
    m.add_function(wrap_pyfunction!(get_swap_events, m)?)?;
    m.add_function(wrap_pyfunction!(get_mint_events, m)?)?;
    m.add_function(wrap_pyfunction!(get_burn_events, m)?)?;
    m.add_class::<PyPoolInput>()?;
    Ok(())
}
