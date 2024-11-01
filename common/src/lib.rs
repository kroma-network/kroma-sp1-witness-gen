pub mod db;
pub mod deps_version;
pub mod task_info;
pub mod utils;
pub mod version;

use once_cell::sync::Lazy;
use sp1_sdk::{HashableKey, ProverClient};

pub const SINGLE_BLOCK_ELF: &[u8] = include_bytes!("../../program/elf/fault-proof-elf");
pub static PROGRAM_KEY: Lazy<String> = Lazy::new(|| {
    let prover = ProverClient::new();
    let (_, vkey) = prover.setup(SINGLE_BLOCK_ELF);
    vkey.bytes32()
});
