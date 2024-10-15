use alloy_primitives::{hex::FromHex, B256};
use anyhow::Result;
use std::env;

use crate::errors::KromaError;

pub fn check_request(l2_hash: &String, l1_head_hash: &String) -> Result<(B256, B256)> {
    let l2_hash = b256_from_str(&l2_hash).unwrap();
    let l1_head_hash = b256_from_str(&l1_head_hash).unwrap();

    // TODO: check more detail.

    Ok((l2_hash, l1_head_hash))
}

pub fn check_endpoints() -> Result<(), KromaError> {
    if env::var("L1_RPC").is_err() {
        return Err(KromaError::empty_l1_rpc_endpoint());
    }
    if env::var("L1_BEACON_RPC").is_err() {
        return Err(KromaError::empty_l1_beacon_endpoint());
    }
    if env::var("L2_RPC").is_err() {
        return Err(KromaError::empty_l2_rpc_endpoint());
    }
    if env::var("L2_NODE_RPC").is_err() {
        return Err(KromaError::empty_l2_node_rpc_endpoint());
    }
    Ok(())
}

/// Convert a u32 array to a u8 array. Useful for converting the range vkey to a B256.
pub fn u32_to_u8(input: [u32; 8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    for (i, &value) in input.iter().enumerate() {
        let bytes = value.to_be_bytes();
        output[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }
    output
}

pub fn b256_from_str(s: &str) -> Result<B256> {
    match B256::from_hex(s.strip_prefix("0x").unwrap_or(s)) {
        Ok(b) => Ok(b),
        Err(e) => Err(anyhow::anyhow!("failed to parse B256 from string: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32_to_u8() {
        let input = [1, 2, 3, 4, 5, 6, 7, 8];
        let output = u32_to_u8(input);
        assert_eq!(
            output,
            [
                0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 7,
                0, 0, 0, 8
            ]
        );
    }
}
