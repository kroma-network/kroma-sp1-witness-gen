mod client;

use alloy_primitives::{b256, B256};
use anyhow::Result;
use client::TestClient;
use kroma_witnessgen::{
    errors::ErrorCode as WitnessErrorCode, types::RequestResult as WitnessRequest,
};
use std::{thread::sleep, time::Duration};

struct TestCtx {
    l2_hash: B256,
    l1_head_hash: B256,
}

impl TestCtx {
    fn new(l2_hash: B256, l1_head_hash: B256) -> Self {
        Self { l2_hash, l1_head_hash }
    }
}

async fn scenario(client: &TestClient, ctx: TestCtx) {
    client.witnessgen_spec().await.unwrap();

    // The response should be `Processing`.
    let request_result = client.request_witness(ctx.l2_hash, ctx.l1_head_hash).await.unwrap();
    assert_eq!(
        request_result,
        WitnessRequest::Processing,
        "Consider removing the witness data and trying again!"
    );

    // The same response is returned for the same request.
    let request_result = client.request_witness(ctx.l2_hash, ctx.l1_head_hash).await.unwrap();
    assert_eq!(request_result, WitnessRequest::Processing);

    // If a different request arrives while another request is being processed, the WitnessGenerator returns an error.
    let tweaked_l2_hash = b256!("0000000000000000000000000000000000000000000000000000000000000001");
    let request_result = client.request_witness(tweaked_l2_hash, ctx.l1_head_hash).await;
    assert!(matches!(request_result.err().unwrap().code, WitnessErrorCode::AlreadyInProgress));

    let _ = loop {
        let witness_result = client.get_witness(ctx.l2_hash, ctx.l1_head_hash).await;
        if witness_result.status == WitnessRequest::Completed {
            break witness_result;
        }
        if witness_result.status == WitnessRequest::Failed {
            panic!("Failed to get witness");
        }
        sleep(Duration::from_secs(3));
    };
}

#[tokio::test]
async fn test_online_scenario() -> Result<()> {
    dotenv::dotenv().ok();
    kona_host::init_tracing_subscriber(0)?;

    let metadata =
        cargo_metadata::MetadataCommand::new().exec().expect("Failed to get cargo metadata");
    let witnessgen_bin_path = metadata.target_directory.join("release/witness-gen-server");
    let mut child = tokio::process::Command::new(witnessgen_bin_path)
        .args(vec!["--data", "data/witness_store"])
        .spawn()?;

    let client = TestClient::default();
    while client.witnessgen_spec().await.is_err() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    println!("Witness generator server is ready.");

    let ctx = TestCtx::new(
        b256!("c620c1601621527b982fd8a9b781629edad908d7917c043e243f2277a48f561b"),
        b256!("b00118b43ea791285813f88bf1774508b6c495de9ec17f3f58cc810248d15d5d"),
    );
    scenario(&client, ctx).await;

    let mut sys = sysinfo::System::new();
    sys.refresh_all();
    if let Some(pid) = child.id() {
        for process in sys.processes().values() {
            if let Some(parent_pid) = process.parent() {
                if parent_pid.as_u32() == pid {
                    process.kill();
                }
            }
        }
        child.kill().await?
    }

    std::fs::remove_dir_all("data")?;

    Ok(())
}
