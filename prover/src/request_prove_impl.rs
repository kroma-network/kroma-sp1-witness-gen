use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RequestResult {
    None,
    Requested,
    Processing,
    Completed,
    UnexpectedError(String),
}
