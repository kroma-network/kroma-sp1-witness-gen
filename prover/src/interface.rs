use anyhow::Result;
use jsonrpc_core::{Error as JsonError, Result as JsonResult};
use jsonrpc_derive::rpc;
use kroma_utils::deps_version::SP1_SDK_VERSION;
use kroma_utils::utils::preprocessing;
use kroma_witnessgen::get_witness_impl::WitnessResult;
use sp1_sdk::network::client::NetworkClient;
use sp1_sdk::proto::network::{ProofMode, ProofStatus};
use sp1_sdk::{block_on, SP1ProofWithPublicValues, SP1Stdin};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::errors::ProverError;
use crate::get_proof_impl::ProofResult;
use crate::proof_db::ProofDB;
use crate::request_prove_impl::RequestResult;
use crate::spec_impl::{spec_impl, SpecResult, SINGLE_BLOCK_ELF};

static DEFAULT_PROOF_STORE_PATH: &str = "data/proof_store";
static DEFAULT_PROOF_WAIT_TIMEOUT_SEC: u64 = 14_400;
static DEFAULT_PROOF_WAIT_POLLING_RATE: u64 = 60;

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
    proof_db: Arc<RwLock<ProofDB>>,
    client: Arc<NetworkClient>,
    proof_wait_timeout: u64,
}

impl RpcImpl {
    pub fn new(store_path: &str, sp1_private_key: &str, proof_wait_timeout: Option<u64>) -> Self {
        RpcImpl {
            proof_db: Arc::new(RwLock::new(ProofDB::new(store_path.into()))),
            client: Arc::new(NetworkClient::new(&sp1_private_key)),
            proof_wait_timeout: proof_wait_timeout.unwrap_or(DEFAULT_PROOF_WAIT_TIMEOUT_SEC),
        }
    }
}

impl Default for RpcImpl {
    fn default() -> Self {
        let sp1_private_key = std::env::var("SP1_PRIVATE_KEY")
            .expect("SP1_PRIVATE_KEY must be set for remote proving");
        Self::new(DEFAULT_PROOF_STORE_PATH, &sp1_private_key, None)
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
        let timeout = Duration::from_secs(self.proof_wait_timeout);

        let start_time = Instant::now();
        let proof = loop {
            let (response, maybe_proof) = block_on(async {
                self.client.get_proof_status::<SP1ProofWithPublicValues>(&request_id).await.unwrap()
            });

            if maybe_proof.is_some() {
                assert_eq!(response.status(), ProofStatus::ProofFulfilled);
                break Some(maybe_proof.unwrap());
            }

            tracing::info!("waiting for proof: {:?}", response.status());

            if start_time.elapsed() > timeout {
                tracing::error!("timeout to wait for proof: {:?}", request_id);
                break None;
            }

            std::thread::sleep(Duration::from_secs(DEFAULT_PROOF_WAIT_POLLING_RATE));
        };

        if let Some(proof) = proof {
            self.proof_db.write().unwrap().set_proof(&request_id, &proof)?;
            tracing::info!("proof saved to db");
        }

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

        // If this request has been made before, the `request_prove` method will terminate here.
        let proof_db = self.proof_db.read().unwrap();
        if let Ok(_) = proof_db.get_request_id(&l2_hash, &l1_head_hash) {
            if let Ok(_) = proof_db.get_proof(&l2_hash, &l1_head_hash) {
                // The request is already completed.
                tracing::info!("The request is already completed: {:?}", user_req_id);
                return Ok(RequestResult::Completed);
            } else {
                // The request is in processing.
                tracing::info!("The request is in processing: {:?}", user_req_id);
                return Ok(RequestResult::Processing);
            }
        }
        drop(proof_db);

        // Send a request to SP1 network.
        let proof_db = self.proof_db.write().unwrap();
        let service = self.clone();
        let request_id = block_on(async move { service.request_prove_to_sp1(witness).await })
            .map_err(|e| {
                tracing::error!("Failed to send request to SP1 network: {:?}", e);
                ProverError::sp1_network_error(e.to_string())
            })?;
        tracing::info!("Sent request to SP1 network: {:?}", request_id);

        // Store the `request_id` to the database.
        proof_db.set_request_id(&l2_hash, &l1_head_hash, &request_id).map_err(|e| {
            tracing::info!("The database is full: {:?}", e.to_string());
            ProverError::db_error(e.to_string())
        })?;
        drop(proof_db);

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
        let proof_db = self.proof_db.read().unwrap();
        let request_id = match proof_db.get_request_id(&l2_hash, &l1_head_hash) {
            Ok(id) => id,
            Err(_) => {
                tracing::info!("The request is not found: {:?}", user_req_id);
                return Ok(ProofResult::none());
            }
        };

        // Check if the proof is already stored.
        if let Ok(proof) = proof_db.get_proof(&l2_hash, &l1_head_hash) {
            tracing::info!("The proof is already stored: {:?}", request_id);
            return Ok(ProofResult::new(
                &request_id,
                RequestResult::Completed,
                proof.public_values.raw(),
                proof.raw(),
            ));
        }
        drop(proof_db);

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

                let proof_db = self.proof_db.write().unwrap();
                proof_db.set_proof(&request_id, &proof).unwrap();
                tracing::info!("The proof is stored to database: {:?}", request_id);
                drop(proof_db);

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
