use clap::Parser;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_core::client::ClientT;
use jsonrpsee_core::rpc_params;
use kroma_common::{FAULT_PROOF_ELF, test_ctx::{OP_SEPOLIA_L2_HEAD, SEPOLIA_L1_HEAD}};
use kroma_witnessgen::types::{RequestResult, SpecResult, WitnessResult};
use sp1_sdk::{ProverClient, SP1Stdin};
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
    let params = rpc_params![OP_SEPOLIA_L2_HEAD, SEPOLIA_L1_HEAD];
    let witness_result: RequestResult = cli.request("requestWitness", params).await.unwrap();

    print!("status: {:?}", witness_result);
    true
}

async fn test_get(cli: HttpClient) -> bool {
    let params = rpc_params![OP_SEPOLIA_L2_HEAD, SEPOLIA_L1_HEAD];
    let witness_result: WitnessResult = cli.request("getWitness", params).await.unwrap();

    match witness_result.status {
        RequestResult::Completed => {
            let prover = ProverClient::new();
            let mut sp1_stdin = SP1Stdin::new();
            sp1_stdin.buffer = witness_result.get_witness_buf();

            let (_, report) = prover.execute(FAULT_PROOF_ELF, sp1_stdin).run().unwrap();
            println!("report: {:?}", report);
        }
        _ => {
            println!("status: {:?}", witness_result.status);
        }
    }

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
        test_spec(http_client.clone()).await;
    }
    if args.request {
        test_request(http_client.clone()).await;
    }
    if args.get {
        test_get(http_client).await;
    }
}
