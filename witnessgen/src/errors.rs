use jsonrpc_http_server::jsonrpc_core::{Error as JsonError, ErrorCode as JsonErrorCode};
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

/// Error Code
#[derive(Debug)]
pub enum ErrorCode {
    InvalidInputHash,
    AlreadyInProgress,
}

impl ErrorCode {
    pub fn code(&self) -> i64 {
        match *self {
            ErrorCode::InvalidInputHash => 1000,
            ErrorCode::AlreadyInProgress => 1001,
        }
    }
}

impl From<i64> for ErrorCode {
    fn from(code: i64) -> Self {
        match code {
            1000 => ErrorCode::InvalidInputHash,
            1001 => ErrorCode::AlreadyInProgress,
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
pub struct WitnessGenError {
    /// Code
    pub code: ErrorCode,
    /// Message
    pub message: Option<String>,
}

impl std::error::Error for WitnessGenError {}

impl std::fmt::Display for WitnessGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<WitnessGenError> for JsonError {
    fn from(err: WitnessGenError) -> Self {
        Self { code: JsonErrorCode::InternalError, message: err.to_string(), data: None }
    }
}

impl WitnessGenError {
    pub fn new(code: ErrorCode, message: Option<String>) -> Self {
        WitnessGenError { code, message }
    }

    pub fn invalid_input_hash(message: String) -> Self {
        Self::new(ErrorCode::InvalidInputHash, Some(message))
    }

    pub fn already_in_progress() -> Self {
        Self::new(ErrorCode::AlreadyInProgress, Some("Another request is in progress".to_string()))
    }
}
