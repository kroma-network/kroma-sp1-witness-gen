use alloy_primitives::B256;
use anyhow::Result;
use kroma_common::deps_version::SP1_SDK_VERSION;
use kroma_witnessgen::types::WitnessResult;
use sp1_sdk::network::client::NetworkClient;
use sp1_sdk::network::proto::network::{ProofMode, ProofStatus};
use sp1_sdk::{block_on, SP1ProofWithPublicValues, SP1Stdin};
use std::sync::Arc;

use crate::proof_db::ProofDB;
use crate::types::{RequestResult, SINGLE_BLOCK_ELF};

pub fn request_prove_to_sp1(client: &Arc<NetworkClient>, witness: String) -> Result<String> {
    // Recover a SP1Stdin from the witness string.
    let mut sp1_stdin = SP1Stdin::new();
    sp1_stdin.buffer = WitnessResult::string_to_witness_buf(&witness);

    // Send a request to generate a proof to the sp1 network.
    tracing::debug!("ready to send request to SP1 network prover");
    let request_id = block_on(async move {
        client.create_proof(SINGLE_BLOCK_ELF, &sp1_stdin, ProofMode::Plonk, SP1_SDK_VERSION).await
    })?;
    tracing::debug!("Sent the request to SP1 network prover: {:?}", request_id);
    Ok(request_id)
}

pub fn get_status_by_local_id(
    client: &Arc<NetworkClient>,
    proof_db: &Arc<ProofDB>,
    l2_hash: &B256,
    l1_head_hash: &B256,
) -> RequestResult {
    let request_id = proof_db.get_request_id(l2_hash, l1_head_hash);
    match request_id {
        Some(id) => get_status_by_remote_id(client, proof_db, &id),
        None => RequestResult::None,
    }
}

pub fn get_status_by_remote_id(
    client: &Arc<NetworkClient>,
    proof_db: &Arc<ProofDB>,
    request_id: &str,
) -> RequestResult {
    match block_on(async { client.get_proof_status(request_id).await }) {
        Ok((response, maybe_proof)) => match response.status() {
            ProofStatus::ProofFulfilled => {
                proof_db.set_proof(&request_id, &maybe_proof.unwrap()).unwrap();
                RequestResult::Completed
            }
            ProofStatus::ProofPreparing
            | ProofStatus::ProofRequested
            | ProofStatus::ProofClaimed => RequestResult::Processing,
            ProofStatus::ProofUnclaimed => {
                RequestResult::Failed(response.unclaim_description.unwrap())
            }
            ProofStatus::ProofUnspecifiedStatus => {
                tracing::error!("The proof status is unspecified: {:?}", request_id);
                RequestResult::None
            }
        },
        // There is only one error case: "Failed to get proof status"
        Err(_) => RequestResult::None,
    }
}

pub fn get_proof_by_local_id(
    proof_db: &Arc<ProofDB>,
    l2_hash: &B256,
    l1_head_hash: &B256,
) -> Option<SP1ProofWithPublicValues> {
    let request_id = proof_db.get_request_id(l2_hash, l1_head_hash);
    match request_id {
        Some(id) => proof_db.get_proof_by_id(&id),
        None => None,
    }
}
