use alloy_primitives::B256;
use anyhow::Result;
use op_succinct_host_utils::{
    fetcher::{CacheMode, OPSuccinctDataFetcher},
    get_proof_stdin,
    witnessgen::WitnessGenExecutor,
};
use sp1_sdk::{ProverClient, SP1Stdin};
use std::{
    env,
    panic::{self, AssertUnwindSafe},
    sync::Arc,
};

use crate::{
    types::{RequestResult, TaskInfo},
    witness_db::WitnessDB,
    FAULT_PROOF_ELF,
};

#[allow(clippy::redundant_closure)]
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

    if env::var("SKIP_SIMULATION").unwrap_or("false".to_string()) == "true" {
        tracing::info!("Simulation has been started");
        let executor = ProverClient::new();
        let (_, report) = executor
            .execute(FAULT_PROOF_ELF, sp1_stdin.clone())
            .run()
            .map_err(|e| anyhow::anyhow!("Failed to get proof stdin: {:?}", e.to_string()))?;
        tracing::info!(
            "successfully witness result generated - cycle: {:?}",
            report.total_instruction_count()
        );
    }

    Ok(sp1_stdin)
}

pub fn get_status_by_local_id(
    current_task: &mut TaskInfo,
    witness_db: Arc<WitnessDB>,
    l2_hash: &B256,
    l1_head_hash: &B256,
    is_mutable: bool,
) -> Result<RequestResult> {
    // `idle` is `true` if the witness generator is in idle (i.e., current task is empty).
    let idle = current_task.is_empty();
    // `processing` is `true` if the request with local id (`l2_hash` and `l1_head_hash`) is in progress.
    let processing = current_task.is_equal(*l2_hash, *l1_head_hash);

    // `found_witness` is `true` if the witness is found from the db.
    let mut found_witness = false;
    // `meaningful_witness` is `true` if the found witness does not equals to `WitnessResult::EMPTY_WITNESS`.
    // Note that if the witness equals to `WitnessResult::EMPTY_WITNESS` implies that the previous task has been failed.
    let mut meaningful_witness = false;
    let witness = witness_db.get(l2_hash, l1_head_hash);
    if let Some(witness) = witness {
        found_witness = true;
        if !witness.is_empty() {
            meaningful_witness = true;
        }
    }

    // If a meaningful witness exists regardless of the current task, consider it `Complete`.
    if meaningful_witness {
        return Ok(RequestResult::Completed);
    }

    // If there is no currently running task.
    if idle && found_witness {
        if is_mutable {
            println!("db remove: {:#?}", l2_hash);
            witness_db.remove(l2_hash, l1_head_hash).unwrap();
        }
        return Ok(RequestResult::Failed);
    } else if idle && !found_witness {
        return Ok(RequestResult::None);
    } else {
        // Do nothing.
    }

    // If there is a currently running task.
    if processing && found_witness {
        if is_mutable {
            println!("db remove: {:#?}", l2_hash);
            witness_db.remove(l2_hash, l1_head_hash).unwrap();
        }
        Ok(RequestResult::Failed)
    } else if processing && !found_witness {
        // The same request is already being processed.
        Ok(RequestResult::Processing)
    } else {
        // A reqeust is in progress but not equals to this request.
        Err(anyhow::anyhow!("Another request is in progress"))
    }
}
