mod client;

use anyhow::Result;
use std::{fs::File, io::Write};

pub use client::IntegratedClient;
pub use kroma_prover_proxy::types::{
    ProofResult, RequestResult as ProverRequest, SpecResult as ProverSpec,
};
pub use kroma_witnessgen::types::{
    RequestResult as WitnessRequest, SpecResult as WitnessSpec, WitnessResult,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Method {
    Spec,
    Request,
    Get,
    Scenario,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProofFixture {
    pub program_key: String,
    pub public_values: String,
    pub proof: String,
}

impl ProofFixture {
    fn from_proof_result(proof_result: &ProofResult) -> Self {
        Self {
            program_key: proof_result.program_key.clone(),
            public_values: proof_result.public_values.clone(),
            proof: proof_result.proof.clone(),
        }
    }
}

pub fn load_witness(witness_data: &String) -> Result<WitnessResult> {
    let file = File::open(witness_data)?;
    let reader = std::io::BufReader::new(file);
    let witness_result = serde_json::from_reader(reader)?;

    Ok(witness_result)
}

pub fn save_witness(witness_data: &String, witness_result: &WitnessResult) -> Result<()> {
    let witness_json = serde_json::to_string_pretty(&witness_result)?;
    let mut file = File::create(witness_data)?;
    file.write_all(witness_json.as_bytes())?;
    println!("Witness was saved");
    Ok(())
}

pub fn save_proof(proof_data: &String, proof_result: &ProofResult) -> Result<()> {
    let proof_fixture = ProofFixture::from_proof_result(proof_result);

    let proof_json = serde_json::to_string_pretty(&proof_fixture)?;
    let mut file = File::create(proof_data)?;
    file.write_all(proof_json.as_bytes())?;
    println!("Proof was saved");

    Ok(())
}
