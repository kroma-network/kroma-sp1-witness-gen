use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_primitives::B256;
use op_succinct_host_utils::fetcher::OPSuccinctDataFetcher;
use serde::{Deserialize, Serialize};

use crate::{get_l1_origin_of, get_output_at, init_fetcher};

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub number_dec: u64,
    pub number_hex: String,
    pub hash: B256,
}

impl Block {
    pub fn new(number_dec: u64, number_hex: String, hash: B256) -> Self {
        Self { number_dec, number_hex, hash }
    }

    pub async fn from_l1_block_id(
        l1_number_or_hash: BlockId,
        fetcher_opt: Option<&OPSuccinctDataFetcher>,
    ) -> Self {
        let fetcher = match fetcher_opt {
            Some(fetcher) => fetcher,
            _ => &init_fetcher(),
        };

        let header = fetcher.get_l1_header(l1_number_or_hash.into()).await.unwrap();

        Self {
            number_dec: header.number,
            number_hex: format!("{:x}", header.number),
            hash: header.hash_slow(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviewReport {
    l2_block_number: u64,
    output_root: B256,
    parent_output_root: B256,

    l1_origin: Block,
}

impl PreviewReport {
    pub async fn from_fetcher(
        l2_block_number: u64,
        fetcher_opt: Option<&OPSuccinctDataFetcher>,
    ) -> Self {
        let (_, origin_number) = get_l1_origin_of(l2_block_number, fetcher_opt);

        Self {
            l2_block_number,
            output_root: get_output_at(l2_block_number, fetcher_opt),
            parent_output_root: get_output_at(l2_block_number - 1, fetcher_opt),
            l1_origin: Block::from_l1_block_id(origin_number.into(), fetcher_opt).await,
        }
    }

    pub async fn l1_head(
        &self,
        distance: u64,
        fetcher_opt: Option<&OPSuccinctDataFetcher>,
    ) -> Block {
        let l1_head_number = self.l1_origin.number_dec + distance;

        let fetcher = match fetcher_opt {
            Some(fetcher) => fetcher,
            _ => &init_fetcher(),
        };

        let latest_l1_header =
            fetcher.get_l1_header(BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();

        if latest_l1_header.number < l1_head_number {
            panic!("L1 Head Number exceeds the latest L1 block number");
        }

        Block::from_l1_block_id(l1_head_number.into(), fetcher_opt).await
    }
}
