#!/usr/bin/env sh

set dotenv-load

default:
    @just --list

run-unit-tests:
    cargo test --release --lib -- --show-output

run-witness-scenario witness_store="/tmp/witness_store" witness_data="/tmp/witness.json":
    #!/usr/bin/env sh
    # build the witness generator.
    cargo build --release --bin witness_gen_server

    # Run the witness generator in he backgound.
    ./target/release/witness_gen_server --data {{witness_store}} &
    witness_pid=$!
    
    trap "kill $witness_pid; rm -rf {{witness_store}};" EXIT QUIT INT

    # Do test
    cargo run --bin witness_scenario --release -- \
    --l2-hash "0x564ec49e7c9ea0fe167c0ed3796b9c4ba884e059865c525f198306e72febedf8" \
    --l1-head-hash "0xe22242e0d09d8236658b67553f41b183de2ce0dbbef94daf50dba64610f509a4" \
    --witness-data {{witness_data}}

run-proof-scenario proof_store="/tmp/proof_store" witness_data="/tmp/witness.json" proof_data="/tmp/proof.json":
    #!/usr/bin/env sh
    # build the prover.
    cargo build --release --bin prover

    # Run the prover in he backgound.
    ./target/release/prover --data {{proof_store}} &
    prover_pid=$!

    trap "kill $prover_pid; rm -rf {{proof_store}};" EXIT QUIT INT

    # Do test
    cargo run --bin proof_scenario --release -- \
    --l2-hash "0x564ec49e7c9ea0fe167c0ed3796b9c4ba884e059865c525f198306e72febedf8" \
    --l1-head-hash "0xe22242e0d09d8236658b67553f41b183de2ce0dbbef94daf50dba64610f509a4" \
    --witness-data {{witness_data}} \
    --proof-data {{proof_data}}

run-integration-tests:    
    #!/usr/bin/env sh
    WITNESS_STORE_PATH="/tmp/witness_store"
    PROOF_STORE_PATH="/tmp/proof_store"
    WITNESS_DATA="/tmp/witness.json"
    PROOF_DATA="/tmp/proof.json"
    
    # just run-witness-scenario $WITNESS_STORE_PATH $WITNESS_DATA
    just run-proof-scenario $PROOF_STORE_PATH $WITNESS_DATA $PROOF_DATA

