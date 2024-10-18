use anyhow::Result;
use jsonrpc_core::{Error as JsonError, Result as JsonResult};
use jsonrpc_derive::rpc;
use kroma_utils::deps_version::SP1_SDK_VERSION;
use kroma_utils::utils::preprocessing;
use kroma_witnessgen::get_witness_impl::WitnessResult;
use sp1_sdk::network::client::NetworkClient;
use sp1_sdk::proto::network::{ProofMode, ProofStatus};
use sp1_sdk::{block_on, SP1ProofWithPublicValues, SP1Stdin};
use std::sync::Arc;
use std::time::Duration;

use crate::errors::ProverError;
use crate::get_proof_impl::ProofResult;
use crate::proof_db::ProofDB;
use crate::request_prove_impl::RequestResult;
use crate::spec_impl::{spec_impl, SpecResult, SINGLE_BLOCK_ELF};

static DEFAULT_PROOF_STORE_PATH: &str = "data/proof_store";

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult>;

    #[rpc(name = "requestProve")]
    fn request_prove(
        &self,
        l2_hash: String,
        l1_head_hash: String,
        witness: String,
    ) -> JsonResult<RequestResult>;

    #[rpc(name = "getProof")]
    fn get_proof(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<ProofResult>;
}

#[derive(Clone)]
pub struct RpcImpl {
    proof_db: Arc<ProofDB>,
    client: Arc<NetworkClient>,
}

impl RpcImpl {
    pub fn new(store_path: &str, sp1_private_key: &str) -> Self {
        RpcImpl {
            proof_db: Arc::new(ProofDB::new(store_path.into())),
            client: Arc::new(NetworkClient::new(&sp1_private_key)),
        }
    }
}

impl Default for RpcImpl {
    fn default() -> Self {
        let sp1_private_key = std::env::var("SP1_PRIVATE_KEY")
            .expect("SP1_PRIVATE_KEY must be set for remote proving");
        Self::new(DEFAULT_PROOF_STORE_PATH, &sp1_private_key)
    }
}

impl RpcImpl {
    async fn request_prove_to_sp1(&self, witness: String) -> Result<String> {
        // Recover a SP1Stdin from the witness string.
        let mut sp1_stdin = SP1Stdin::new();
        sp1_stdin.buffer = WitnessResult::string_to_witness_buf(&witness);

        // Send a request to generate a proof to the sp1 network.
        self.client
            .create_proof(SINGLE_BLOCK_ELF, &sp1_stdin, ProofMode::Plonk, SP1_SDK_VERSION)
            .await
    }

    async fn wait_proof(&self, request_id: String) -> Result<()> {
        // TODO: Add a timeout.
        let proof = loop {
            let (response, maybe_proof) = block_on(async {
                self.client.get_proof_status::<SP1ProofWithPublicValues>(&request_id).await.unwrap()
            });

            if maybe_proof.is_some() {
                assert_eq!(response.status(), ProofStatus::ProofFulfilled);
                break maybe_proof.unwrap();
            }

            tracing::info!("waiting for proof: {:?}", response.status());
            std::thread::sleep(Duration::from_secs(30));
        };

        //TODO: it should get a db lock (TBD).
        self.proof_db.set_proof(&request_id, &proof)?;
        //TODO: it should release a db lock (TBD).
        tracing::info!("proof saved to db");

        Ok(())
    }
}

