use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::env;

use alloy_consensus::Header;
use alloy_primitives::Bytes;
use alloy_rlp::Decodable;
use op_succinct_host_utils::fetcher::{OPSuccinctDataFetcher, RPCMode};

pub async fn assert_if_invalid_rpcs() -> Result<()> {
    dotenv::dotenv().ok();
    let fetcher = OPSuccinctDataFetcher::default();

    // Check if L1 Geth is alive.
    let _: Value = fetcher
        .fetch_rpc_data(RPCMode::L1, "net_version", vec![])
        .await
        .expect("L1 Geth is not alive");

    // Check if L2 Geth is alive.
    let _: Value = fetcher
        .fetch_rpc_data(RPCMode::L2, "net_version", vec![])
        .await
        .expect("L2 Geth is not alive");

    // Check if L1 Geth is in debug mode
    let _: Value = fetcher
        .fetch_rpc_data(RPCMode::L1, "debug_getRawHeader", vec!["latest".into()])
        .await
        .expect("L1 Geth is not in debug mode");

    // Check L2 Geth is in debug mode
    let raw_header: Bytes = fetcher
        .fetch_rpc_data(RPCMode::L2, "debug_getRawHeader", vec!["latest".into()])
        .await
        .expect("L2 Geth is not in debug mode");

    // Check if L2 Node is alive.
    let header = Header::decode(&mut raw_header.as_ref())
        .map_err(|e| anyhow!("Failed to decode header: {e}"))?;
    let l2_latest_number = header.number;
    let _: Value = fetcher
        .fetch_rpc_data(
            RPCMode::L2Node,
            "optimism_outputAtBlock",
            vec![format!("{:#x}", l2_latest_number).into()],
        )
        .await
        .expect("L2 Node is not alive");

    // TODO(Ethan): Check if L1 beacon is alive.

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;

    use super::assert_if_invalid_rpcs;

    #[test]
    fn test_rpc_valid() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            assert_if_invalid_rpcs().await.unwrap();
        });
    }
}
