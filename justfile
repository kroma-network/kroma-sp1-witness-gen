#!/usr/bin/env sh

set dotenv-load

default:
    @just --list

run-unit-tests:
    cargo test --release --lib -- --show-output

run-integration-tests:    
    #!/usr/bin/env sh
    WITNESS_STORE_PATH="/tmp/witness_store"
    PROOF_STORE_PATH="/tmp/proof_store"
    
    # build the binaries of the witness generator and prover.
    cargo build --release --bin witness_gen_server
    cargo build --release --bin prover
    
    # Run the witness generator in he backgound.
    ./target/release/witness_gen_server --data $WITNESS_STORE_PATH &
    witness_pid=$!

    # Run the prover in he backgound.
    ./target/release/prover --data $PROOF_STORE_PATH &
    prover_pid=$!
    trap "kill $witness_pid; kill $prover_pid; rm -rf $WITNESS_STORE_PATH; rm -rf $PROOF_STORE_PATH" EXIT

    # Do test
    cargo run --bin integration-tests --release -- \
    --l2-hash "0x564ec49e7c9ea0fe167c0ed3796b9c4ba884e059865c525f198306e72febedf8" \
    --l1-head-hash "0xe22242e0d09d8236658b67553f41b183de2ce0dbbef94daf50dba64610f509a4"
