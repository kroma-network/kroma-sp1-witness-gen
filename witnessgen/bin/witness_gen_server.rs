use anyhow::Result;
use clap::Parser;
use jsonrpc_http_server::ServerBuilder;
use kroma_witnessgen::{
    errors::WitnessError,
    interface::{Rpc, RpcImpl},
    utils::check_endpoints,
};

static WITNESS_STORE_PATH: &str = "data/witness_store";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "endpoint", default_value = "127.0.0.1:3030")]
    endpoint: String,
}

fn main() -> Result<(), WitnessError> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::Subscriber::builder().init();

    // Check if the endpoints are empty.
    check_endpoints()?;

    let args = Args::parse();

    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(RpcImpl::new(WITNESS_STORE_PATH).to_delegate());

    let server = ServerBuilder::new(io)
        .threads(3)
        .max_request_body_size(200 * 1024 * 1024)
        .start_http(&args.endpoint.parse().unwrap())
        .unwrap();

    server.wait();

    Ok(())
}
