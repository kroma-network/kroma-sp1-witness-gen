use std::env;

use alloy_primitives::B256;
use anyhow::Result;
use op_succinct_host_utils::{
    fetcher::{CacheMode, OPSuccinctDataFetcher, RPCMode},
    get_proof_stdin,
};
use serde::{Deserialize, Serialize};
use sp1_sdk::{block_on, SP1Stdin};

use crate::utils::b256_from_str;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RequestResult {
    None,
    Requested,
    Processing,
    Completed,
}

pub fn check_request(l2_hash: &String, l1_head_hash: &String) -> Result<(B256, B256)> {
    let l2_hash = b256_from_str(&l2_hash).unwrap();
    let l1_head_hash = b256_from_str(&l1_head_hash).unwrap();

    // TODO: check more detail.

    Ok((l2_hash, l1_head_hash))
}

pub fn generate_witness_impl(l2_hash: B256, l1_head_hash: B256) -> Result<SP1Stdin> {
    let data_fetcher = OPSuccinctDataFetcher {
        l2_rpc: env::var("L2_RPC").expect("L2_RPC is not set."),
        ..Default::default()
    };

    let l2_header = block_on(async { data_fetcher.get_header_by_hash(RPCMode::L2, l2_hash).await });
    let l2_block_number = l2_header.unwrap().number;
    // TODO: return error if `l2_header` is `None`.
    // TODO: return error if `l1_head` is `None` after fetching the header.

    let cache_mode = CacheMode::KeepCache;
    // TODO: change `cache_mode` to `CacheMode::KeepCache` after finishing the implementation.
    // let cache_mode = CacheMode::DeleteCache;

    let host_cli = block_on(async {
        data_fetcher
            .get_host_cli_args(
                l2_block_number - 1,
                l2_block_number,
                op_succinct_host_utils::ProgramType::Single,
                cache_mode,
            )
            .await
    });
    let mut host_cli = host_cli.unwrap();
    host_cli.l1_head = l1_head_hash;

    // TODO: activate this code after finishing the implementation.
    // // Start the server and native client.
    // let mut witnessgen_executor = WitnessGenExecutor::default();
    // witnessgen_executor.spawn_witnessgen(&host_cli).await?;
    // witnessgen_executor.flush().await?;

    Ok(get_proof_stdin(&host_cli).unwrap())
}
