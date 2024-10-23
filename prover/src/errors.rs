use jsonrpc_http_server::jsonrpc_core::{Error as JsonError, ErrorCode as JsonErrorCode};
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

/// Error Code
#[derive(Debug, Clone)]
pub enum ProverErrorCode {
    ProofGenerationFailed,
    InvalidInputHash,
    DBError,
    SP1NetworkError,
}

impl ProverErrorCode {
    pub fn code(&self) -> i64 {
        match *self {
            ProverErrorCode::ProofGenerationFailed => 1000,
            ProverErrorCode::DBError => 2000,
            ProverErrorCode::InvalidInputHash => 3000,
            ProverErrorCode::SP1NetworkError => 4000,
        }
    }

    pub fn default_message(&self) -> String {
        match *self {
            ProverErrorCode::ProofGenerationFailed => String::from("Proof generation failed"),
            ProverErrorCode::DBError => String::from("Database error"),
            ProverErrorCode::InvalidInputHash => String::from("Invalid parameters"),
            ProverErrorCode::SP1NetworkError => String::from("SP1 network error"),
        }
    }
}

impl From<i64> for ProverErrorCode {
    fn from(code: i64) -> Self {
        match code {
            1000 => ProverErrorCode::ProofGenerationFailed,
            2000 => ProverErrorCode::DBError,
            3000 => ProverErrorCode::InvalidInputHash,
            4000 => ProverErrorCode::SP1NetworkError,
            _ => panic!("not supported code: {:?}", code),
        }
    }
}

impl<'a> Deserialize<'a> for ProverErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<ProverErrorCode, D::Error>
    where
        D: Deserializer<'a>,
    {
        let code: i64 = Deserialize::deserialize(deserializer)?;
        Ok(ProverErrorCode::from(code))
    }
}

impl Serialize for ProverErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.code())
    }
}

/// Error object as defined in Spec
#[derive(Debug)]
pub struct ProverError {
    /// Code
    pub code: ProverErrorCode,
    /// Message
    pub message: Option<String>,
}

impl std::error::Error for ProverError {}

impl std::fmt::Display for ProverError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&ProverError> for JsonError {
    fn from(err: &ProverError) -> Self {
        Self { code: JsonErrorCode::InternalError, message: err.to_string(), data: None }
    }
}

impl ProverError {
    pub fn new(code: ProverErrorCode, message: Option<String>) -> Self {
        ProverError { code, message }
    }

    pub fn to_json_error(&self) -> JsonError {
        JsonError::from(self)
    }

    pub fn proof_generation_failed(msg: Option<String>) -> Self {
        let code = ProverErrorCode::ProofGenerationFailed;
        let msg = match msg {
            Some(m) => m.clone(),
            None => code.default_message(),
        };
        Self::new(code.clone(), Some(msg))
    }

    pub fn db_error(msg: String) -> Self {
        let code = ProverErrorCode::DBError;
        Self::new(code.clone(), Some(msg))
    }

    pub fn invalid_input_hash(msg: String) -> Self {
        let code = ProverErrorCode::InvalidInputHash;
        Self::new(code.clone(), Some(msg))
    }

    pub fn sp1_network_error(msg: String) -> Self {
        let code = ProverErrorCode::SP1NetworkError;
        Self::new(code.clone(), Some(msg))
    }
}
