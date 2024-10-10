use alloy_primitives::B256;
use anyhow::Result;
use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;

use crate::request_witness_impl::{check_request, generate_witness_impl, RequestResult};
use crate::spec_impl::{spec_impl, SpecResult};

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(spec_impl())
    }

    #[rpc(name = "requestWitness")]
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult>;
}

#[derive(Clone)]
pub struct RpcImpl;

impl RpcImpl {
    pub async fn generate_witness(&self, l2_hash: B256, l1_head_hash: B256) -> Result<()> {
        tracing::info!("start to generate witness");
        let _witness_result = generate_witness_impl(l2_hash, l1_head_hash).unwrap();
        tracing::info!("successfully witness result generated");

        // TODO: store the result to db.
        Ok(())
    }
}

impl Rpc for RpcImpl {
    fn request_witness(&self, l2_hash: String, l1_head_hash: String) -> JsonResult<RequestResult> {
        let (l2_hash, l1_head_hash) = check_request(&l2_hash, &l1_head_hash).unwrap();

        tracing::info!("do generate witness");
        let service = self.clone();
        tokio::task::spawn(async move { service.generate_witness(l2_hash, l1_head_hash).await });

        Ok(RequestResult::Processing)
    }
}
