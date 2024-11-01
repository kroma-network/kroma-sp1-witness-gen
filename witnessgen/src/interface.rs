use alloy_primitives::B256;
use anyhow::Result;
use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;
use kroma_common::{task_info::TaskInfo, utils::preprocessing};
use std::sync::{Arc, RwLock};

use crate::errors::WitnessGenError;
use crate::types::{RequestResult, SpecResult, WitnessResult};
use crate::utils::get_status_by_local_id;
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
                WitnessGenError::invalid_input_hash(e.to_string()).to_json_error()
            })?;

        let current_task = self.current_task.read().unwrap();
        match get_status_by_local_id(&current_task, &self.witness_db, &l2_hash, &l1_head_hash) {
            Ok(RequestResult::Completed) => {
                tracing::info!("The request is already completed: {:?}", user_req_id);
                Ok(RequestResult::Completed)
            }
            Ok(RequestResult::Processing) => {
                tracing::info!("the request is in progress: {:?}", user_req_id);
                Ok(RequestResult::Processing)
            }
            Ok(RequestResult::Failed) | Ok(RequestResult::None) => {
                tracing::info!("start to generate witness");
                let service = self.clone();
                drop(current_task);
                tokio::task::spawn(
                    async move { service.generate_witness(l2_hash, l1_head_hash).await },
                );
                Ok(RequestResult::Processing)
            }
            Err(e) => {
                tracing::error!("{:?}", e);
                Err(WitnessGenError::already_in_progress(e.to_string()).to_json_error())
            }
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
                WitnessGenError::invalid_input_hash(e.to_string()).to_json_error()
            })?;

        // Return cached witness if it exists. Otherwise, start to generate witness.
        let current_task = self.current_task.read().unwrap();
        match get_status_by_local_id(&current_task, &self.witness_db, &l2_hash, &l1_head_hash) {
            Ok(RequestResult::Completed) => {
                let witness = self.witness_db.get(&l2_hash, &l1_head_hash).unwrap();
                tracing::info!("The request is already completed: {:?}", user_req_id);
                Ok(WitnessResult::new_from_witness_buf(RequestResult::Completed, witness))
            }
            Ok(status) => {
                tracing::info!("the request's status: {:?}, {:?} ", user_req_id, status);
                Ok(WitnessResult::new_with_status(status))
            }
            Err(_) => {
                tracing::info!(
                    "the request's status: {:?}, {:?} ",
                    user_req_id,
                    RequestResult::None
                );
                Ok(WitnessResult::new_with_status(RequestResult::None))
            }
        }
    }
}
