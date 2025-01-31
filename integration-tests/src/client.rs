use alloy_primitives::B256;
use anyhow::Result;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_core::{client::ClientT, rpc_params};

use kroma_prover_proxy::{
    errors::ProverError,
    types::{ProofResult, RequestResult as ProverRequest, SpecResult as ProverSpec},
};
use kroma_witnessgen::{
    errors::WitnessGenError,
    types::{RequestResult as WitnessRequest, SpecResult as WitnessSpec, WitnessResult},
    FAULT_PROOF_ELF,
};
use sp1_sdk::{ExecutionReport, ProverClient, SP1Stdin};
use std::time::Duration;

const CLIENT_TIMEOUT_SEC: u64 = 10800;
const DEFAULT_WITNESSGEN_RPC_ENDPOINT: &str = "http://0.0.0.0:3030";
const DEFAULT_PROVER_RPC_ENDPOINT: &str = "http://0.0.0.0:3031";

pub struct IntegratedClient {
    witnessgen_client: HttpClient,
    prover_client: HttpClient,
}

impl IntegratedClient {
    pub fn new(witnessgen_url: &str, prover_proxy_url: &str) -> Self {
        let witnessgen_client = HttpClientBuilder::default()
            .max_request_body_size(300 * 1024 * 1024)
            .request_timeout(Duration::from_secs(CLIENT_TIMEOUT_SEC))
            .build(witnessgen_url)
            .unwrap();

        let prover_client = HttpClientBuilder::default()
            .max_request_body_size(300 * 1024 * 1024)
            .request_timeout(Duration::from_secs(CLIENT_TIMEOUT_SEC))
            .build(prover_proxy_url)
            .unwrap();

        Self { witnessgen_client, prover_client }
    }
}

impl Default for IntegratedClient {
    fn default() -> Self {
        let witnessgen_client = HttpClientBuilder::default()
            .max_request_body_size(300 * 1024 * 1024)
            .request_timeout(Duration::from_secs(CLIENT_TIMEOUT_SEC))
            .build(DEFAULT_WITNESSGEN_RPC_ENDPOINT)
            .unwrap();

        let prover_client = HttpClientBuilder::default()
            .max_request_body_size(300 * 1024 * 1024)
            .request_timeout(Duration::from_secs(CLIENT_TIMEOUT_SEC))
            .build(DEFAULT_PROVER_RPC_ENDPOINT)
            .unwrap();

        Self { witnessgen_client, prover_client }
    }
}

impl IntegratedClient {
    pub async fn witnessgen_spec(&self) -> WitnessSpec {
        let params = rpc_params![];
        self.witnessgen_client.request("spec", params).await.unwrap()
    }

    pub async fn prover_spec(&self) -> ProverSpec {
        let params = rpc_params![];
        self.prover_client.request("spec", params).await.unwrap()
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

    pub async fn execute_witness(witness_result: &WitnessResult) -> Result<ExecutionReport> {
        let prover = ProverClient::new();
        let mut sp1_stdin = SP1Stdin::new();
        sp1_stdin.buffer = witness_result.get_witness_buf();

        let (_, report) = prover.execute(FAULT_PROOF_ELF, sp1_stdin).run()?;
        Ok(report)
    }

    pub async fn request_prove(
        &self,
        l2_hash: B256,
        l1_head_hash: B256,
        witness_result: &WitnessResult,
    ) -> Result<ProverRequest, ProverError> {
        let params = rpc_params![l2_hash, l1_head_hash, &witness_result.witness];
        match self.prover_client.request("requestProve", params).await {
            Ok(result) => Ok(result),
            Err(e) if e.to_string().contains("Invalid parameters") => {
                Err(ProverError::invalid_input_hash("Invalid parameters".to_string()))
            }
            Err(e) if e.to_string().contains("SP1 NETWORK ERROR") => {
                // TODO: correct error message for `sp1_network_error`
                Err(ProverError::sp1_network_error(e.to_string()))
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    pub async fn get_witness(&self, l2_hash: B256, l1_head_hash: B256) -> WitnessResult {
        let params = rpc_params![l2_hash, l1_head_hash];
        self.witnessgen_client.request("getWitness", params).await.unwrap()
    }

    pub async fn get_proof(&self, l2_hash: B256, l1_head_hash: B256) -> ProofResult {
        let params = rpc_params![l2_hash, l1_head_hash];
        self.prover_client.request("getProof", params).await.unwrap()
    }
}
