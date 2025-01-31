use std::sync::Arc;

use crate::{
    types::{TaskInfo, WitnessResult},
    utils::generate_witness_impl,
    witness_db::WitnessDB,
};

pub struct Executor {
    rx: tokio::sync::mpsc::Receiver<TaskInfo>,
    witness_db: Arc<WitnessDB>,
}

impl Executor {
    pub fn new(rx: tokio::sync::mpsc::Receiver<TaskInfo>, witness_db: Arc<WitnessDB>) -> Self {
        Self { rx, witness_db }
    }

    pub async fn run(&mut self) {
        while let Some(task_info) = self.rx.recv().await {
            let l2_hash = task_info.l2_hash;
            let l1_head_hash = task_info.l1_head_hash;

            // Trying to generate a witness.
            let sp1_stdin = generate_witness_impl(l2_hash, l1_head_hash).await;

            // Store the witness to db.
            match sp1_stdin {
                Ok(value) => {
                    tracing::info!("successfully witness result generated");
                    self.witness_db.set(&l2_hash, &l1_head_hash, value.buffer).unwrap();
                }
                Err(e) => {
                    tracing::info!("failed to generate witness: {:?}", e);
                    self.witness_db
                        .set(&l2_hash, &l1_head_hash, WitnessResult::EMPTY_WITNESS)
                        .unwrap();
                }
            }
        }
    }
}
