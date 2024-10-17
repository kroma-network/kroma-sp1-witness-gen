use anyhow::Result;
use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;
use kroma_utils::deps_version::SP1_SDK_VERSION;
use kroma_utils::utils::check_request;
use kroma_witnessgen::get_witness_impl::WitnessResult;
use sp1_sdk::network::client::NetworkClient;
use sp1_sdk::proto::network::{ProofMode, ProofStatus};
use sp1_sdk::{block_on, SP1ProofWithPublicValues, SP1Stdin};
use std::sync::Arc;
use std::time::Duration;

use crate::get_proof_impl::ProofResult;
use crate::proof_db::ProofDB;
use crate::request_prove_impl::RequestResult;
use crate::spec_impl::{spec_impl, SpecResult, SINGLE_BLOCK_ELF};

static DEFAULT_PROOF_STORE_PATH: &str = "data/proof_store";

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult>;

    #[rpc(name = "requestProve")]
    fn request_prove(
        &self,
        l2_hash: String,
        l1_head_hash: String,
        witness: String,
    ) -> JsonResult<RequestResult>;

    #[rpc(name = "getProof")]
    fn get_proof(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<ProofResult>;
}

#[derive(Clone)]
pub struct RpcImpl {
    proof_db: Arc<ProofDB>,
    client: Arc<NetworkClient>,
}

impl RpcImpl {
    pub fn new(store_path: &str, sp1_private_key: &str) -> Self {
        RpcImpl {
            proof_db: Arc::new(ProofDB::new(store_path.into())),
            client: Arc::new(NetworkClient::new(&sp1_private_key)),
        }
    }
}

impl Default for RpcImpl {
    fn default() -> Self {
        let sp1_private_key = std::env::var("SP1_PRIVATE_KEY")
            .expect("SP1_PRIVATE_KEY must be set for remote proving");
        Self::new(DEFAULT_PROOF_STORE_PATH, &sp1_private_key)
    }
}

impl RpcImpl {
    async fn request_prove_to_sp1(&self, witness: String) -> Result<String> {
        // Recover a SP1Stdin from the witness string.
        let mut sp1_stdin = SP1Stdin::new();
        sp1_stdin.buffer = WitnessResult::string_to_witness_buf(&witness);

        // Send a request to generate a proof to the sp1 network.
        let request_id = self
            .client
            .create_proof(SINGLE_BLOCK_ELF, &sp1_stdin, ProofMode::Plonk, SP1_SDK_VERSION)
            .await
            .unwrap();
        Ok(request_id)
    }

    async fn wait_proof(&self, request_id: String) -> Result<()> {
        let proof = loop {
            let (response, maybe_proof) = block_on(async {
                self.client.get_proof_status::<SP1ProofWithPublicValues>(&request_id).await.unwrap()
            });

            if maybe_proof.is_some() {
                assert_eq!(response.status(), ProofStatus::ProofFulfilled);
                break maybe_proof.unwrap();
            }

            tracing::info!("waiting for proof: {:?}", response.status());
            std::thread::sleep(Duration::from_secs(30));
        };

        self.proof_db.set_proof(&request_id, &proof)?;
        tracing::info!("proof saved to db");

        Ok(())
    }
}

impl Rpc for RpcImpl {
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(spec_impl())
    }

    fn request_prove(
        &self,
        l2_hash: String,
        l1_head_hash: String,
        witness: String,
    ) -> JsonResult<RequestResult> {
        let (l2_hash, l1_head_hash) = check_request(&l2_hash, &l1_head_hash).unwrap();

        // Return cached witness if it exists.
        let proof_result = self.proof_db.get_request_id(&l2_hash, &l1_head_hash);
        if proof_result.is_ok() {
            tracing::info!("return cached proof");
            return Ok(RequestResult::Completed);
        }

        // Send a request to sp1 proof to generate proof"
        println!("requesting proof to sp1 network");
        let service = self.clone();
        let request_id =
            block_on(async move { service.request_prove_to_sp1(witness).await.unwrap() });

        // Store the `request_id` to the database.
        if let Err(_) = self.proof_db.set_request_id(&l2_hash, &l1_head_hash, &request_id) {
            return Ok(RequestResult::UnexpectedError("Failed to store request id".to_string()));
        }
        tracing::info!("sent request to sp1 network: {:?}", request_id);

        // Wait for the proof to be generated.
        let service = self.clone();
        tokio::task::spawn(async move {
            let _ = service.wait_proof(request_id).await;
        });

        Ok(RequestResult::Requested)
    }

    fn get_proof(&self, _l2_hash: String, _l1_head_hash: String) -> JsonResult<ProofResult> {
        todo!()
    }
}
