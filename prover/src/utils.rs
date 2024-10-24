use std::sync::Arc;

use anyhow::Result;
use kroma_utils::deps_version::SP1_SDK_VERSION;
use kroma_witnessgen::get_witness_impl::WitnessResult;
use sp1_sdk::network::client::NetworkClient;
use sp1_sdk::proto::network::{ProofMode, ProofStatus};
use sp1_sdk::{block_on, SP1ProofWithPublicValues, SP1Stdin};

use crate::proof_db::ProofDB;
use crate::types::{RequestResult, SINGLE_BLOCK_ELF};

pub fn request_prove_to_sp1(client: &Arc<NetworkClient>, witness: String) -> Result<String> {
    // Recover a SP1Stdin from the witness string.
    let mut sp1_stdin = SP1Stdin::new();
    sp1_stdin.buffer = WitnessResult::string_to_witness_buf(&witness);

    // Send a request to generate a proof to the sp1 network.
    let request_id = block_on(async move {
        client.create_proof(SINGLE_BLOCK_ELF, &sp1_stdin, ProofMode::Plonk, SP1_SDK_VERSION).await
    })?;
    Ok(request_id)
}

pub fn get_proof_status_from_sp1(
    client: &Arc<NetworkClient>,
    proof_db: &Arc<ProofDB>,
    request_id: &str,
) -> Result<RequestResult> {
    match block_on(async { client.get_proof_status::<SP1ProofWithPublicValues>(request_id).await })
    {
        Ok((response, maybe_proof)) => match response.status() {
            ProofStatus::ProofFulfilled => {
                proof_db.set_proof(&request_id, &maybe_proof.unwrap())?;
                Ok(RequestResult::Completed)
            }
            ProofStatus::ProofPreparing
            | ProofStatus::ProofRequested
            | ProofStatus::ProofClaimed => Ok(RequestResult::Processing),
            ProofStatus::ProofUnclaimed => {
                Ok(RequestResult::Failed(response.unclaim_description.unwrap()))
            }
            ProofStatus::ProofUnspecifiedStatus => {
                tracing::error!("The proof status is unspecified: {:?}", request_id);
                Ok(RequestResult::None)
            }
        },
        // There is only one error case: "Failed to get proof status"
        Err(_) => Ok(RequestResult::None),
    }
}
