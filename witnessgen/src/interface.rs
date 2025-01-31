use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;
use kroma_common::utils::preprocessing;
use std::sync::{Arc, RwLock};

use crate::errors::WitnessGenError;
use crate::types::{RequestResult, SpecResult, TaskInfo, WitnessResult};
use crate::utils::get_status_by_local_id;
use crate::witness_db::WitnessDB;

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult>;

    #[rpc(name = "requestWitness")]
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult>;

    #[rpc(name = "getWitness")]
    fn get_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<WitnessResult>;
}

pub struct RpcImpl {
    pub tx: tokio::sync::mpsc::Sender<TaskInfo>,
    pub current_task: Arc<RwLock<TaskInfo>>,
    pub witness_db: Arc<WitnessDB>,
}

impl RpcImpl {
    pub fn new(tx: tokio::sync::mpsc::Sender<TaskInfo>, witness_db: Arc<WitnessDB>) -> Self {
        RpcImpl { tx, current_task: Arc::new(RwLock::new(TaskInfo::default())), witness_db }
    }

    pub fn update_prev_req_status(&self) {
        let mut current_task = self.current_task.write().unwrap();
        if current_task.is_empty() {
            return;
        }

        // Flush `current_task` if the previous request has been completed.
        if self.witness_db.get(&current_task.l2_hash, &current_task.l2_hash).is_some() {
            current_task.release();
        }
    }

    // Release `current_task` if the witness related to the `current_task` is already generated.
    pub fn release_current_task_if_completed(&self, current_task: &mut TaskInfo) {
        if let Some(witness) =
            self.witness_db.get(&current_task.l2_hash, &current_task.l1_head_hash)
        {
            if !witness.is_empty() {
                current_task.release();
            }
        }
    }

    // Release `current_task` if the witness related to the `current_task` has been faild.
    pub fn release_current_task_if_failed(&self, current_task: &mut TaskInfo) {
        if let Some(witness) =
            self.witness_db.get(&current_task.l2_hash, &current_task.l1_head_hash)
        {
            if witness.is_empty() {
                current_task.release();
            }
        }
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
        self.update_prev_req_status();

        let mut current_task = self.current_task.write().unwrap();
        self.release_current_task_if_completed(&mut current_task);
        self.release_current_task_if_failed(&mut current_task);

        match get_status_by_local_id(
            &mut current_task,
            self.witness_db.clone(),
            &l2_hash,
            &l1_head_hash,
            true,
        ) {
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
                current_task.set(l2_hash, l1_head_hash);

                let tx = self.tx.clone();
                tokio::task::spawn(async move {
                    let task = TaskInfo { l2_hash, l1_head_hash };
                    tx.send(task).await.unwrap();
                });
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
        self.update_prev_req_status();

        // Return cached witness if it exists. Otherwise, start to generate witness.
        let mut current_task = self.current_task.write().unwrap();
        self.release_current_task_if_completed(&mut current_task);
        self.release_current_task_if_failed(&mut current_task);

        match get_status_by_local_id(
            &mut current_task,
            self.witness_db.clone(),
            &l2_hash,
            &l1_head_hash,
            false,
        ) {
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
