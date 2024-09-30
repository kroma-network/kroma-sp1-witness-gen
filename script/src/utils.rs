use std::{env, path::PathBuf, str::FromStr};

use alloy_primitives::B256;
use alloy_provider::Provider;
use op_succinct_host_utils::{fetcher::OPSuccinctDataFetcher, stats::ExecutionStats};
use sp1_sdk::{block_on, ExecutionReport};

const REPORT_DIR: &str = "execution-reports";

pub fn report_execution(
    data_fetcher: &OPSuccinctDataFetcher,
    report: &ExecutionReport,
    execution_duration: std::time::Duration,
    l2_chain_id: u64,
    l2_number: u64,
) {
    let mut stats = ExecutionStats::default();
    block_on(async { stats.add_block_data(&data_fetcher, l2_number, l2_number).await });
    stats.add_report_data(&report, execution_duration);
    stats.add_aggregate_data();
    println!("Execution Stats: \n{:?}", stats);

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

pub fn get_l1_block_hash(block_number: u64) -> B256 {
    let data_fetcher = OPSuccinctDataFetcher {
        l1_rpc: env::var("L1_RPC").expect("L1_RPC is not set."),
        ..Default::default()
    };
    let l1_provider = data_fetcher.l1_provider;
    let l1_head_block = block_on(async move {
        l1_provider
            .get_block_by_number(block_number.into(), false)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Block not found for block number {}", block_number))
    })
    .unwrap();
    l1_head_block.header.hash
}
