pub mod errors;
pub mod interface;
pub mod proof_db;
pub mod types;
pub mod utils;

// NOTE(Ethan): equals to `DEFAULT_NETWORK_RPC_URL`` in sp1/creates/sdk/src/network/mod.rs
pub const DEFAULT_NETWORK_RPC_URL: &str = "https://rpc.production.succinct.xyz/";
pub const DEFAULT_PROOF_STORE_PATH: &str = "data/proof_store";
