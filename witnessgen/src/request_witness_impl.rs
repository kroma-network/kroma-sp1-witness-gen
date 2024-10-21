use alloy_primitives::B256;
use anyhow::Result;
use op_succinct_host_utils::{
    fetcher::{CacheMode, OPSuccinctDataFetcher, RPCMode},
    get_proof_stdin,
    witnessgen::WitnessGenExecutor,
};
use serde::{Deserialize, Serialize};
use sp1_sdk::{ProverClient, SP1Stdin};

use crate::spec_impl::SINGLE_BLOCK_ELF;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RequestResult {
    None,
    Processing,
    Completed,
    Failed,
}

pub async fn generate_witness_impl(l2_hash: B256, l1_head_hash: B256) -> Result<SP1Stdin> {
    let data_fetcher = OPSuccinctDataFetcher::new().await;

    // Check the l2 block exists in the chain.
    let l2_header = data_fetcher.get_header_by_hash(RPCMode::L2, l2_hash).await?;
    let l2_number = l2_header.number;

    // Check the l1 block exists in the chain.
    data_fetcher.get_header_by_hash(RPCMode::L1, l1_head_hash).await?;

    // Prepare the host CLI args.
    let mut host_cli = data_fetcher
        .get_host_cli_args(
            l2_number - 1,
            l2_number,
            op_succinct_host_utils::ProgramType::Single,
            CacheMode::KeepCache,
        )
        .await?;
    host_cli.l1_head = l1_head_hash;

    // Start the server and native client.
    let mut witnessgen_executor = WitnessGenExecutor::default();
    witnessgen_executor.spawn_witnessgen(&host_cli).await?;
    witnessgen_executor.flush().await?;

    let sp1_stdin = get_proof_stdin(&host_cli)
        .map_err(|e| anyhow::anyhow!("Failed to get proof stdin: {:?}", e.to_string()))?;
    let executor = ProverClient::new();
    let (_, report) = executor
        .execute(SINGLE_BLOCK_ELF, sp1_stdin.clone())
        .run()
        .map_err(|e| anyhow::anyhow!("Failed to get proof stdin: {:?}", e.to_string()))?;
    tracing::info!(
        "successfully witness result generated - cycle: {:?}",
        report.total_instruction_count()
    );

    Ok(sp1_stdin)
}
