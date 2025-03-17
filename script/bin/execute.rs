use alloy_primitives::B256;
use anyhow::Result;
use clap::Parser;
use kroma_witnessgen::{
    types::{RequestResult, WitnessResult},
    utils::save_witness,
};
use op_succinct_host_utils::{get_proof_stdin, witnessgen::WitnessGenExecutor};
use script::{get_kroma_host_cli_by_l1_head_hash, get_l1_block_hash, init_fetcher, parse_u64};
use sp1_sdk::SP1Stdin;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// L2 block number for derivation.
    #[arg(long, value_parser = parse_u64)]
    l2_block: u64,

    /// L1 head hash (optional).
    #[cfg(feature = "kroma")]
    #[arg(long)]
    l1_head_hash: Option<B256>,

    /// L1 head hash (optional).
    #[cfg(feature = "kroma")]
    #[arg(long)]
    l1_head_number: Option<u64>,
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
            self.l1_head_hash = Some(get_l1_block_hash(self.l1_head_number.unwrap(), None));
            self.l1_head_hash.unwrap()
        }
    }
}

pub const FAULT_PROOF_ELF: &[u8] = include_bytes!("../../program/elf/fault-proof-elf");

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = Args::parse();
    let fetcher = init_fetcher();

    let host_cli =
        get_kroma_host_cli_by_l1_head_hash(args.l2_block, args.get_l1_head_hash(), Some(&fetcher))
            .await?;

    // Start the server and native client.
    let mut witnessgen_executor = WitnessGenExecutor::default();
    witnessgen_executor.spawn_witnessgen(&host_cli).await?;
    witnessgen_executor.flush().await?;

    // Get the stdin for the block.
    let sp1_stdin = {
        let sp1_stdin_v3_4 = get_proof_stdin(&host_cli)?;
        let mut sp1_stdin_v3_0 = SP1Stdin::default();
        sp1_stdin_v3_0.buffer = sp1_stdin_v3_4.buffer;
        sp1_stdin_v3_0
    };

    let wr = WitnessResult::new_from_witness_buf(RequestResult::Completed, sp1_stdin.buffer);
    save_witness(&"./witness.json".to_string(), &wr)?;
    println!("SP1 Stdin was saved");

    Ok(())
}
