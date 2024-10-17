use clap::Parser;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_core::client::ClientT;
use jsonrpsee_core::rpc_params;
use kroma_prover::spec_impl::SpecResult;
use kroma_utils::utils::b256_from_str;
use kroma_witnessgen::get_witness_impl::WitnessResult;
use kroma_witnessgen::request_witness_impl::RequestResult;
use kroma_witnessgen::witness_db::WitnessDB;
use std::sync::Arc;
use std::time::Duration;

const CLIENT_TIMEOUT_SEC: u64 = 10800;
const DEFAULT_RPC_SERVER_ENDPOINT: &str = "http://127.0.0.1:3031";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    spec: bool,

    #[clap(short, long)]
    request: bool,
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
    let witness_db = Arc::new(WitnessDB::new(db_path.into()));
    let witness: Vec<Vec<u8>> = witness_db.get(&l2_head, &l1_head_hash).unwrap();

    WitnessResult::new_from_witness_buf(RequestResult::Completed, witness)
}

async fn test_request(cli: HttpClient) {
    // TODO: Change these from hard-coded values to values from the command line
    let l2_head = "0x86df565e6a6e3e266411e3718d5ceba49026606a00624e48c08448f8bf7bc82e";
    let l1_head = "0x42c0d60066fbd229758f8deaee337afc6cd0a75ddf120896258a4fd846aafbfd";

    let witness_result = get_witness_from_db(l2_head, l1_head);

    let params = rpc_params![l2_head, l1_head, &witness_result.witness];
    let result: RequestResult = cli.request("requestProve", params).await.unwrap();
    println!("request result: {:?}", result);
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
}
