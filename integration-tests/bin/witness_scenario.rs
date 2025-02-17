use alloy_primitives::{b256, B256};
use anyhow::Result;
use clap::Parser;
use integration_tests::{save_witness, Method, TestClient, WitnessRequest};
use kroma_witnessgen::errors::ErrorCode as WitnessErrorCode;
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
    client: &TestClient,
    l2_hash: B256,
    l1_head_hash: B256,
    witness_data: &String,
) -> Result<()> {
    client.witnessgen_spec().await;

    // The response should be `Processing`.
    let request_result = client.request_witness(l2_hash, l1_head_hash).await.unwrap();
    assert_eq!(
        request_result,
        WitnessRequest::Processing,
        "Consider removing the witness data and trying again!"
    );

    // The same response is returned for the same request.
    let request_result = client.request_witness(l2_hash, l1_head_hash).await.unwrap();
    assert_eq!(request_result, WitnessRequest::Processing);

    // If a different request arrives while another request is being processed, the WitnessGenerator returns an error.
    let tweaked_l2_hash = b256!("0000000000000000000000000000000000000000000000000000000000000001");
    let request_result = client.request_witness(tweaked_l2_hash, l1_head_hash).await;
    assert!(matches!(request_result.err().unwrap().code, WitnessErrorCode::AlreadyInProgress));

    let witness_result = loop {
        let witness_result = client.get_witness(l2_hash, l1_head_hash).await;
        if witness_result.status == WitnessRequest::Completed {
            break witness_result;
        }
        if witness_result.status == WitnessRequest::Failed {
            panic!("Failed to get witness");
        }
        sleep(Duration::from_secs(20));
    };

    save_witness(witness_data, &witness_result)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = TestClient::default();

    match args.method {
        Method::Spec => {
            let witnessgen_spec = client.witnessgen_spec().await;
            println!("WitnessGen spec: {:#?}", witnessgen_spec);
        }
        Method::Request => {
            args.assert_if_empty_hashes();
            let request_result =
                client.request_witness(args.l2_hash, args.l1_head_hash).await.unwrap();
            println!("Request result: {:#?}", request_result);
        }
        Method::Get => {
            args.assert_if_empty_hashes();
            let witness_result = client.get_witness(args.l2_hash, args.l1_head_hash).await;
            save_witness(&args.witness_data, &witness_result).expect("failed to save witness");
            println!("Witness status: {:?}", witness_result.status);
        }
        Method::Scenario => {
            args.assert_if_empty_hashes();
            scenario(&client, args.l2_hash, args.l1_head_hash, &args.witness_data).await?
        }
    }

    Ok(())
}
