use alloy_primitives::B256;
use anyhow::Result;
use kroma_common::{task_info::TaskInfo, FAULT_PROOF_ELF};
use op_succinct_host_utils::{
    fetcher::{CacheMode, OPSuccinctDataFetcher},
    get_proof_stdin,
    witnessgen::WitnessGenExecutor,
};
use sp1_sdk::{ProverClient, SP1Stdin};
use std::{
    panic::{self, AssertUnwindSafe},
    sync::Arc,
};

use crate::{types::RequestResult, witness_db::WitnessDB};

pub async fn generate_witness_impl(l2_hash: B256, l1_head_hash: B256) -> Result<SP1Stdin> {
    let data_fetcher = panic::catch_unwind(AssertUnwindSafe(|| OPSuccinctDataFetcher::default()))
        .map_err(|e| anyhow::anyhow!("Failed to create data fetcher: {:?}", e))?;

    // Check the l2 block exists in the chain.
    let l2_header = data_fetcher.get_l2_header(l2_hash.into()).await?;
    let l2_number = l2_header.number;

    // Check the l1 block exists in the chain.
    data_fetcher.get_l1_header(l1_head_hash.into()).await?;

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
        .execute(FAULT_PROOF_ELF, sp1_stdin.clone())
        .run()
        .map_err(|e| anyhow::anyhow!("Failed to get proof stdin: {:?}", e.to_string()))?;
    tracing::info!(
        "successfully witness result generated - cycle: {:?}",
        report.total_instruction_count()
    );

    Ok(sp1_stdin)
}

pub fn get_status_by_local_id(
    current_task: &mut TaskInfo,
    witness_db: Arc<WitnessDB>,
    l2_hash: &B256,
    l1_head_hash: &B256,
) -> Result<RequestResult> {
    // If the witness is empty, it means the witness generation failed.
    if let Some(witness) = witness_db.get(l2_hash, l1_head_hash) {
        if witness.is_empty() {
            Ok(RequestResult::Failed)
        } else {
            Ok(RequestResult::Completed)
        }
    } else if current_task.is_equal(*l2_hash, *l1_head_hash) {
        Ok(RequestResult::Processing)
    } else if !current_task.is_empty() {
        Err(anyhow::anyhow!("Another request is in progress"))
    } else {
        Ok(RequestResult::None)
    }
}
