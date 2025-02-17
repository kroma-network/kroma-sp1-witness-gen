# Kroma SP1 Prover

This project provides `WitnessGenerator` for users attempting to assert the validity of a specific 
block on [Optimism](https://www.optimism.io/).

Since the order of all transactions included in an Optimism block is finalized when they are
uploaded to L1, verifying the validity of an L2 block starts with downloading the transactions it
includes from L1 and then executing them. This procedure, known as the derivation process, utilizes
the [Kona](https://github.com/anton-rs/kona) implementation. 

The witness refers to all the external data required during the execution of the derivation process. This witness can be passed to the [zkvm-prover-proxy](https://github.com/kroma-network/zkvm-prover-proxy) to generate the sp1 proof.

## Run a Witness Generator

### Environment Variables

The 4 RPC endpoints must be set to run `WitnessGenerator`. It is recommended to fill in
the following environment variables in the `.env` file.

```shell
L1_RPC=
L1_BEACON_RPC=
L2_RPC=
L2_NODE_RPC=
MAX_BATCH_POST_DELAY_MIN=<Integer - According to the Sequencer's spec>
SKIP_SIMULATION=<Boolen - if `true`, it returns the generated witness without simulating it>
```

### Run

``` shell
> cargo run --bin witness-gen-server --release -- --endpoint <IP_WITH_PORT> --data <DB_PATH>
```

``` shell
# example
> cargo run --bin witness-gen-server --release -- --endpoint 0.0.0.0:3030 --data /data/witness_store
```

### API Overview

#### `requestWitness` method

Register a request to generate a witness.

``` shell
{
    "jsonrpc": "2.0",
    "method": "requestWitness",
    "params": [<0xL2Hash>, <0xL1HeadHash>],
    "id": 0
}
```

#### `getWitness` method

It returns the witness after finishing to generate it.

``` shell
{
    "jsonrpc": "2.0",
    "method": "getWitness",
    "params": [<0xL2Hash>, <0xL1HeadHash>],
    "id": 0
}
```

## Integration Test

The test client requests generating `Witness` to the `WitnessGenerator`.

``` shell
> just run-witness-scenario <0xL2Hash> <0xL1HeadHash>
```

## Build Docker Images

Inject the .env file described above.

``` shell
> make build-witness-generator
> docker build -f docker/Dockerfile.witnessgen.ubuntu -t witness-gen .
> docker run -itd --env-file .env -p 3030:3030 witness-gen
```

## Useful scripts

### Preview script

This script provides a general context corresponding to the specific l2 block in advance.

```shell
> cargo run --release --bin script -- --method preview --l2-block <L2Number>
# - output_root: <OutputRootAtTheL2Block>
# - parent_output_root: <OutputRootAtThePreviousL2Block>
# - l1_origin_hash: <L1OriginBlockHashOfTheL2Block>
# - l1_origin_number: <L1OriginBlockNumberOfTheL2Block>
# - l1_head_number: <L1OriginBlockNumberOfTheL2Block> + 300
```

### Execute script

Execute the specific l2 block.

```shell
> cargo run --release --bin script -- --method execute --l2-block <L2Number> --l1-head-number <L2HeadNumber>

# Or
> cargo run --release --bin script -- --method execute --l2-block <L2Number> --l1-head-hash <L2HeadHash>
```



