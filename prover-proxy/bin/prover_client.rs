use clap::Parser;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_core::{client::ClientT, rpc_params};
use kroma_prover_proxy::types::{ProofResult, SpecResult};
use kroma_utils::utils::b256_from_str;
use kroma_witnessgen::{
    types::{RequestResult, WitnessResult},
    witness_db::WitnessDB,
};
use std::{sync::Arc, time::Duration};

const CLIENT_TIMEOUT_SEC: u64 = 10800;
const DEFAULT_RPC_SERVER_ENDPOINT: &str = "http://127.0.0.1:3031";

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

fn get_witness_from_db(l2_head: &str, l1_head_hash: &str) -> WitnessResult {
    let l2_head = b256_from_str(l2_head).unwrap();
    let l1_head_hash = b256_from_str(l1_head_hash).unwrap();

    let db_path = "data/witness_store";
    let witness_db = Arc::new(WitnessDB::new(db_path));
    let witness: Vec<Vec<u8>> = witness_db.get(&l2_head, &l1_head_hash).unwrap();

    WitnessResult::new_from_witness_buf(RequestResult::Completed, witness)
}

async fn test_request(cli: HttpClient) {
    // TODO: Change these from hard-coded values to values from the command line
    let l2_head = "0x564ec49e7c9ea0fe167c0ed3796b9c4ba884e059865c525f198306e72febedf8";
    let l1_head = "0xe22242e0d09d8236658b67553f41b183de2ce0dbbef94daf50dba64610f509a4";

    let witness_result = get_witness_from_db(l2_head, l1_head);

    let params = rpc_params![l2_head, l1_head, &witness_result.witness];
    let result: RequestResult = cli.request("requestProve", params).await.unwrap();
    println!("request result: {:?}", result);
}

async fn test_get(cli: HttpClient) {
    // TODO: Change these from hard-coded values to values from the command line
    let l2_head = "0x564ec49e7c9ea0fe167c0ed3796b9c4ba884e059865c525f198306e72febedf8";
    let l1_head = "0xe22242e0d09d8236658b67553f41b183de2ce0dbbef94daf50dba64610f509a4";

    let params = rpc_params![l2_head, l1_head];
    let result: ProofResult = cli.request("getProof", params).await.unwrap();
    println!("proof result: {:?}", result);
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
        test_get(http_client.clone()).await;
    }
}
