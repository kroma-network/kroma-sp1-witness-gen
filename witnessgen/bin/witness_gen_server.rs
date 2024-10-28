use anyhow::Result;
use clap::Parser;
use jsonrpc_http_server::ServerBuilder;
use kroma_witnessgen::interface::{Rpc, RpcImpl};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "endpoint", default_value = "127.0.0.1:3030")]
    endpoint: String,

    #[clap(short, long = "data", default_value = "data/witness_store")]
    data_path: String,
}

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::Subscriber::builder().init();

    // Check if the endpoints are empty.
    // TODO: implement check_endpoints

    let args = Args::parse();

    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(RpcImpl::new(&args.data_path).to_delegate());

    tracing::info!("Starting Witness Generator at {}", args.endpoint);
    let server = ServerBuilder::new(io)
        .threads(3)
        .max_request_body_size(200 * 1024 * 1024)
        .start_http(&args.endpoint.parse().unwrap())
        .unwrap();

    server.wait();

    Ok(())
}
