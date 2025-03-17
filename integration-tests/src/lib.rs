mod client;

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
