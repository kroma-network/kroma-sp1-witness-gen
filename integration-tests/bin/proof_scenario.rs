use alloy_primitives::B256;
use anyhow::Result;
use clap::Parser;
use integration_tests::{ProverRequest, TestClient};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    fs::File,
    io::{BufReader, Write},
    thread::sleep,
    time::Duration,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    l2_hash: B256,

    #[clap(short, long)]
    l1_head_hash: B256,

    #[clap(short, long)]
    witness_data: String,

    #[clap(short, long)]
    proof_data: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProofFixture {
    program_key: String,
    public_values: String,
    proof: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = TestClient::default();
    client.prover_spec().await;

    let file = File::open(&args.witness_data)?;
    let reader = BufReader::new(file);
    let witness_result = from_reader(reader)?;

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
        if let ProverRequest::Failed = proof_result.request_status {
            panic!("Failed to get witness");
        }
        sleep(Duration::from_secs(20));
    };

    let proof_fixture = ProofFixture {
        program_key: proof_result.program_key,
        public_values: proof_result.public_values,
        proof: proof_result.proof,
    };
    let proof_json = serde_json::to_string_pretty(&proof_fixture)?;
    let mut file = File::create(args.proof_data)?;
    file.write_all(proof_json.as_bytes())?;
    tracing::info!("Proof was saved");

    Ok(())
}
