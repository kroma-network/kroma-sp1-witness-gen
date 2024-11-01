use alloy_primitives::{hex::FromHex, B256};
use anyhow::Result;

pub fn preprocessing(l2_hash: &str, l1_head_hash: &str) -> Result<(B256, B256, String)> {
    let user_req_id = format!(
        "{}-{}",
        l2_hash.chars().take(8).collect::<String>(),
        l1_head_hash.chars().take(8).collect::<String>()
    );
    let l2_hash = b256_from_str(l2_hash)?;
    let l1_head_hash = b256_from_str(l1_head_hash)?;

    Ok((l2_hash, l1_head_hash, user_req_id))
}

pub fn b256_from_str(s: &str) -> Result<B256> {
    match B256::from_hex(s.strip_prefix("0x").unwrap_or(s)) {
        Ok(b) => Ok(b),
        Err(e) => Err(anyhow::anyhow!("failed to parse B256 from string: {}", e)),
    }
}
