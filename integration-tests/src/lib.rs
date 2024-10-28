mod client;

pub use client::TestClient;
pub use kroma_prover::types::{
    ProofResult, RequestResult as ProverRequest, SpecResult as ProverSpec,
};
pub use kroma_witnessgen::types::{
    RequestResult as WitnessRequest, SpecResult as WitnessSpec, WitnessResult,
};
