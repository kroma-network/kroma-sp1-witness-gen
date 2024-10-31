use alloy_primitives::{b256, B256};
use anyhow::Result;
use clap::Parser;
use integration_tests::{TestClient, WitnessRequest};
use kroma_witnessgen::errors::ErrorCode as WitnessErrorCode;
use std::{fs::File, io::Write, thread::sleep, time::Duration};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    l2_hash: B256,

    #[clap(short, long)]
    l1_head_hash: B256,

    #[clap(short, long)]
    witness_data: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = TestClient::new();

    let _ = client.witnessgen_spec().await;

    // The response should be `Processing`.
    let request_result = client.request_witness(args.l2_hash, args.l1_head_hash).await.unwrap();
    assert_eq!(request_result, WitnessRequest::Processing);

    // The same response is returned for the same request.
    let request_result = client.request_witness(args.l2_hash, args.l1_head_hash).await.unwrap();
    assert_eq!(request_result, WitnessRequest::Processing);

    // If a different request arrives while another request is being processed, the WitnessGenerator returns an error.
    let tweaked_l2_hash = b256!("0000000000000000000000000000000000000000000000000000000000000001");
    let request_result = client.request_witness(tweaked_l2_hash, args.l1_head_hash).await;
    assert!(matches!(request_result.err().unwrap().code, WitnessErrorCode::AlreadyInProgress));

    let witness_result = loop {
        let witness_result = client.get_witness(args.l2_hash, args.l1_head_hash).await;
        if witness_result.status == WitnessRequest::Completed {
            break witness_result;
        }
        if witness_result.status == WitnessRequest::Failed {
            panic!("Failed to get witness");
        }
        sleep(Duration::from_secs(20));
    };

    let report =
        TestClient::execute_witness(&witness_result).await.expect("Failed to execute witness");
    tracing::info!("Witness execution succeeded: {:?}", report.total_instruction_count());

    let witness_json = serde_json::to_string_pretty(&witness_result)?;
    let mut file = File::create(args.witness_data)?;
    file.write_all(witness_json.as_bytes())?;
    tracing::info!("Witness was saved");

    Ok(())
}
