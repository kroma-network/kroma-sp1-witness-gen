use alloy_primitives::B256;
use anyhow::Result;
use jsonrpc_core::{Error as JsonError, ErrorCode as JsonErrorCode, Result as JsonResult};
use jsonrpc_derive::rpc;
use std::sync::Arc;
use std::sync::RwLock;

use crate::request_witness_impl::{check_request, generate_witness_impl, RequestResult};
use crate::spec_impl::{spec_impl, SpecResult};
use crate::task_info::TaskInfo;

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(spec_impl())
    }

    #[rpc(name = "requestWitness")]
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult>;
}

#[derive(Clone, Default)]
pub struct RpcImpl {
    current_task: Arc<RwLock<TaskInfo>>,
}

impl RpcImpl {
    pub async fn generate_witness(&self, l2_hash: B256, l1_head_hash: B256) -> Result<()> {
        tracing::info!("start to generate witness");

        // Get lock to update the current task.
        let mut current_task = self.current_task.write().unwrap();
        current_task.set(l2_hash, l1_head_hash);
        drop(current_task);

        // Generate witness.
        let _witness_result = generate_witness_impl(l2_hash, l1_head_hash).unwrap();
        tracing::info!("successfully witness result generated");

        // Get lock to release the current task.
        let mut current_task = self.current_task.write().unwrap();
        current_task.release();
        drop(current_task);

        // TODO: store the result to db.
        Ok(())
    }
}

impl Rpc for RpcImpl {
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult> {
        let (l2_hash, l1_head_hash) = check_request(&l2_hash, &l1_head_hash).unwrap();

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
