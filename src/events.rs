/// Event log queries from Reth database
///
/// This module provides efficient event log scanning using:
/// - Direct access to Receipts table
/// - Bloom filter optimization to skip irrelevant blocks
/// - Parallel block processing capabilities

use alloy_primitives::{Address, BloomInput, Log, B256};

#[cfg(test)]
use alloy_primitives::Bloom;
use eyre::Result;
use reth_db::{cursor::DbCursorRO, tables, transaction::DbTx};
use serde::{Deserialize, Serialize};

// BlockNumber is just u64 in Reth
type BlockNumber = u64;

/// Event log with associated block and transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLog {
    /// The log data
    pub log: Log,
    /// Block number where this log was emitted
    pub block_number: BlockNumber,
    /// Transaction index within the block
    pub transaction_index: u64,
    /// Transaction hash (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_hash: Option<B256>,
}

/// Result of scanning for events in a block range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventScanResult {
    /// Address that was queried
    pub address: Address,
    /// Start block (inclusive)
    pub from_block: BlockNumber,
    /// End block (inclusive)
    pub to_block: BlockNumber,
    /// All logs found for this address
    pub logs: Vec<EventLog>,
    /// Number of blocks scanned
    pub blocks_scanned: u64,
    /// Number of blocks skipped by bloom filter
    pub blocks_skipped_by_bloom: u64,
}

/// Scan for event logs from a specific address within a block range
///
/// This function:
/// 1. Iterates through blocks in the range
/// 2. Uses bloom filters to skip blocks without relevant logs
/// 3. Reads receipts only for potentially relevant blocks
/// 4. Filters logs by address and topic (if specified)
pub fn scan_events<TX: DbTx>(
    tx: &TX,
    address: Address,
    from_block: BlockNumber,
    to_block: BlockNumber,
    topics: Option<Vec<B256>>, // Optional topic filters (topic0, topic1, etc.)
) -> Result<EventScanResult> {
    let mut logs = Vec::new();
    let mut blocks_scanned = 0u64;
    let mut blocks_skipped_by_bloom = 0u64;

    // Cursors for reading data
    let mut header_cursor = tx.cursor_read::<tables::Headers>()?;
    let mut body_cursor = tx.cursor_read::<tables::BlockBodyIndices>()?;
    let mut receipt_cursor = tx.cursor_read::<tables::Receipts>()?;

    // Iterate through each block in the range
    for block_num in from_block..=to_block {
        blocks_scanned += 1;

        // Step 1: Check bloom filter in block header
        if let Some((_, header)) = header_cursor.seek_exact(block_num)? {
            // Check if the bloom filter contains our address
            if !header.logs_bloom.contains_input(BloomInput::Raw(address.as_slice())) {
                // Bloom filter says this block definitely doesn't have logs from this address
                blocks_skipped_by_bloom += 1;
                continue;
            }

            // If topics are specified, also check bloom for topics
            if let Some(ref topic_list) = topics {
                let mut has_all_topics = true;
                for topic in topic_list {
                    if !header.logs_bloom.contains_input(BloomInput::Raw(topic.as_slice())) {
                        has_all_topics = false;
                        break;
                    }
                }
                if !has_all_topics {
                    blocks_skipped_by_bloom += 1;
                    continue;
                }
            }
        } else {
            // Block header not found, skip
            continue;
        }

        // Step 2: Get transaction range for this block
        if let Some((_, body_indices)) = body_cursor.seek_exact(block_num)? {
            // Step 3: Read receipts for all transactions in this block
            for tx_index in 0..body_indices.tx_count {
                let tx_num = body_indices.first_tx_num + tx_index;

                if let Some((_, receipt)) = receipt_cursor.seek_exact(tx_num)? {

                    // Step 4: Filter logs by address and topics
                    for log in receipt.logs {
                        // Check if log is from the target address
                        if log.address != address {
                            continue;
                        }

                        // Check topics if specified
                        if let Some(ref topic_list) = topics {
                            let mut matches_topics = true;
                            for (i, required_topic) in topic_list.iter().enumerate() {
                                if i >= log.data.topics().len() {
                                    matches_topics = false;
                                    break;
                                }
                                if &log.data.topics()[i] != required_topic {
                                    matches_topics = false;
                                    break;
                                }
                            }
                            if !matches_topics {
                                continue;
                            }
                        }

                        // This log matches our filters
                        logs.push(EventLog {
                            log: log.clone(),
                            block_number: block_num,
                            transaction_index: tx_index,
                            transaction_hash: None, // We'd need TransactionBlocks table for this
                        });
                    }
                }
            }
        }
    }

    Ok(EventScanResult {
        address,
        from_block,
        to_block,
        logs,
        blocks_scanned,
        blocks_skipped_by_bloom,
    })
}

