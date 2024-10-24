use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;
use kroma_utils::utils::preprocessing;
use sp1_sdk::network::client::NetworkClient;
use std::sync::{Arc, RwLock};

use crate::errors::ProverError;
use crate::proof_db::ProofDB;
use crate::types::{ProofResult, RequestResult, SpecResult};

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
    task_lock: Arc<RwLock<()>>,
    proof_db: Arc<ProofDB>,
    client: Arc<NetworkClient>,
}

impl RpcImpl {
    pub fn new(store_path: &str, sp1_private_key: &str) -> Self {
        RpcImpl {
            task_lock: Arc::new(RwLock::new(())),
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

impl Rpc for RpcImpl {
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(SpecResult::default())
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
                ProverError::invalid_input_hash(e.to_string()).to_json_error()
            })?;

        // If this request has been made before, the `request_prove` method will terminate here.
        let guard = self.task_lock.read().unwrap();
        if let Some(request_id) = self.proof_db.get_request_id(&l2_hash, &l1_head_hash) {
            match self.proof_db.get_proof_by_id(&request_id) {
                Some(_) => {
                    // The request is already completed.
                    tracing::info!(
                        "The request is already completed: {:?}, {:?}",
                        user_req_id,
                        request_id
                    );
                    return Ok(RequestResult::Completed);
                }
                None => {
                    // The request is in processing.
                    tracing::info!(
                        "The request is in processing: {:?}, {:?}",
                        user_req_id,
                        request_id
                    );
                    return Ok(RequestResult::Processing);
                }
            }
        }
        drop(guard);

        // Send a request to SP1 network.
        let guard = self.task_lock.write().unwrap();
        let request_id =
            crate::utils::request_prove_to_sp1(&self.client, witness).map_err(|e| {
                tracing::error!("Failed to send request to SP1 network: {:?}", e);
                ProverError::sp1_network_error(e.to_string()).to_json_error()
            })?;
        tracing::info!("Sent request to SP1 network: {:?}", request_id);

        // Store the `request_id` to the database.
        self.proof_db.set_request_id(&l2_hash, &l1_head_hash, &request_id).map_err(|e| {
            tracing::info!("The database is full: {:?}", e.to_string());
            ProverError::db_error(e.to_string()).to_json_error()
        })?;
        drop(guard);

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
                ProverError::invalid_input_hash(e.to_string()).to_json_error()
            })?;

        // Check if it has been requested.
        let guard = self.task_lock.read().unwrap();
        let request_id = match self.proof_db.get_request_id(&l2_hash, &l1_head_hash) {
            Some(id) => id,
            None => {
                tracing::info!("The request is not found: {:?}", user_req_id);
                return Ok(ProofResult::none());
            }
        };

        // Check if the proof is already stored.
        if let Some(proof) = self.proof_db.get_proof(&l2_hash, &l1_head_hash) {
            tracing::info!("The proof is already stored: {:?}, {:?}", user_req_id, request_id);
            return Ok(ProofResult::new(
                &request_id,
                RequestResult::Completed,
                proof.public_values.raw(),
                proof.raw(),
            ));
        }
        drop(guard);

        // Check if the proof is already being generated in SP1 network.
        let request_result =
            crate::utils::get_proof_status_from_sp1(&self.client, &self.proof_db, &request_id)
                .map_err(|e| {
                    tracing::error!(
                        "Failed to get proof status from SP1 network: {:?}, {:?}",
                        user_req_id,
                        request_id
                    );
                    ProverError::sp1_network_error(e.to_string()).to_json_error()
                })?;

        match request_result {
            RequestResult::Completed => {
                let _guard = self.task_lock.read().unwrap();
                let proof = self.proof_db.get_proof_by_id(&request_id).unwrap();
                Ok(ProofResult::new(
                    &request_id,
                    RequestResult::Completed,
                    proof.public_values.raw(),
                    format!("0x{}", proof.raw()),
                ))
            }
            RequestResult::Processing => {
                tracing::info!("The proof is in processing: {:?}, {:?}", user_req_id, request_id);
                Ok(ProofResult::processing(request_id))
            }
            RequestResult::None => {
                tracing::info!("The request is not found: {:?}, {:?}", user_req_id, request_id);
                Ok(ProofResult::none())
            }
            RequestResult::Failed(msg) => {
                tracing::info!("The request has been failed: {:?}", msg);
                Ok(ProofResult::failed(request_id, msg))
            }
        }
    }
}
