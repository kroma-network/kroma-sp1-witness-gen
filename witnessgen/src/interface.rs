mod methods;

use crate::{types::TaskInfo, witness_db::WitnessDB};
use jsonrpc_http_server::ServerBuilder;
use methods::{Rpc, RpcImpl};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub static DEFAULT_WITNESS_STORE_PATH: &str = "data/witness_store";
pub static DEFAULT_WITNESSGEN_RPC_ENDPOINT: &str = "0.0.0.0:3030";

pub async fn run<T: ToString>(db: Arc<WitnessDB>, tx: Sender<TaskInfo>, endpoint: T) {
    // Run the server.
    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(RpcImpl::new(tx, db).to_delegate());

    tracing::info!("Starting Witness Generator at {:?}", endpoint.to_string());
    // NOTE(Ethan): We don't want this v3 verification key hash to be used.
    // tracing::info!("verification key hash: {:#?}", VERIFICATION_KEY_HASH.to_string());
    let server = ServerBuilder::new(io)
        .threads(3)
        .max_request_body_size(200 * 1024 * 1024)
        .start_http(&endpoint.to_string().parse().unwrap())
        .unwrap();

    server.wait();
}
