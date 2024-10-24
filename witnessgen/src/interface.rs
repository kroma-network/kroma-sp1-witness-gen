use alloy_primitives::B256;
use anyhow::Result;
use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;
use kroma_utils::{task_info::TaskInfo, utils::preprocessing};
use std::sync::{Arc, RwLock};

use crate::errors::WitnessGenError;
use crate::types::{RequestResult, SpecResult, WitnessResult};
use crate::witness_db::WitnessDB;

static DEFAULT_WITNESS_STORE_PATH: &str = "data/witness_store";

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult>;

    #[rpc(name = "requestWitness")]
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult>;

    #[rpc(name = "getWitness")]
    fn get_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<WitnessResult>;
}

#[derive(Clone)]
pub struct RpcImpl {
    pub current_task: Arc<RwLock<TaskInfo>>,
    pub witness_db: Arc<WitnessDB>,
}

impl Default for RpcImpl {
    fn default() -> Self {
        Self::new(DEFAULT_WITNESS_STORE_PATH)
    }
}

impl RpcImpl {
    pub fn new(store_path: &str) -> Self {
        RpcImpl {
            current_task: Arc::new(RwLock::new(TaskInfo::default())),
            witness_db: Arc::new(WitnessDB::new(store_path)),
        }
    }

    async fn generate_witness(&self, l2_hash: B256, l1_head_hash: B256) -> Result<()> {
        crate::utils::generate_witness(self, l2_hash, l1_head_hash).await
    }
}

impl Rpc for RpcImpl {
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(SpecResult::default())
    }

    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult> {
        let (l2_hash, l1_head_hash, user_req_id) =
            preprocessing(&l2_hash, &l1_head_hash).map_err(|e| {
                tracing::error!(
                    "Invalid parameters - \"l2_hash\": {:?}, \"l1_head_hash\": {:?}",
                    l2_hash,
                    l1_head_hash
                );
                WitnessGenError::invalid_input_hash(e.to_string())
            })?;

        // Return cached witness if it exists. Otherwise, start to generate witness.
        // If the witness is empty, it means the witness generation failed, so a retry is required.
        if let Some(witness) = self.witness_db.get(&l2_hash, &l1_head_hash) {
            if !witness.is_empty() {
                tracing::info!("The request is already completed: {:?}", user_req_id);
                return Ok(RequestResult::Completed);
            }
        }

        let current_task = self.current_task.read().unwrap();
        if current_task.is_empty() {
            // Return error if the request is already in progress.
            drop(current_task);

            tracing::info!("start to generate witness");
            let service = self.clone();
            tokio::task::spawn(
                async move { service.generate_witness(l2_hash, l1_head_hash).await },
            );
            Ok(RequestResult::Processing)
        } else if current_task.is_equal(l2_hash, l1_head_hash) {
            tracing::info!("the request in already progress");
            Ok(RequestResult::Processing)
        } else {
            tracing::info!("server is in progress with another request");
            return Err(WitnessGenError::already_in_progress().into());
        }
    }

    fn get_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<WitnessResult> {
        let (l2_hash, l1_head_hash, user_req_id) =
            preprocessing(&l2_hash, &l1_head_hash).map_err(|e| {
                tracing::error!(
                    "Invalid parameters - \"l2_hash\": {:?}, \"l1_head_hash\": {:?}",
                    l2_hash,
                    l1_head_hash
                );
                WitnessGenError::invalid_input_hash(e.to_string())
            })?;

        // Check if it exists in the database.
        match self.witness_db.get(&l2_hash, &l1_head_hash) {
            Some(witness_result) => {
                if witness_result.is_empty() {
                    tracing::info!("The request is not found: {:?}", user_req_id);
                    return Ok(WitnessResult::new_with_status(RequestResult::Failed));
                } else {
                    tracing::info!("The proof is already stored: {:?}", user_req_id);
                    Ok(WitnessResult::new_from_witness_buf(
                        RequestResult::Completed,
                        witness_result,
                    ))
                }
            }
            None => {
                // Check if the request is in progress.
                let current_task = Arc::clone(&self.current_task);
                if current_task.try_read().unwrap().is_equal(l2_hash, l1_head_hash) {
                    tracing::info!("the request is in progress: {:?}", user_req_id);
                    Ok(WitnessResult::new_with_status(RequestResult::Processing))
                } else {
                    tracing::info!("the request is not found: {:?}", user_req_id);
                    Ok(WitnessResult::new_with_status(RequestResult::None))
                }
            }
        }
    }
}
