use clap::Parser;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_core::client::ClientT;
use jsonrpsee_core::rpc_params;
use kroma_prover::spec_impl::SpecResult;
use std::time::Duration;

const CLIENT_TIMEOUT_SEC: u64 = 10800;
const DEFAULT_RPC_SERVER_ENDPOINT: &str = "http://127.0.0.1:3031";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    spec: bool,
}

async fn test_spec(cli: HttpClient) {
    let params = rpc_params![];
    let spec: SpecResult = cli.request("spec", params).await.unwrap();
    println!("spec: {:?}", spec);
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let args = Args::parse();

    // TODO: define reasonably `max_request_body_size`
    let http_client = HttpClientBuilder::default()
        .max_request_body_size(300 * 1024 * 1024)
        .request_timeout(Duration::from_secs(CLIENT_TIMEOUT_SEC))
        .build(DEFAULT_RPC_SERVER_ENDPOINT)
        .unwrap();

    if args.spec {
        let _ = test_spec(http_client.clone()).await;
    }
}
