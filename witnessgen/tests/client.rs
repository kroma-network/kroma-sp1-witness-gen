use alloy_primitives::B256;
use anyhow::Result;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_core::{client::ClientT, rpc_params};

use kroma_witnessgen::{
    errors::WitnessGenError,
    types::{RequestResult as WitnessRequest, SpecResult as WitnessSpec, WitnessResult},
    FAULT_PROOF_ELF,
};
use sp1_sdk::{ExecutionReport, ProverClient, SP1Stdin};
use std::time::Duration;

const CLIENT_TIMEOUT_SEC: u64 = 10800;
const DEFAULT_WITNESSGEN_RPC_ENDPOINT: &str = "http://0.0.0.0:3030";

pub struct TestClient {
    witnessgen_client: HttpClient,
}

impl TestClient {
    #[allow(dead_code)]
    pub fn new(witnessgen_url: &str) -> Self {
        let witnessgen_client = HttpClientBuilder::default()
            .max_request_body_size(300 * 1024 * 1024)
            .request_timeout(Duration::from_secs(CLIENT_TIMEOUT_SEC))
            .build(witnessgen_url)
            .unwrap();

        Self { witnessgen_client }
    }
}

impl Default for TestClient {
    fn default() -> Self {
        let witnessgen_client = HttpClientBuilder::default()
            .max_request_body_size(300 * 1024 * 1024)
            .request_timeout(Duration::from_secs(CLIENT_TIMEOUT_SEC))
            .build(DEFAULT_WITNESSGEN_RPC_ENDPOINT)
            .unwrap();

        Self { witnessgen_client }
    }
}

impl TestClient {
    pub async fn witnessgen_spec(&self) -> Result<WitnessSpec> {
        let params = rpc_params![];
        self.witnessgen_client
            .request::<WitnessSpec, _>("spec", params)
            .await
            .map_err(|e| anyhow::anyhow!("the server is not ready: {:?}", e))
    }

    pub async fn request_witness(
        &self,
        l2_hash: B256,
        l1_head_hash: B256,
    ) -> Result<WitnessRequest, WitnessGenError> {
        let params = rpc_params![l2_hash, l1_head_hash];
        match self.witnessgen_client.request("requestWitness", params).await {
            Ok(result) => Ok(result),
            Err(e) if e.to_string().contains("Another request is in progress") => {
                Err(WitnessGenError::already_in_progress(e.to_string()))
            }
            Err(e) if e.to_string().contains("Invalid parameters") => {
                Err(WitnessGenError::invalid_input_hash("Invalid parameters".to_string()))
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[allow(dead_code)]
    pub async fn execute_witness(witness_result: &WitnessResult) -> Result<ExecutionReport> {
        let prover = ProverClient::new();
        let mut sp1_stdin = SP1Stdin::new();
        sp1_stdin.buffer = witness_result.get_witness_buf();

        let (_, report) = prover.execute(FAULT_PROOF_ELF, sp1_stdin).run()?;
        Ok(report)
    }

    pub async fn get_witness(&self, l2_hash: B256, l1_head_hash: B256) -> WitnessResult {
        let params = rpc_params![l2_hash, l1_head_hash];
        self.witnessgen_client.request("getWitness", params).await.unwrap()
    }
}
