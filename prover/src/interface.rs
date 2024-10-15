use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;

use crate::get_proof_impl::ProofResult;
use crate::request_prove_impl::RequestResult;
use crate::spec_impl::{spec_impl, SpecResult};

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
pub struct RpcImpl {}

impl RpcImpl {
    pub fn new(store_path: &str) -> Self {
        todo!()
    }
}

impl Default for RpcImpl {
    fn default() -> Self {
        todo!()
    }
}

impl Rpc for RpcImpl {
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(spec_impl())
    }

    fn request_prove(
        &self,
        _l2_hash: String,
        _l1_head_hash: String,
        _witness: String,
    ) -> JsonResult<RequestResult> {
        todo!()
    }

    fn get_proof(&self, _l2_hash: String, _l1_head_hash: String) -> JsonResult<ProofResult> {
        todo!()
    }
}
