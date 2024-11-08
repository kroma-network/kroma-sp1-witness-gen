#![allow(unused_mut)]
mod utils;

use anyhow::Result;
use cfg_if::cfg_if;
use clap::{Parser, ValueEnum};
use op_succinct_host_utils::{
    fetcher::{CacheMode, OPSuccinctDataFetcher},
    get_proof_stdin,
    witnessgen::WitnessGenExecutor,
    ProgramType,
};
use sp1_sdk::{utils as sdk_utils, ProverClient};
use std::time::Instant;
use utils::{get_l1_origin_of, get_output_at};
cfg_if! {
    if #[cfg(feature = "kroma")] {
        use alloy_primitives::B256;
    } else {
        use op_succinct_client_utils::BootInfoWithBytesConfig;
    }
}

pub const FAULT_PROOF_ELF: &[u8] = include_bytes!("../../program/elf/fault-proof-elf");

#[derive(ValueEnum, Debug, Clone, PartialEq)]
#[clap(rename_all = "kebab-case")]
enum Method {
    /// Preview an argument to execute.
    Preview,
    /// Native-execute the guest program.
    Execute,
    /// Generate a proof.
    Prove,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// L2 block number for derivation.
    #[arg(short, long)]
    l2_block: u64,

    /// L1 head hash (optional).
    #[cfg(feature = "kroma")]
    #[arg(short, long)]
    l1_head_hash: Option<B256>,

    /// L1 head hash (optional).
    #[cfg(feature = "kroma")]
    #[arg(short, long)]
    l1_head_number: Option<u64>,

    /// Skip running native execution.
    #[arg(short, long)]
    use_cache: bool,

    /// Generate proof.
    #[arg(short, long, default_value = "execute")]
    method: Method,
}

#[cfg(feature = "kroma")]
impl Args {
    fn get_l1_head_hash(&mut self) -> B256 {
        if self.l1_head_hash.is_none() && self.l1_head_number.is_none() {
            panic!("Missing L1 Head Hash or Number");
        }
        if self.l1_head_hash.is_some() {
            self.l1_head_hash.unwrap()
        } else {
            self.l1_head_hash = Some(utils::get_l1_block_hash(self.l1_head_number.unwrap()));
            self.l1_head_hash.unwrap()
        }
    }
}

/// Execute the OP Succinct program for a single block.
#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let mut args = Args::parse();
    sdk_utils::setup_logger();

    let data_fetcher = OPSuccinctDataFetcher::default();

    if args.method == Method::Preview {
        let output_root = get_output_at(&data_fetcher, args.l2_block);
        let parent_output_root = get_output_at(&data_fetcher, args.l2_block - 1);
        let (l1_origin_hash, l1_origin_number) = get_l1_origin_of(&data_fetcher, args.l2_block);

        println!("- output_root: {:?}", output_root);
        println!("- parent_output_root: {:?}", parent_output_root);
        println!("- l1_origin_hash: {:?}", l1_origin_hash);
        println!("- l1_origin_number: {:?}", l1_origin_number);
        println!("- l1_head_number: {:?}", l1_origin_number + 300);
        return Ok(());
    }

    let l2_safe_head = args.l2_block - 1;

    let cache_mode = if args.use_cache { CacheMode::KeepCache } else { CacheMode::DeleteCache };

    let mut host_cli = data_fetcher
        .get_host_cli_args(l2_safe_head, args.l2_block, ProgramType::Single, cache_mode)
        .await?;
    #[cfg(feature = "kroma")]
    {
        host_cli.l1_head = args.get_l1_head_hash();
        println!("L1 head hash has been changed");
    }

    // By default, re-run the native execution unless the user passes `--use-cache`.
    let start_time = Instant::now();
    if !args.use_cache {
        // Start the server and native client.
        let mut witnessgen_executor = WitnessGenExecutor::default();
        witnessgen_executor.spawn_witnessgen(&host_cli).await?;
        witnessgen_executor.flush().await?;
    }
    let witness_generation_time_sec = start_time.elapsed();

    // Get the stdin for the block.
    let sp1_stdin = get_proof_stdin(&host_cli)?;

    let l2_chain_id = data_fetcher.get_l2_chain_id().await?;
    let prover = ProverClient::new();

    match args.method {
        Method::Preview => {}
        Method::Execute => {
            let start_time = Instant::now();
            let (mut public_values, report) =
                prover.execute(FAULT_PROOF_ELF, sp1_stdin).run().unwrap();
            let execution_duration = start_time.elapsed();

            cfg_if! {
                if #[cfg(feature = "kroma")] {
                    let parent_output_root = public_values.read::<B256>();
                    let expected_parent_output_root = get_output_at(&data_fetcher, args.l2_block - 1);
                    assert_eq!(parent_output_root, expected_parent_output_root);

                    let output_root = public_values.read::<B256>();
                    let expected_output_root = get_output_at(&data_fetcher, args.l2_block);
                    assert_eq!(output_root, expected_output_root);

                    let l1_head = public_values.read::<B256>();
                    assert_eq!(l1_head, args.l1_head_hash.unwrap());
                } else {
                    let boot_info = public_values.read::<BootInfoWithBytesConfig>();
                    println!("{:#?}", boot_info);
                }
            }

            utils::report_execution(
                &data_fetcher,
                args.l2_block,
                args.l2_block,
                &report,
                witness_generation_time_sec,
                execution_duration,
                l2_chain_id,
                args.l2_block,
            )
            .await;
        }
        Method::Prove => {
            // If the prove flag is set, generate a proof.
            let (pk, _) = prover.setup(FAULT_PROOF_ELF);

            // Generate proofs in PLONK mode for on-chain verification.
            let proof = prover.prove(&pk, sp1_stdin).plonk().run().unwrap();

            // Create a proof directory for the chain ID if it doesn't exist.
            let proof_dir = format!("data/{}/proofs", data_fetcher.get_l2_chain_id().await?);
            if !std::path::Path::new(&proof_dir).exists() {
                std::fs::create_dir_all(&proof_dir)?;
            }
            proof
                .save(format!("{}/{}.bin", proof_dir, args.l2_block))
                .expect("Failed to save proof");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy_primitives::hex;
    use sp1_sdk::{HashableKey, ProverClient, SP1ProofWithPublicValues};

    use crate::FAULT_PROOF_ELF;

    #[test]
    fn print_proof() {
        // Setup the prover client.
        let client = ProverClient::new();

        // Setup the program.
        let (pk, vk) = client.setup(FAULT_PROOF_ELF);
        println!("{:?}", vk.bytes32().to_string());

        let proof = SP1ProofWithPublicValues::load(
            "/home/ubuntu/kroma-sp1-prover/data/11155420/proofs/18579127.bin",
        )
        .unwrap();

        println!("public values bytes: 0x{:?}", hex::encode(proof.public_values.as_slice()));
        println!("proof bytes: 0x{}", hex::encode(proof.bytes()));
    }
}
