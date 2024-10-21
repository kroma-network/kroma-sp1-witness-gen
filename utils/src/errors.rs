use jsonrpc_http_server::jsonrpc_core::{Error as JsonError, ErrorCode as JsonErrorCode};
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

/// Error Code
#[derive(Debug)]
pub enum ErrorCode {
    EmptyL1RpcUrl,
    EmptyL1BeaconUrl,
    EmptyL2RpcUrl,
    EmptyL2NodeRpcUrl,
    ServerBusy,
}

impl ErrorCode {
    pub fn code(&self) -> i64 {
        match *self {
            // Endpoint empty starts with `1`
            ErrorCode::EmptyL1RpcUrl => 1000,
            ErrorCode::EmptyL1BeaconUrl => 1001,
            ErrorCode::EmptyL2RpcUrl => 1002,
            ErrorCode::EmptyL2NodeRpcUrl => 1003,
            ErrorCode::ServerBusy => 2000,
        }
    }
}

impl From<i64> for ErrorCode {
    fn from(code: i64) -> Self {
        match code {
            1000 => ErrorCode::EmptyL1RpcUrl,
            1001 => ErrorCode::EmptyL1BeaconUrl,
            1002 => ErrorCode::EmptyL2RpcUrl,
            1003 => ErrorCode::EmptyL2NodeRpcUrl,
            2000 => ErrorCode::ServerBusy,
            _ => panic!("not supported code: {:?}", code),
        }
    }
}

impl<'a> Deserialize<'a> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<ErrorCode, D::Error>
    where
        D: Deserializer<'a>,
    {
        let code: i64 = Deserialize::deserialize(deserializer)?;
        Ok(ErrorCode::from(code))
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.code())
    }
}

/// Error object as defined in Spec
#[derive(Debug)]
pub struct KromaError {
    /// Code
    pub code: ErrorCode,
    /// Message
    pub message: Option<String>,
}

impl std::error::Error for KromaError {}

impl std::fmt::Display for KromaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<KromaError> for JsonError {
    fn from(err: KromaError) -> Self {
        Self { code: JsonErrorCode::InternalError, message: err.to_string(), data: None }
    }
}

impl KromaError {
    pub fn new(code: ErrorCode, message: Option<String>) -> Self {
        KromaError { code, message }
    }

    pub fn empty_l1_rpc_endpoint() -> Self {
        Self::new(ErrorCode::EmptyL1RpcUrl, Some("L1 RPC endpoint is not found".to_string()))
    }

    pub fn empty_l1_beacon_endpoint() -> Self {
        Self::new(ErrorCode::EmptyL1BeaconUrl, Some("L1 Beacon endpoint is not found".to_string()))
    }

    pub fn empty_l2_rpc_endpoint() -> Self {
        Self::new(ErrorCode::EmptyL2RpcUrl, Some("L2 RPC endpoint is not found".to_string()))
    }

    pub fn empty_l2_node_rpc_endpoint() -> Self {
        Self::new(
            ErrorCode::EmptyL2NodeRpcUrl,
            Some("L2 Node RPC endpoint is not found".to_string()),
        )
    }

    pub fn server_busy() -> Self {
        Self::new(ErrorCode::ServerBusy, Some("Server is busy".to_string()))
    }
}
