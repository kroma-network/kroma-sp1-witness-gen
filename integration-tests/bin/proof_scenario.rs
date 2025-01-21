use alloy_primitives::B256;
use anyhow::Result;
use clap::Parser;
use integration_tests::{load_witness, save_proof, IntegratedClient, Method, ProverRequest};
use std::{thread::sleep, time::Duration};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value_t = B256::default())]
    l2_hash: B256,

    #[clap(short, long, default_value_t = B256::default())]
    l1_head_hash: B256,

    #[clap(short, long, default_value = "./witness.json")]
    witness_data: String,

    #[clap(short, long, default_value = "./proof.json")]
    proof_data: String,

    #[clap(short, long, default_value = "scenario")]
    method: Method,
}

impl Args {
    fn assert_if_empty_hashes(&self) {
        assert!(
            self.l2_hash != B256::default() && self.l1_head_hash != B256::default(),
            "This method requires both l2_hash and l1_head_hash"
        );
    }
}

async fn scenario(
    client: &IntegratedClient,
    l2_hash: B256,
    l1_head_hash: B256,
    witness_data: &String,
    proof_data: &String,
) -> Result<()> {
    client.prover_spec().await;

    let witness_result = load_witness(witness_data).expect("failed to load witness");

    // The response should be `Processing`.
    let request_result =
        client.request_prove(l2_hash, l1_head_hash, &witness_result).await.unwrap();
    assert_eq!(
        request_result,
        ProverRequest::Processing,
        "Consider removing the witness data and trying again!"
    );

    // The same response is returned for the same request.
    let request_result =
        client.request_prove(l2_hash, l1_head_hash, &witness_result).await.unwrap();
    assert_eq!(request_result, ProverRequest::Processing);

    let proof_result = loop {
        let proof_result = client.get_proof(l2_hash, l1_head_hash).await;
        if proof_result.request_status == ProverRequest::Completed {
            break proof_result;
        }
        if let ProverRequest::Failed = proof_result.request_status {
            panic!("Failed to get proof");
        }
        sleep(Duration::from_secs(20));
    };

    save_proof(proof_data, &proof_result)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = IntegratedClient::default();

    match args.method {
        Method::Spec => {
            let spec = client.prover_spec().await;
            println!("Prover-Proxy spec: {:#?}", spec);
        }
        Method::Request => {
            args.assert_if_empty_hashes();
            let witness_result = load_witness(&args.witness_data).expect("failed to load witness");
            let request_result = client
                .request_prove(args.l2_hash, args.l1_head_hash, &witness_result)
                .await
                .unwrap();
            println!("Request result: {:#?}", request_result);
        }
        Method::Get => {
            args.assert_if_empty_hashes();
            let proof_result = client.get_proof(args.l2_hash, args.l1_head_hash).await;
            save_proof(&args.proof_data, &proof_result).expect("failed to save witness");
            println!("Proof status: {:?}", proof_result.request_status);
        }
        Method::Scenario => {
            args.assert_if_empty_hashes();
            scenario(&client, args.l2_hash, args.l1_head_hash, &args.witness_data, &args.proof_data)
                .await?
        }
    }

    Ok(())
}
