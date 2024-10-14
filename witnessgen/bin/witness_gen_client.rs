use clap::Parser;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_core::client::ClientT;
use jsonrpsee_core::rpc_params;
use kroma_witnessgen::{RequestResult, SpecResult, WitnessResult};
use std::time::Duration;

const CLIENT_TIMEOUT_SEC: u64 = 10800;
const DEFAULT_RPC_SERVER_ENDPOINT: &str = "http://127.0.0.1:3030";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    spec: bool,

    #[clap(short, long)]
    request: bool,

    #[clap(short, long)]
    get: bool,
}

async fn test_spec(cli: HttpClient) {
    let params = rpc_params![];
    let spec: SpecResult = cli.request("spec", params).await.unwrap();
    println!("spec: {:?}", spec);
}

async fn test_request(cli: HttpClient) -> bool {
    // TODO: Change these from hard-coded values to values from the command line
    let l2_head = "0x86df565e6a6e3e266411e3718d5ceba49026606a00624e48c08448f8bf7bc82e";
    let l1_head = "0x5fa696ce65c95ed8e3931a285cbc7d101dc5ac47e0251f44d86c41c2aa9233f6";

    let params = rpc_params![l2_head, l1_head];
    let witness_result: RequestResult = cli.request("requestWitness", params).await.unwrap();

    print!("status: {:?}", witness_result);
    true
}

async fn test_get(cli: HttpClient) -> bool {
    // TODO: Change these from hard-coded values to values from the command line
    let l2_head = "0x86df565e6a6e3e266411e3718d5ceba49026606a00624e48c08448f8bf7bc82e";
    let l1_head = "0x5fa696ce65c95ed8e3931a285cbc7d101dc5ac47e0251f44d86c41c2aa9233f6";

    let params = rpc_params![l2_head, l1_head];
    let witness_result: WitnessResult = cli.request("getWitness", params).await.unwrap();

    print!("witness_result: {:?}", witness_result);
    true
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
    if args.request {
        let _ = test_request(http_client.clone()).await;
    }
    if args.get {
        let _ = test_get(http_client).await;
    }
}
