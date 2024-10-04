use jsonrpc_core::Result as JsonResult;
use jsonrpc_derive::rpc;

use crate::spec_impl::{spec_impl, SpecResult};

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    fn spec(&self) -> JsonResult<SpecResult> {
        Ok(spec_impl())
    }
}

pub struct RpcImpl;

impl Rpc for RpcImpl {}
