use std::{thread::sleep, time::Duration};

use alloy_primitives::{b256, B256};
use anyhow::Result;
use clap::Parser;
use integration_tests::{ProverRequest, TestClient, WitnessRequest};
use kroma_witnessgen::errors::ErrorCode as WitnessErrorCode;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    l2_hash: B256,

    #[clap(short, long)]
    l1_head_hash: B256,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = TestClient::new();

    let _ = client.witnessgen_spec().await;
    let _ = client.prover_spec().await;

    ////////////////////////////////////////////////////////////////
    //                   WITNESS GENERATOR TEST                   //
    ////////////////////////////////////////////////////////////////

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

    ////////////////////////////////////////////////////////////////
    //                        PROVER TEST                         //
    ////////////////////////////////////////////////////////////////

    // The response should be `Processing`.
    let request_result =
        client.request_prove(args.l2_hash, args.l1_head_hash, &witness_result).await.unwrap();
    assert_eq!(request_result, ProverRequest::Processing);

    // The same response is returned for the same request.
    let request_result =
        client.request_prove(args.l2_hash, args.l1_head_hash, &witness_result).await.unwrap();
    assert_eq!(request_result, ProverRequest::Processing);

    let proof_result = loop {
        let proof_result = client.get_proof(args.l2_hash, args.l1_head_hash).await;
        if proof_result.request_status == ProverRequest::Completed {
            break proof_result;
        }
        if let ProverRequest::Failed(_) = proof_result.request_status {
            panic!("Failed to get witness");
        }
        sleep(Duration::from_secs(20));
    };

    println!("Proof: {:?}", proof_result.proof);

    // TODO: send proof to the verifier contract

    Ok(())
}
