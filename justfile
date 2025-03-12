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
    
    # Wait for the witness generator to start.
    sleep 10

    trap "kill $witness_pid; rm -rf {{witness_store}};" EXIT QUIT INT

    # Do test
    cargo run --bin witness-scenario --release -- \
    --l2-hash {{l2_hash}} \
    --l1-head-hash {{l1_head_hash}} \
    --witness-data {{witness_data}}