impl Rpc for RpcImpl {
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(spec_impl())
    }

    fn request_prove(
        &self,
        l2_hash: String,
        l1_head_hash: String,
        witness: String,
    ) -> JsonResult<RequestResult> {
        let (l2_hash, l1_head_hash, user_req_id) =
            preprocessing(&l2_hash, &l1_head_hash).map_err(|e| {
                tracing::error!(
                    "Invalid parameters - \"l2_hash\": {:?}, \"l1_head_hash\": {:?}",
                    l2_hash,
                    l1_head_hash
                );
                ProverError::invalid_input_hash(e.to_string())
            })?;

        // TODO: return error if the request is already requested.
        // ProverError::occupied(user_req_id)

        // Check if the request has been requested.
        // TODO: it should get a db lock (TBD).
        if let Ok(_) = self.proof_db.get_request_id(&l2_hash, &l1_head_hash) {
            tracing::info!("The request have been requested: {:?}", user_req_id);
            return Ok(RequestResult::Completed);
        }

        // TODO: execute guest program to validate witness.
        // ProverError::failed_to_execute_witness()

        // Send a request to SP1 network.
        let service = self.clone();
        let request_id = block_on(async move { service.request_prove_to_sp1(witness).await })
            .map_err(|e| {
                tracing::error!("Failed to send request to SP1 network: {:?}", e);
                ProverError::sp1_network_error(e.to_string())
            })?;
        tracing::info!("Sent request to SP1 network: {:?}", request_id);

        // Store the `request_id` to the database.
        self.proof_db
            .set_request_id(&l2_hash, &l1_head_hash, &request_id)
            .map_err(|e| ProverError::db_error(e.to_string()))?;

        // TODO: it should release a db lock (TBD).

        // Wait for the proof to be generated.
        let service = self.clone();
        tokio::spawn(async move {
            // TODO: handle timeout error.
            let _ = service.wait_proof(request_id).await;
        });

        Ok(RequestResult::Processing)
    }

    fn get_proof(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<ProofResult> {
        let (l2_hash, l1_head_hash, user_req_id) =
            preprocessing(&l2_hash, &l1_head_hash).map_err(|e| {
                tracing::error!(
                    "Invalid parameters - \"l2_hash\": {:?}, \"l1_head_hash\": {:?}",
                    l2_hash,
                    l1_head_hash
                );
                ProverError::invalid_input_hash(e.to_string())
            })?;

        // Check if it has been requested.
        // TODO: it should get a db lock (TBD).
        let request_id = match self.proof_db.get_request_id(&l2_hash, &l1_head_hash) {
            Ok(id) => id,
            Err(_) => {
                return Ok(ProofResult::none());
            }
        };

        // Check if the proof is already stored.
        if let Ok(proof) = self.proof_db.get_proof(&l2_hash, &l1_head_hash) {
            return Ok(ProofResult::new(
                &request_id,
                RequestResult::Completed,
                proof.public_values.raw(),
                proof.raw(),
            ));
        }
        // TODO: it should release a db lock (TBD).

        // Check if the proof is already being generated in SP1 network.
        let (response, maybe_proof) = block_on(async {
            self.client.get_proof_status::<SP1ProofWithPublicValues>(&request_id).await
        })
        .map_err(|e| {
            tracing::error!(
                "Failed to get proof status from SP1 network - \"user_req_id\": {:?}, \"request_id\": {:?}",
                user_req_id,
                request_id
            );
            ProverError::sp1_network_error(e.to_string())
        })?;

        match response.status() {
            ProofStatus::ProofFulfilled => {
                // Store the proof to the database.
                let proof = maybe_proof.unwrap();
                tracing::info!("The proof is fetched from SP1 network: {:?}", request_id);

                // TODO: it should get a db lock (TBD).
                self.proof_db.set_proof(&request_id, &proof).unwrap();
                // TODO: it should release a db lock (TBD).
                Ok(ProofResult::new(
                    &request_id,
                    RequestResult::Completed,
                    proof.public_values.raw(),
                    proof.raw(),
                ))
            }
            ProofStatus::ProofPreparing
            | ProofStatus::ProofRequested
            | ProofStatus::ProofClaimed => {
                tracing::info!("The proof is in processing: {:?}", request_id);
                Ok(ProofResult::processing(request_id))
            }
            ProofStatus::ProofUnspecifiedStatus => {
                tracing::info!("The request is not found: {:?}", request_id);
                Ok(ProofResult::none())
            }
            ProofStatus::ProofUnclaimed => {
                let msg = format!("Unexpected status: {:?}", response.status());
                Err(JsonError::from(ProverError::unexpected(Some(msg))))
            }
        }
    }
}
