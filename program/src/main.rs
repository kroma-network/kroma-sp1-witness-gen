//! A program to verify a OP Stack chain's block STF in the zkVM.
//!
//! This binary contains the client program for executing the Optimism rollup state transition
//! across a single block, which can be used in an on chain dispute game. Depending on the
//! compilation pipeline, it will compile to be run either in native mode or in zkVM mode. In native
//! mode, the data for verifying the execute of the Optimism rollup's state transition is fetched
//! from RPC, while in zkVM mode, the data is supplied by the host binary to the verifiable program.

#![cfg_attr(target_os = "zkvm", no_main)]

extern crate alloc;

use alloc::sync::Arc;

use alloy_consensus::{BlockBody, Sealed};
use alloy_eips::{eip2718::Decodable2718, eip4895::Withdrawals};
use cfg_if::cfg_if;
use kona_executor::StatelessL2BlockExecutor;
use kona_proof::{
    l1::{OracleBlobProvider, OracleL1ChainProvider},
    BootInfo,
};
use log::info;
use op_alloy_consensus::{OpBlock, OpTxEnvelope};
use op_succinct_client_utils::{
    driver::MultiBlockDerivationDriver, l2_chain_provider::MultiblockOracleL2ChainProvider,
    precompiles::zkvm_handle_register,
};
cfg_if! {
    if #[cfg(feature = "kroma")] {
        mod utils;
        use utils::compute_output_root_at;
    }
}

cfg_if! {
    if #[cfg(target_os = "zkvm")] {
        sp1_zkvm::entrypoint!(main);
        use op_succinct_client_utils::{InMemoryOracle, boot::BootInfoStruct, BootInfoWithBytesConfig};
        use op_alloy_genesis::RollupConfig;
        use alloc::vec::Vec;
        use serde_json;
    } else {
        use kona_proof::CachingOracle;
        use op_succinct_client_utils::pipes::{ORACLE_READER, HINT_WRITER};
    }
}

cfg_if! {
    if #[cfg(feature = "kroma")] {
        use kona_derive::traits::ChainProvider;
    }
}

