//! Python bindings for scrape_rethdb_data
//!
//! Build with: maturin develop --features python

use pyo3::prelude::*;
use pyo3::types::PyList;
use std::str::FromStr;

use crate::{collect_pool_data, PoolInput, Protocol};
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

/// Python module
#[pymodule]
fn scrape_rethdb_data(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(collect_pools, m)?)?;
    m.add_class::<PyPoolInput>()?;
    Ok(())
}
