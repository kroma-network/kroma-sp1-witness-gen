#!/usr/bin/env sh

set dotenv-load

default:
    @just --list

run-unit-tests:
    cargo test --release --lib -- --show-output

run-witness-scenario l2_hash l1_head_hash witness_store="/tmp/witness_store" witness_data="/tmp/witness.json":
    #!/usr/bin/env sh
    # build the witness generator.
    cargo build --release --bin witness-gen-server

    # Run the witness generator in the background.
    ./target/release/witness-gen-server --data {{witness_store}} &
    witness_pid=$!
    
    trap "kill $witness_pid; rm -rf {{witness_store}};" EXIT QUIT INT

    # Do test
    cargo run --bin witness-scenario --release -- \
    --l2-hash {{l2_hash}} \
    --l1-head-hash {{l1_head_hash}} \
    --witness-data {{witness_data}}

run-proof-scenario l2_hash l1_head_hash proof_store="/tmp/proof_store" witness_data="/tmp/witness.json" proof_data="/tmp/proof.json":
    #!/usr/bin/env sh
    # build the prover.
    cargo build --release --bin prover-proxy

    # Run the prover in the background.
    ./target/release/prover-proxy --data {{proof_store}} &
    prover_pid=$!

    trap "kill $prover_pid; rm -rf {{proof_store}};" EXIT QUIT INT

    # Do test
    cargo run --bin proof-scenario --release -- \
    --l2-hash {{l2_hash}} \
    --l1-head-hash {{l1_head_hash}} \
    --witness-data {{witness_data}} \
    --proof-data {{proof_data}}

run-onchain-verify proof_data="/tmp/proof.json":
    #!/usr/bin/env sh
    anvil --accounts 1 &
    geth_pid=$!
    
    trap "kill $geth_pid" EXIT QUIT INT

    // Deploy the verifier contract.
    cd sp1-contracts/contracts
    forge create --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 src/v3.0.0/SP1VerifierPlonk.sol:SP1Verifier
    cd ../../

    program_key=$(jq -r '.program_key' {{proof_data}})
    public_values=$(jq -r '.public_values' {{proof_data}})
    proof=$(jq -r '.proof' {{proof_data}})

    cast call 0x5FbDB2315678afecb367f032d93F642f64180aa3 "verifyProof(bytes32,bytes calldata,bytes calldata)" $program_key $public_values $proof

run-integration-tests l2_hash="0x564ec49e7c9ea0fe167c0ed3796b9c4ba884e059865c525f198306e72febedf8" l1_head_hash="0xe22242e0d09d8236658b67553f41b183de2ce0dbbef94daf50dba64610f509a4":    
    #!/usr/bin/env sh
    WITNESS_STORE_PATH="/tmp/witness_store"
    PROOF_STORE_PATH="/tmp/proof_store"
    WITNESS_DATA="/tmp/witness.json"
    PROOF_DATA="/tmp/proof.json"
    
    just run-witness-scenario {{l2_hash}} {{l1_head_hash}} $WITNESS_STORE_PATH $WITNESS_DATA
    
    just run-proof-scenario {{l2_hash}} {{l1_head_hash}} $PROOF_STORE_PATH $WITNESS_DATA $PROOF_DATA
    
    # just run-onchain-verify $PROOF_DATA

    rm -rf $WITNESS_DATA
    rm -rf $PROOF_DATA