fn main() {
    cfg_if! {
        if #[cfg(all(target_os = "zkvm", feature = "kroma"))] {
            println!("Configuration: target_os(zkvm), feature(kroma)");
        } else {
            println!("Configuration: target_os(native)");
        }
    }

    op_succinct_client_utils::block_on(async move {
        ////////////////////////////////////////////////////////////////
        //                          PROLOGUE                          //
        ////////////////////////////////////////////////////////////////

        cfg_if! {
            // If we are compiling for the zkVM, read inputs from SP1 to generate boot info
            // and in memory oracle.
            if #[cfg(target_os = "zkvm")] {
                println!("cycle-tracker-start: boot-load");
                let boot_info_with_bytes_config = sp1_zkvm::io::read::<BootInfoWithBytesConfig>();

                // BootInfoStruct is identical to BootInfoWithBytesConfig, except it replaces
                // the rollup_config_bytes with a hash of those bytes (rollupConfigHash). Securely
                // hashes the rollup config bytes.
                let boot_info_struct = BootInfoStruct::from(boot_info_with_bytes_config.clone());
                #[cfg(not(feature = "kroma"))]
                sp1_zkvm::io::commit::<BootInfoStruct>(&boot_info_struct);

                let rollup_config: RollupConfig = serde_json::from_slice(&boot_info_with_bytes_config.rollup_config_bytes).expect("failed to parse rollup config");
                let boot: Arc<BootInfo> = Arc::new(BootInfo {
                    l1_head: boot_info_with_bytes_config.l1_head,
                    agreed_l2_output_root: boot_info_with_bytes_config.l2_output_root,
                    claimed_l2_output_root: boot_info_with_bytes_config.l2_claim,
                    claimed_l2_block_number: boot_info_with_bytes_config.l2_claim_block,
                    chain_id: boot_info_with_bytes_config.chain_id,
                    rollup_config,
                });
                println!("cycle-tracker-end: boot-load");

                println!("cycle-tracker-start: oracle-load");
                let in_memory_oracle_bytes: Vec<u8> = sp1_zkvm::io::read_vec();
                let oracle = Arc::new(InMemoryOracle::from_raw_bytes(in_memory_oracle_bytes));
                println!("cycle-tracker-end: oracle-load");

                println!("cycle-tracker-start: oracle-verify");
                oracle.verify().expect("key value verification failed");
                println!("cycle-tracker-end: oracle-verify");
            }
            // If we are compiling for online mode, create a caching oracle that speaks to the
            // fetcher via hints, and gather boot info from this oracle.
            else {
                let oracle = Arc::new(CachingOracle::new(1024, ORACLE_READER, HINT_WRITER));
                let boot = Arc::new(BootInfo::load(oracle.as_ref()).await.unwrap());
            }
        }

        #[allow(unused_mut)]
        let mut l1_provider = OracleL1ChainProvider::new(boot.clone(), oracle.clone());
        let mut l2_provider = MultiblockOracleL2ChainProvider::new(boot.clone(), oracle.clone());
        let beacon = OracleBlobProvider::new(oracle.clone());

        ////////////////////////////////////////////////////////////////
        //                   DERIVATION & EXECUTION                   //
        ////////////////////////////////////////////////////////////////

        println!("cycle-tracker-start: derivation-instantiation");
        let mut driver = MultiBlockDerivationDriver::new(
            boot.as_ref(),
            oracle.as_ref(),
            beacon,
            l1_provider.clone(),
            l2_provider.clone(),
        )
        .await
        .unwrap();
        println!("cycle-tracker-end: derivation-instantiation");

        #[cfg(feature = "kroma")]
        {
            println!("cycle-tracker-start: compute-parent-output-root");
            let l2_safe_header = driver.clone_l2_safe_head_header();
            let parent_output_root =
                compute_output_root_at(&l2_safe_header, l2_provider.clone(), l2_provider.clone());
            #[cfg(target_os = "zkvm")]
            sp1_zkvm::io::commit(&parent_output_root);
            println!("Validated Parent Output Root: {}", parent_output_root);
            println!("cycle-tracker-end: compute-parent-output-root");

            println!("cycle-tracker-start: check-l1-connectivity");
            let l1_origin = driver.l2_safe_head.l1_origin;
            let mut current_header = l1_provider.header_by_hash(boot.l1_head).await.unwrap();

            let loop_num = current_header.number - l1_origin.number;
            for _ in 0..loop_num {
                let parent_header =
                    l1_provider.header_by_hash(current_header.parent_hash).await.unwrap();
                assert_eq!(parent_header.hash_slow(), current_header.parent_hash);
                current_header = parent_header;
            }
            assert_eq!(current_header.hash_slow(), l1_origin.hash);
            assert_eq!(current_header.number, l1_origin.number);
            println!("cycle-tracker-end: check-l1-connectivity");
        }

        // The initial payload requires block derivation.
        println!("cycle-tracker-report-start: payload-derivation");
        let mut payload = driver.produce_payload().await.unwrap();
        println!("cycle-tracker-report-end: payload-derivation");

        println!("cycle-tracker-start: execution-instantiation");
        let mut executor = StatelessL2BlockExecutor::builder(
            &boot.rollup_config,
            l2_provider.clone(),
            l2_provider.clone(),
        )
        .with_parent_header(driver.clone_l2_safe_head_header())
        .with_handle_register(zkvm_handle_register)
        .build();
        println!("cycle-tracker-end: execution-instantiation");

        // The initial payload requires block derivation.
        let mut l2_block_info;
        let mut new_block_header;
        'step: loop {
            // Execute the payload to generate a new block header.
            info!("Executing Payload for L2 Block: {}", payload.parent.block_info.number + 1);
            println!("cycle-tracker-report-start: block-execution");
            new_block_header = executor.execute_payload(payload.attributes.clone()).unwrap();
            println!("cycle-tracker-report-end: block-execution");
            let new_block_number = new_block_header.number;
            assert_eq!(new_block_number, payload.parent.block_info.number + 1);

            // Increment last_block_num and check if we have reached the claim block.
            if new_block_number == boot.claimed_l2_block_number {
                break 'step;
            }

            // Generate the Payload Envelope, which can be used to derive cached data.
            let optimism_block = OpBlock {
                header: new_block_header.clone(),
                body: BlockBody {
                    transactions: payload
                        .attributes
                        .transactions
                        .unwrap()
                        .iter()
                        .map(|raw_tx| OpTxEnvelope::decode_2718(&mut raw_tx.as_ref()).unwrap())
                        .collect::<Vec<OpTxEnvelope>>(),
                    ommers: Vec::new(),
                    withdrawals: boot
                        .rollup_config
                        .is_canyon_active(new_block_header.timestamp)
                        .then(|| Withdrawals(vec![])),
                },
            };
            // Add all data from this block's execution to the cache.
            l2_block_info = l2_provider
                .update_cache(new_block_header, optimism_block, &boot.rollup_config)
                .unwrap();

            // Update data for the next iteration.
            driver.update_safe_head(
                l2_block_info,
                Sealed::new_unchecked(new_block_header.clone(), new_block_header.hash_slow()),
            );

            println!("cycle-tracker-report-start: payload-derivation");
            // Produce the next payload. If a span batch boundary is passed, the driver will step until the next batch.
            payload = driver.produce_payload().await.unwrap();
            println!("cycle-tracker-report-end: payload-derivation");
        }

        println!("cycle-tracker-start: output-root");
        let output_root = executor.compute_output_root().unwrap();
        println!("cycle-tracker-end: produce-output");

        ////////////////////////////////////////////////////////////////
        //                          EPILOGUE                          //
        ////////////////////////////////////////////////////////////////

        // Note: We don't need the last_block_num == claim_block check, because it's the only way to
        // exit the loop in `driver.produce_output`.
        assert_eq!(output_root, boot.claimed_l2_output_root);

        cfg_if! {
            if #[cfg(all(target_os = "zkvm", feature = "kroma"))] {
                sp1_zkvm::io::commit(&output_root);
                sp1_zkvm::io::commit(&boot.l1_head);
                println!("Validated derivation and STF. Output Root: {} | L1 Head: {}", output_root, boot.l1_head);
            } else {
                println!("Validated derivation and STF. Output Root: {}", output_root);
            }
        }
    });
}
