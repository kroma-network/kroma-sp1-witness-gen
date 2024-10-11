use alloy_primitives::B256;
use anyhow::Result;
use jsonrpc_core::{Error as JsonError, ErrorCode as JsonErrorCode, Result as JsonResult};
use jsonrpc_derive::rpc;
use std::sync::Arc;
use std::sync::RwLock;

use crate::db::WitnessStore;
use crate::get_witness_impl::WitnessResult;
use crate::request_witness_impl::{check_request, generate_witness_impl, RequestResult};
use crate::spec_impl::{spec_impl, SpecResult};
use crate::task_info::TaskInfo;

static DEFAULT_WITNESS_STORE_PATH: &str = "data/witness_store";

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(spec_impl())
    }

    #[rpc(name = "requestWitness")]
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult>;
}

#[derive(Clone)]
pub struct RpcImpl {
    current_task: Arc<RwLock<TaskInfo>>,
    witness_db: Arc<WitnessStore>,
}

impl RpcImpl {
    pub fn new(store_path: &str) -> Self {
        RpcImpl {
            current_task: Arc::new(RwLock::new(TaskInfo::default())),
            witness_db: Arc::new(WitnessStore::new(store_path.into())),
        }
    }
}

impl Default for RpcImpl {
    fn default() -> Self {
        RpcImpl {
            current_task: Arc::new(RwLock::new(TaskInfo::default())),
            witness_db: Arc::new(WitnessStore::new(DEFAULT_WITNESS_STORE_PATH.into())),
        }
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
        let witness_result = WitnessResult::new(RequestResult::Completed, Some(sp1_stdin.buffer));
        self.witness_db.set(l2_hash, l1_head_hash, witness_result)?;
        tracing::info!("store witness to db");

        Ok(())
    }
}

impl Rpc for RpcImpl {
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult> {
        let (l2_hash, l1_head_hash) = check_request(&l2_hash, &l1_head_hash).unwrap();

        // Return cached witness if it exists.
        let witness_result = self.witness_db.get(l2_hash, l1_head_hash);
        if witness_result.is_ok() {
            tracing::info!("return cached witness");
            return Ok(RequestResult::Completed);
        }

        let current_task = self.current_task.read().unwrap();
        if current_task.is_empty() {
            // Return error if the request is already in progress.
            drop(current_task);

            tracing::info!("do generate witness");
            let service = self.clone();
            tokio::task::spawn(
                async move { service.generate_witness(l2_hash, l1_head_hash).await },
            );
            Ok(RequestResult::Requested)
        } else if current_task.is_equal(l2_hash, l1_head_hash) {
            tracing::info!("the request in progress");
            Ok(RequestResult::Processing)
        } else {
            return Err(JsonError {
                code: JsonErrorCode::ServerError(1),
                message: "The server is busy".to_string(),
                data: None,
            });
        }
    }
}
