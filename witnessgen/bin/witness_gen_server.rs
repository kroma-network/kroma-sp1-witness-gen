use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use kroma_witnessgen::{
    checker::{assert_if_invalid_rpcs, check_rollup_config_before_mpt_time},
    interface::{DEFAULT_WITNESSGEN_RPC_ENDPOINT, DEFAULT_WITNESS_STORE_PATH},
    witness_db::WitnessDB,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "endpoint", default_value = DEFAULT_WITNESSGEN_RPC_ENDPOINT)]
    endpoint: String,

    #[clap(short, long = "data", default_value = DEFAULT_WITNESS_STORE_PATH)]
    data_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::Subscriber::builder().init();

    let args = Args::parse();

    // NOTE(Ethan)Before the `MPT time`, the rollup configuration must be read from a JSON file instead of via RPC.
    // If the launch time is before the MPT time, set `ROLLUP_CONFIG_FROM_FILE` to `true`.
    check_rollup_config_before_mpt_time().await?;

    // Check if All the RPCs are valid.
    assert_if_invalid_rpcs().await?;
    tracing::info!("All validation for safe launching has been passed.");

    let witness_db = Arc::new(WitnessDB::new(&args.data_path));
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    kroma_witnessgen::executor::run(witness_db.clone(), rx).await;

    kroma_witnessgen::interface::run(witness_db.clone(), tx, args.endpoint).await;

    Ok(())
}
