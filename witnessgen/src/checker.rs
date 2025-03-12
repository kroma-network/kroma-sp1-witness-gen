use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::env;

use alloy_consensus::Header;
use alloy_primitives::Bytes;
use alloy_rlp::Decodable;
use op_succinct_host_utils::fetcher::{OPSuccinctDataFetcher, RPCMode};

pub async fn fetch_l2_rpc_data(method: &str, params: Vec<Value>) -> Result<Value> {
    let l2_rpc_url = env::var("L2_RPC").expect("L2_RPC is not set");
    let client = reqwest::Client::new();
    let response = client
        .post(l2_rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    if let Some(error) = response.get("error") {
        let error_message = error["message"].as_str().unwrap_or("Unknown error");
        return Err(anyhow::anyhow!("RPC error: {}", error_message));
    }

    serde_json::from_value(response["result"].clone()).map_err(Into::into)
}

pub async fn check_rollup_config_before_mpt_time() -> Result<()> {
    dotenv::dotenv().ok();

    let chain_config = fetch_l2_rpc_data("debug_chainConfig", vec![]).await?;
    if chain_config.get("optimism").is_none() {
        env::set_var("ROLLUP_CONFIG_FROM_FILE", "true");
        println!("It should be before MPT time. `ROLLUP_CONFIG_FROM_FILE` is set as true.");
    }
    let _ = OPSuccinctDataFetcher::new_with_rollup_config().await.unwrap();

    Ok(())
}

pub async fn assert_if_invalid_rpcs() -> Result<()> {
    dotenv::dotenv().ok();
    let fetcher = OPSuccinctDataFetcher::new_with_rollup_config().await.unwrap();

    // Check if L1 Geth is alive.
    let _: Value = fetcher
        .fetch_rpc_data_with_mode(RPCMode::L1, "net_version", vec![])
        .await
        .expect("L1 Geth is not alive");

    // Check if L2 Geth is alive.
    let _: Value = fetcher
        .fetch_rpc_data_with_mode(RPCMode::L2, "net_version", vec![])
        .await
        .expect("L2 Geth is not alive");

    // Check if L1 Geth is in debug mode
    let _: Value = fetcher
        .fetch_rpc_data_with_mode(RPCMode::L1, "debug_getRawHeader", vec!["latest".into()])
        .await
        .expect("L1 Geth is not in debug mode");

    // Check L2 Geth is in debug mode
    let raw_header: Bytes = fetcher
        .fetch_rpc_data_with_mode(RPCMode::L2, "debug_getRawHeader", vec!["latest".into()])
        .await
        .expect("L2 Geth is not in debug mode");

    // Check if L2 Node is alive.
    let header = Header::decode(&mut raw_header.as_ref())
        .map_err(|e| anyhow!("Failed to decode header: {e}"))?;
    let latest = format!("0x{:x}", header.number - 1);
    let _: Value = fetcher
        .fetch_rpc_data_with_mode(RPCMode::L2Node, "optimism_outputAtBlock", vec![latest.into()])
        .await
        .expect("L2 Node is not alive");

    // TODO(Ethan): Check if L1 beacon is alive.

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use tokio::runtime::Runtime;

    use super::{assert_if_invalid_rpcs, check_rollup_config_before_mpt_time};

    #[test]
    fn test_rpc_valid() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            assert_if_invalid_rpcs().await.unwrap();
        });
    }

    #[test]
    fn test_rollup_config() {
        let _ = Command::new("cp")
            .args(&["-r", "../configs", "."])
            .output()
            .expect("Failed to copy .env file");

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            check_rollup_config_before_mpt_time().await.unwrap();
        });

        let _ = Command::new("rm")
            .args(&["-rf", "./configs"])
            .output()
            .expect("Failed to copy .env file");
    }
}
