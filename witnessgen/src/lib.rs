pub mod checker;
pub mod errors;
pub mod executor;
pub mod interface;
pub mod types;
pub mod utils;
pub mod version;
pub mod witness_db;

use once_cell::sync::Lazy;
use sp1_sdk::{HashableKey, ProverClient};

pub const FAULT_PROOF_ELF: &[u8] = include_bytes!("../../program/elf/fault-proof-elf");
pub static VERIFICATION_KEY_HASH: Lazy<String> = Lazy::new(|| {
    let prover = ProverClient::new();
    let (_, vkey) = prover.setup(FAULT_PROOF_ELF);
    vkey.bytes32()
});
