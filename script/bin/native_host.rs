use alloy_primitives::{hex::FromHex, B256};
use anyhow::Result;
use kona_host::{init_tracing_subscriber, start_server_and_native_client, HostCli};
use serde_json::Value;
use std::{fs::File, io::BufReader};

fn json_to_b256(key: &str, json: &Value) -> B256 {
    B256::from_hex(json[key].as_str().map(|s| s.to_string()).unwrap_or_default()).unwrap()
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let file = File::open("host_cli_output.json")?;
    let reader = BufReader::new(file);
    let host_cli_json: Value = serde_json::from_reader(reader)?;

    let host_cli = HostCli {
        v: host_cli_json["v"].as_u64().unwrap_or(0) as u8,
        l1_head: json_to_b256("l1_head", &host_cli_json),
        agreed_l2_head_hash: json_to_b256("agreed_l2_head_hash", &host_cli_json),
        agreed_l2_output_root: json_to_b256("agreed_l2_output_root", &host_cli_json),
        claimed_l2_output_root: json_to_b256("claimed_l2_output_root", &host_cli_json),
        claimed_l2_block_number: host_cli_json["claimed_l2_block_number"].as_u64().unwrap_or(0),
        l2_node_address: host_cli_json["l2_node_address"].as_str().map(|s| s.to_string()),
        l1_node_address: host_cli_json["l1_node_address"].as_str().map(|s| s.to_string()),
        l1_beacon_address: host_cli_json["l1_beacon_address"].as_str().map(|s| s.to_string()),
        data_dir: host_cli_json["data_dir"].as_str().map(|s| s.to_string().into()),
        exec: host_cli_json["exec"].as_str().map(|s| s.to_string()),
        server: host_cli_json["server"].as_bool().unwrap_or(false),
        l2_chain_id: host_cli_json["l2_chain_id"].as_u64(),
        rollup_config_path: host_cli_json["rollup_config_path"]
            .as_str()
            .map(|s| s.to_string().into()),
    };

    init_tracing_subscriber(host_cli.v)?;

    let res = start_server_and_native_client(host_cli).await;
    if res.is_err() {
        std::process::exit(1);
    }

    println!("Exiting host program.");
    std::process::exit(0);
}
