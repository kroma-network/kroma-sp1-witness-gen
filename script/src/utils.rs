use std::{path::PathBuf, str::FromStr};

use alloy_primitives::{hex::FromHex, B256};
use alloy_provider::{network::primitives::BlockTransactionsKind, Provider};
use op_succinct_client_utils::block_on;
use op_succinct_host_utils::{
    fetcher::{OPSuccinctDataFetcher, RPCMode, RunContext},
    stats::ExecutionStats,
};
use serde_json::Value;
use sp1_sdk::ExecutionReport;

const REPORT_DIR: &str = "execution-reports";

#[allow(clippy::too_many_arguments)]
pub async fn report_execution(
    data_fetcher: &OPSuccinctDataFetcher,
    start: u64,
    end: u64,
    report: &ExecutionReport,
    witness_generation_time_sec: std::time::Duration,
    execution_duration: std::time::Duration,
    l2_chain_id: u64,
    l2_number: u64,
) {
    let block_data = data_fetcher
        .get_l2_block_data_range(start, end)
        .await
        .expect("Failed to fetch block data range.");

    let mut stats = ExecutionStats::new(
        &block_data,
        report,
        witness_generation_time_sec.as_secs(),
        execution_duration.as_secs(),
    );
    println!("{:#?}", stats);

    let mut report_path = PathBuf::from_str(REPORT_DIR).unwrap();
    report_path.push(l2_chain_id.to_string());
    if !std::path::Path::new(&report_path).exists() {
        std::fs::create_dir_all(&report_path).unwrap();
    }
    report_path.push(format!("{}.csv", l2_number));

    // Write to CSV.
    let mut csv_writer = csv::Writer::from_path(report_path).unwrap();
    csv_writer.serialize(&stats).unwrap();
    csv_writer.flush().unwrap();
}

#[allow(dead_code)]
pub fn get_l1_block_hash(block_number: u64) -> B256 {
    let data_fetcher = OPSuccinctDataFetcher::new(RunContext::Dev);
    let l1_provider = data_fetcher.l1_provider;
    let l1_head_block = block_on(async move {
        l1_provider
            .get_block_by_number(block_number.into(), BlockTransactionsKind::Hashes)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Block not found for block number {}", block_number))
    })
    .unwrap();
    l1_head_block.header.hash
}

fn get_output_at_impl(data_fetcher: &OPSuccinctDataFetcher, block_number: u64) -> Value {
    let block_number_hex = format!("0x{:x}", block_number);
    let result: Value = block_on(async {
        data_fetcher
            .fetch_rpc_data_with_mode(
                RPCMode::L2Node,
                "optimism_outputAtBlock",
                vec![block_number_hex.into()],
            )
            .await
    })
    .unwrap();
    result
}

pub fn get_output_at(data_fetcher: &OPSuccinctDataFetcher, block_number: u64) -> B256 {
    let result = get_output_at_impl(data_fetcher, block_number);
    let output_root = result["outputRoot"].as_str().unwrap().to_string();
    B256::from_hex(output_root).unwrap()
}

pub fn get_l1_origin_of(data_fetcher: &OPSuccinctDataFetcher, block_number: u64) -> (B256, u64) {
    let result = get_output_at_impl(data_fetcher, block_number);
    let l1_origin = &result["blockRef"]["l1origin"];

    let origin_hash = B256::from_hex(l1_origin["hash"].as_str().unwrap()).unwrap();
    let origin_number = l1_origin["number"].as_u64().unwrap();

    (origin_hash, origin_number)
}
