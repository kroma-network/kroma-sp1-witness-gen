use alloy::{providers::Provider, rpc::types::BlockTransactionsKind};
use alloy_primitives::{hex::FromHex, B256};
use anyhow::Result;
use kona_host::HostCli;
use op_succinct_host_utils::{
    fetcher::{CacheMode, OPSuccinctDataFetcher, RPCMode},
    stats::ExecutionStats,
    ProgramType,
};
use serde_json::Value;
use sp1_sdk::{block_on, utils as sdk_utils, ExecutionReport};
use std::{num::ParseIntError, path::PathBuf, str::FromStr};
use utils::PreviewReport;

pub mod utils;

const REPORT_DIR: &str = "execution-reports";

pub fn init_env() {
    dotenv::dotenv().ok();
    sdk_utils::setup_logger();
}

pub async fn init_fetcher() -> Result<OPSuccinctDataFetcher> {
    init_env();
    OPSuccinctDataFetcher::new_with_rollup_config().await
}

pub fn parse_u64(s: &str) -> Result<u64, ParseIntError> {
    if let Some(stripped) = s.strip_prefix("0x") {
        u64::from_str_radix(stripped, 16)
    } else {
        s.parse::<u64>()
    }
}

pub async fn get_kroma_host_cli_impl(
    l2_block: u64,
    fetcher_opt: Option<&OPSuccinctDataFetcher>,
) -> Result<HostCli> {
    let fetcher = match fetcher_opt {
        Some(fetcher) => fetcher,
        _ => &init_fetcher().await?,
    };

    Ok(fetcher
        .get_host_cli_args(l2_block - 1, l2_block, ProgramType::Single, CacheMode::DeleteCache)
        .await?)
}

pub async fn get_kroma_host_cli_by_l1_head_hash(
    l2_block: u64,
    l1_head_hash: B256,
    fetcher_opt: Option<&OPSuccinctDataFetcher>,
) -> Result<HostCli> {
    let mut host_cli = get_kroma_host_cli_impl(l2_block, fetcher_opt).await?;
    host_cli.l1_head = l1_head_hash;

    Ok(host_cli)
}

pub async fn get_kroma_host_cli_by_distance(
    l2_block: u64,
    distance_opt: Option<u64>,
    fetcher_opt: Option<&OPSuccinctDataFetcher>,
) -> Result<HostCli> {
    let fetcher = match fetcher_opt {
        Some(fetcher) => fetcher,
        _ => &init_fetcher().await?,
    };

    let mut host_cli = get_kroma_host_cli_impl(l2_block, Some(&fetcher)).await?;

    let distance = distance_opt.unwrap_or(300);

    let report_base = PreviewReport::from_fetcher(l2_block, Some(&fetcher)).await;
    let report_head = report_base.l1_head(distance, Some(&fetcher)).await;

    host_cli.l1_head = report_head.hash;

    Ok(host_cli)
}

#[allow(clippy::too_many_arguments)]
pub async fn report_execution(
    data_fetcher: &OPSuccinctDataFetcher,
    l2_number: u64,
    report: &ExecutionReport,
    witness_generation_time_sec: std::time::Duration,
    execution_duration: std::time::Duration,
) {
    let block_data = data_fetcher.get_l2_block_data_range(l2_number, l2_number).await.unwrap();
    let mut block_data = block_data.to_vec();
    block_data.sort_by_key(|b| b.block_number);

    let get_cycles = |key: &str| *report.cycle_tracker.get(key).unwrap_or(&0);

    let nb_blocks = block_data.len() as u64;
    let nb_transactions: u64 = block_data.iter().map(|b| b.transaction_count).sum();
    let total_gas_used: u64 = block_data.iter().map(|b| b.gas_used).sum();

    let mut stats = ExecutionStats::default();
    stats.batch_start = block_data[0].block_number;
    stats.batch_end = block_data[block_data.len() - 1].block_number;
    stats.total_instruction_count = report.total_instruction_count();
    // TODO(Ethan): set a proper value to `total_sp1_gas` when sp1_sdk is updated.
    stats.total_sp1_gas = 0;
    stats.block_execution_instruction_count = get_cycles("block-execution");
    stats.oracle_verify_instruction_count = get_cycles("oracle-verify");
    stats.derivation_instruction_count = get_cycles("payload-derivation");
    stats.blob_verification_instruction_count = get_cycles("blob-verification");
    stats.bn_add_cycles = get_cycles("precompile-bn-add");
    stats.bn_mul_cycles = get_cycles("precompile-bn-mul");
    stats.bn_pair_cycles = get_cycles("precompile-bn-pair");
    stats.kzg_eval_cycles = get_cycles("precompile-kzg-eval");
    stats.ec_recover_cycles = get_cycles("precompile-ec-recover");
    stats.nb_transactions;
    stats.eth_gas_used = block_data.iter().map(|b| b.gas_used).sum();
    stats.l1_fees = block_data.iter().map(|b| b.total_l1_fees).sum();
    stats.total_tx_fees = block_data.iter().map(|b| b.total_tx_fees).sum();
    stats.nb_blocks;
    stats.cycles_per_block = report.total_instruction_count() / nb_blocks;
    stats.cycles_per_transaction = report.total_instruction_count() / nb_transactions;
    stats.transactions_per_block = nb_transactions / nb_blocks;
    stats.gas_used_per_block = total_gas_used / nb_blocks;
    stats.gas_used_per_transaction = total_gas_used / nb_transactions;
    stats.witness_generation_time_sec = witness_generation_time_sec.as_secs();
    stats.total_execution_time_sec = execution_duration.as_secs();

    println!("{:#?}", stats);

    let l2_chain_id = data_fetcher.get_l2_chain_id().await.unwrap();
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
pub async fn get_l1_block_hash(
    block_number: u64,
    fetcher_opt: Option<&OPSuccinctDataFetcher>,
) -> B256 {
    let fetcher = match fetcher_opt {
        Some(fetcher) => fetcher,
        _ => &init_fetcher().await.unwrap(),
    };

    let l1_head_block = block_on(async move {
        fetcher
            .l1_provider
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

pub async fn get_output_at(block_number: u64, fetcher_opt: Option<&OPSuccinctDataFetcher>) -> B256 {
    let fetcher = match fetcher_opt {
        Some(fetcher) => fetcher,
        _ => &init_fetcher().await.unwrap(),
    };

    let result = get_output_at_impl(fetcher, block_number);
    let output_root = result["outputRoot"].as_str().unwrap().to_string();
    B256::from_hex(output_root).unwrap()
}

pub async fn get_l1_origin_of(
    block_number: u64,
    fetcher_opt: Option<&OPSuccinctDataFetcher>,
) -> (B256, u64) {
    let fetcher = match fetcher_opt {
        Some(fetcher) => fetcher,
        _ => &init_fetcher().await.unwrap(),
    };

    let result = get_output_at_impl(fetcher, block_number);
    let l1_origin = &result["blockRef"]["l1origin"];

    let origin_hash = B256::from_hex(l1_origin["hash"].as_str().unwrap()).unwrap();
    let origin_number = l1_origin["number"].as_u64().unwrap();

    (origin_hash, origin_number)
}
