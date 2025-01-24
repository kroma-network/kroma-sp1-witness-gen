pub mod checker;
pub mod db;
pub mod task_info;
pub mod utils;
pub mod version;

use once_cell::sync::Lazy;
use sp1_sdk::{CpuProver, HashableKey, Prover};

pub const FAULT_PROOF_ELF: &[u8] = include_bytes!("../../program/elf/fault-proof-elf");
pub static PROGRAM_KEY: Lazy<String> = Lazy::new(|| {
    let prover = CpuProver::new();
    let (_, vkey) = prover.setup(FAULT_PROOF_ELF);
    vkey.bytes32()
});
