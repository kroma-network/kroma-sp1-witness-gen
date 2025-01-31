mod client;

use anyhow::Result;
use std::{fs::File, io::Write};

pub use client::TestClient;
pub use kroma_witnessgen::types::{
    RequestResult as WitnessRequest, SpecResult as WitnessSpec, WitnessResult,
};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Method {
    Spec,
    Request,
    Get,
    Scenario,
}

pub fn save_witness(witness_data: &String, witness_result: &WitnessResult) -> Result<()> {
    let witness_json = serde_json::to_string_pretty(&witness_result)?;
    let mut file = File::create(witness_data)?;
    file.write_all(witness_json.as_bytes())?;
    println!("Witness was saved");
    Ok(())
}
