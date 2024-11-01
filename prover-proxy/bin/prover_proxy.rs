use anyhow::Result;
use clap::Parser;
use jsonrpc_http_server::ServerBuilder;
use kroma_prover_proxy::interface::{Rpc, RpcImpl};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "endpoint", default_value = "127.0.0.1:3031")]
    endpoint: String,

    #[clap(short, long = "data", default_value = "data/proof_store")]
    data_path: String,
}

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::Subscriber::builder().init();

    // Check if the endpoints are empty.
    // TODO: implement check_endpoints

    let args = Args::parse();

    let sp1_private_key =
        std::env::var("SP1_PRIVATE_KEY").expect("SP1_PRIVATE_KEY must be set for remote proving");
    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(RpcImpl::new(&args.data_path, &sp1_private_key).to_delegate());

    tracing::info!("Starting Prover at {}", args.endpoint);
    let server = ServerBuilder::new(io)
        .threads(3)
        .max_request_body_size(200 * 1024 * 1024)
        .start_http(&args.endpoint.parse().unwrap())
        .unwrap();

    server.wait();

    Ok(())
}
