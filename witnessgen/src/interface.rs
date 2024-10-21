use alloy_primitives::B256;
use anyhow::Result;
use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;
use kroma_utils::task_info::TaskInfo;
use kroma_utils::utils::preprocessing;
use std::sync::Arc;
use std::sync::RwLock;

use crate::errors::WitnessGenError;
use crate::get_witness_impl::WitnessResult;
use crate::request_witness_impl::{generate_witness_impl, RequestResult};
use crate::spec_impl::{spec_impl, SpecResult};
use crate::witness_db::WitnessDB;

static DEFAULT_WITNESS_STORE_PATH: &str = "data/witness_store";

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(spec_impl())
    }

    #[rpc(name = "requestWitness")]
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult>;

    #[rpc(name = "getWitness")]
    fn get_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<WitnessResult>;
}

#[derive(Clone)]
pub struct RpcImpl {
    current_task: Arc<RwLock<TaskInfo>>,
    witness_db: Arc<WitnessDB>,
}

impl RpcImpl {
    pub fn new(store_path: &str) -> Self {
        RpcImpl {
            current_task: Arc::new(RwLock::new(TaskInfo::default())),
            witness_db: Arc::new(WitnessDB::new(store_path)),
        }
    }
}

impl Default for RpcImpl {
    fn default() -> Self {
        Self::new(DEFAULT_WITNESS_STORE_PATH)
    }
}

impl RpcImpl {
    pub async fn generate_witness(&self, l2_hash: B256, l1_head_hash: B256) -> Result<()> {
        tracing::info!("start to generate witness");

        // Get lock to update the current task.
        let mut current_task = self.current_task.write().unwrap();
        current_task.set(l2_hash, l1_head_hash);
        drop(current_task);

        // Generate witness.
        let sp1_stdin = generate_witness_impl(l2_hash, l1_head_hash).unwrap();
        tracing::info!("successfully witness result generated");

        // Get lock to release the current task.
        let mut current_task = self.current_task.write().unwrap();
        current_task.release();
        drop(current_task);

        // Store the witness to db.
        self.witness_db.set(&l2_hash, &l1_head_hash, sp1_stdin.buffer)?;
        tracing::info!("store witness to db");

        Ok(())
    }
}

impl Rpc for RpcImpl {
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

        // Return cached witness if it exists.
        let witness_result = self.witness_db.get(&l2_hash, &l1_head_hash);
        if witness_result.is_ok() {
            tracing::info!("The request is already completed: {:?}", user_req_id);
            return Ok(RequestResult::Completed);
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
            Ok(RequestResult::Requested)
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
        let result = self.witness_db.get(&l2_hash, &l1_head_hash);
        match result {
            Ok(witness_result) => {
                tracing::info!("The proof is already stored: {:?}", user_req_id);
                Ok(WitnessResult::new_from_witness_buf(RequestResult::Completed, witness_result))
            }
            Err(_) => {
                // Check if the request is in progress.
                let current_task = Arc::clone(&self.current_task);
                if current_task.try_read().unwrap().is_equal(l2_hash, l1_head_hash) {
                    tracing::info!("the request is in progress: {:?}", user_req_id);
                    Ok(WitnessResult::new(RequestResult::Processing, ""))
                } else {
                    tracing::info!("the request is not found: {:?}", user_req_id);
                    Ok(WitnessResult::new(RequestResult::None, ""))
                }
            }
        }
    }
}