/// Scan for events with multiple addresses (e.g., all pools) - OPTIMIZED
///
/// This function scans each block only ONCE and checks bloom filters for all addresses
/// at once, making it much more efficient than calling scan_events multiple times.
///
/// Performance improvement: If you have N addresses, this scans each block once instead
/// of N times, reducing database reads by ~N times.
pub fn scan_events_multi_address<TX: DbTx>(
    tx: &TX,
    addresses: &[Address],
    from_block: BlockNumber,
    to_block: BlockNumber,
    topics: Option<Vec<B256>>,
) -> Result<Vec<EventScanResult>> {
    if addresses.is_empty() {
        return Ok(Vec::new());
    }

    // Initialize result tracking for each address
    let mut results: Vec<EventScanResult> = addresses
        .iter()
        .map(|addr| EventScanResult {
            address: *addr,
            from_block,
            to_block,
            logs: Vec::new(),
            blocks_scanned: 0,
            blocks_skipped_by_bloom: 0,
        })
        .collect();

    // Cursors for reading data
    let mut header_cursor = tx.cursor_read::<tables::Headers>()?;
    let mut body_cursor = tx.cursor_read::<tables::BlockBodyIndices>()?;
    let mut receipt_cursor = tx.cursor_read::<tables::Receipts>()?;

    // Iterate through each block in the range ONCE
    for block_num in from_block..=to_block {
        // Step 1: Check bloom filter for ANY of the addresses
        if let Some((_, header)) = header_cursor.seek_exact(block_num)? {
            // Check if bloom filter contains ANY of our addresses
            let mut has_any_address = false;
            for addr in addresses {
                if header.logs_bloom.contains_input(BloomInput::Raw(addr.as_slice())) {
                    has_any_address = true;
                    break; // Early exit - at least one address might be present
                }
            }

            if !has_any_address {
                // None of the addresses are in this block - skip it for all addresses
                for result in results.iter_mut() {
                    result.blocks_scanned += 1;
                    result.blocks_skipped_by_bloom += 1;
                }
                continue;
            }

            // If topics are specified, also check bloom for topics
            if let Some(ref topic_list) = topics {
                let mut has_all_topics = true;
                for topic in topic_list {
                    if !header.logs_bloom.contains_input(BloomInput::Raw(topic.as_slice())) {
                        has_all_topics = false;
                        break;
                    }
                }
                if !has_all_topics {
                    for result in results.iter_mut() {
                        result.blocks_scanned += 1;
                        result.blocks_skipped_by_bloom += 1;
                    }
                    continue;
                }
            }
        } else {
            // Block header not found, skip
            continue;
        }

        // Step 2: Block might contain logs from at least one address - scan it
        for result in results.iter_mut() {
            result.blocks_scanned += 1;
        }

        if let Some((_, body_indices)) = body_cursor.seek_exact(block_num)? {
            // Step 3: Read receipts for all transactions in this block
            for tx_index in 0..body_indices.tx_count {
                let tx_num = body_indices.first_tx_num + tx_index;

                if let Some((_, receipt)) = receipt_cursor.seek_exact(tx_num)? {
                    // Step 4: Filter logs by addresses and topics
                    for log in receipt.logs {
                        // Check if log matches any of our target addresses
                        for (i, addr) in addresses.iter().enumerate() {
                            if log.address != *addr {
                                continue;
                            }

                            // Check topics if specified
                            if let Some(ref topic_list) = topics {
                                let mut matches_topics = true;
                                for (topic_idx, required_topic) in topic_list.iter().enumerate() {
                                    if topic_idx >= log.data.topics().len() {
                                        matches_topics = false;
                                        break;
                                    }
                                    if &log.data.topics()[topic_idx] != required_topic {
                                        matches_topics = false;
                                        break;
                                    }
                                }
                                if !matches_topics {
                                    continue;
                                }
                            }

                            // This log matches this address's filters
                            results[i].logs.push(EventLog {
                                log: log.clone(),
                                block_number: block_num,
                                transaction_index: tx_index,
                                transaction_hash: None,
                            });
                            break; // Move to next log (one address matched)
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Get all Uniswap V3 Swap events for a pool
///
/// Swap event signature: Swap(address,address,int256,int256,uint160,uint128,int24)
/// Topic0: keccak256("Swap(address,address,int256,int256,uint160,uint128,int24)")
pub fn get_v3_swap_events<TX: DbTx>(
    tx: &TX,
    pool_address: Address,
    from_block: BlockNumber,
    to_block: BlockNumber,
) -> Result<EventScanResult> {
    // Swap event topic0
    let swap_topic = B256::from_slice(
        &hex::decode("c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67")
            .expect("valid hex"),
    );

    scan_events(tx, pool_address, from_block, to_block, Some(vec![swap_topic]))
}

/// Get all Uniswap V3 Mint events for a pool
///
/// Mint event signature: Mint(address,address,int24,int24,uint128,uint256,uint256)
/// Topic0: keccak256("Mint(address,address,int24,int24,uint128,uint256,uint256)")
pub fn get_v3_mint_events<TX: DbTx>(
    tx: &TX,
    pool_address: Address,
    from_block: BlockNumber,
    to_block: BlockNumber,
) -> Result<EventScanResult> {
    // Mint event topic0
    let mint_topic = B256::from_slice(
        &hex::decode("7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde")
            .expect("valid hex"),
    );

    scan_events(tx, pool_address, from_block, to_block, Some(vec![mint_topic]))
}

/// Get all Uniswap V3 Burn events for a pool
///
/// Burn event signature: Burn(address,int24,int24,uint128,uint256,uint256)
/// Topic0: keccak256("Burn(address,int24,int24,uint128,uint256,uint256)")
pub fn get_v3_burn_events<TX: DbTx>(
    tx: &TX,
    pool_address: Address,
    from_block: BlockNumber,
    to_block: BlockNumber,
) -> Result<EventScanResult> {
    // Burn event topic0
    let burn_topic = B256::from_slice(
        &hex::decode("0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c")
            .expect("valid hex"),
    );

    scan_events(tx, pool_address, from_block, to_block, Some(vec![burn_topic]))
}

/// Estimate the number of blocks that can be scanned efficiently
///
/// Returns a suggested chunk size for batch processing based on:
/// - Average transactions per block
/// - Available memory
pub fn suggest_block_chunk_size<TX: DbTx>(
    tx: &TX,
    sample_from_block: BlockNumber,
    sample_size: u64,
) -> Result<u64> {
    let mut body_cursor = tx.cursor_read::<tables::BlockBodyIndices>()?;

    let mut total_txs = 0u64;
    let mut blocks_sampled = 0u64;

    for block_num in sample_from_block..(sample_from_block + sample_size) {
        if let Some((_, body_indices)) = body_cursor.seek_exact(block_num)? {
            total_txs += body_indices.tx_count;
            blocks_sampled += 1;
        }
    }

    if blocks_sampled == 0 {
        return Ok(10000); // Default chunk size
    }

    let avg_txs_per_block = total_txs / blocks_sampled;

    // Heuristic: aim for ~100k transactions per chunk
    let chunk_size = if avg_txs_per_block > 0 {
        100_000 / avg_txs_per_block
    } else {
        10_000
    };

    // Clamp between reasonable bounds
    Ok(chunk_size.max(1000).min(50_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter_matching() {
        // Create a bloom filter
        let mut bloom = Bloom::ZERO;

        // Add an address to it
        let address = Address::from([0x42; 20]);
        bloom.accrue(BloomInput::Raw(address.as_slice()));

        // Check that it contains the address
        assert!(bloom.contains_input(BloomInput::Raw(address.as_slice())));

        // Check that it doesn't contain a different address
        let other_address = Address::from([0x99; 20]);
        assert!(!bloom.contains_input(BloomInput::Raw(other_address.as_slice())));
    }

    #[test]
    #[ignore] // Requires real database
    fn test_event_scan() {
        // This test demonstrates usage pattern
        // In reality, you'd need a real database connection

        // Example:
        // let db = open_db_read_only(db_path)?;
        // let tx = db.tx()?;
        // let result = scan_events(
        //     &tx,
        //     pool_address,
        //     from_block,
        //     to_block,
        //     None, // No topic filter
        // )?;
        // println!("Found {} logs", result.logs.len());
        // println!("Skipped {} blocks via bloom filter", result.blocks_skipped_by_bloom);
    }
}
