use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use jsonrpc_http_server::ServerBuilder;
use kroma_witnessgen::{
    executor::Executor,
    interface::{Rpc, RpcImpl},
};

static DEFAULT_WITNESS_STORE_PATH: &str = "data/witness_store";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "endpoint", default_value = "127.0.0.1:3030")]
    endpoint: String,

    #[clap(short, long = "data", default_value = DEFAULT_WITNESS_STORE_PATH)]
    data_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::Subscriber::builder().init();

    // Check if the endpoints are empty.
    // TODO: implement check_endpoints

    let args = Args::parse();

    let witness_db = Arc::new(kroma_witnessgen::witness_db::WitnessDB::new(&args.data_path));
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    // Run the executor in a separate task.
    let witness_db_for_executor = witness_db.clone();
    tokio::task::spawn(async {
        let mut executor = Executor::new(rx, witness_db_for_executor);
        executor.run().await;
    });

    // Run the server.
    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(RpcImpl::new(tx, witness_db).to_delegate());

    tracing::info!("Starting Witness Generator at {}", args.endpoint);
    tracing::info!("Program Key: {:#?}", kroma_common::PROGRAM_KEY.to_string());
    let server = ServerBuilder::new(io)
        .threads(3)
        .max_request_body_size(200 * 1024 * 1024)
        .start_http(&args.endpoint.parse().unwrap())
        .unwrap();

    server.wait();

    Ok(())
}
