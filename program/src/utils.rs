use alloy_consensus::{Header, Sealed};
use alloy_primitives::{address, keccak256, Address, B256};
use kona_executor::{TrieDB, TrieDBError, TrieDBProvider};
use kona_mpt::TrieHinter;

/// Fetches the storage root of the L2-to-L1 message passer contract at the specified height of block.
fn get_storage_root<F: TrieDBProvider, H: TrieHinter>(
    header: &Sealed<Header>,
    provider: F,
    hinter: H,
) -> B256 {
    #[cfg(not(feature = "kroma"))]
    const L2_TO_L1_MESSAGE_PASSER_ADDRESS: Address =
        address!("4200000000000000000000000000000000000016");
    #[cfg(feature = "kroma")]
    const L2_TO_L1_MESSAGE_PASSER_ADDRESS: Address =
        address!("4200000000000000000000000000000000000003");

    let mut trie_db = TrieDB::new(header.state_root, header.clone(), provider, hinter);
    trie_db
        .get_trie_account(&L2_TO_L1_MESSAGE_PASSER_ADDRESS)
        .unwrap()
        .ok_or(TrieDBError::MissingAccountInfo)
        .unwrap()
        .storage_root
}

/// Computes the output root of the specified block header.
pub fn compute_output_root_at<F: TrieDBProvider, H: TrieHinter>(
    header: &Sealed<Header>,
    provider: F,
    hinter: H,
) -> B256 {
    const OUTPUT_ROOT_VERSION: u8 = 0;

    let storage_root = get_storage_root(header, provider, hinter);

    // Construct the raw output.
    let mut raw_output = [0u8; 128];
    raw_output[31] = OUTPUT_ROOT_VERSION;
    raw_output[32..64].copy_from_slice(header.state_root.as_ref());
    raw_output[64..96].copy_from_slice(storage_root.as_ref());
    raw_output[96..128].copy_from_slice(header.seal().as_ref());
    keccak256(raw_output)
}
