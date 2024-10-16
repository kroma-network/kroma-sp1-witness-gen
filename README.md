# kroma-sp1-prover

## Environment variables

The following env variables indicating rpc endpoints must be set to run a script or prover
components. Using `.env` file is recommended.

```shell
L1_RPC=https://ultra-newest-firefly.quiknode.pro/eb0a9e666cc768abb9992fbefe8f85780bd57b0e
L1_BEACON_RPC=https://ultra-newest-firefly.quiknode.pro/eb0a9e666cc768abb9992fbefe8f85780bd57b0e

L2_RPC=https://greatest-side-silence.optimism.quiknode.pro/e5dbdeac65fe692a1ba9dff0c74ad2b9ef2bcf3f
L2_NODE_RPC=https://greatest-side-silence.optimism.quiknode.pro/e5dbdeac65fe692a1ba9dff0c74ad2b9ef2bcf3f
```

## Run the core logic

### Preview script

The parameters required for block execution are displayed.

```shell
> cargo run --release --bin script -- --method preview --l2-block 124748230
# - output_root: 0x48f9fe8c248855f9da9e8f6e06f3215f57a6a96f684b5fc63232b565bdb98479
# - parent_output_root: 0xee40a952b20e40f8911e64179baf6afe0cac0e6226f18719560efa55ee51e701
# - l1_origin_hash: 0xfbd8b0793a7f495f6ff812b5cd25e5d127e154b5324f3f2e38b6c43d902f71af
# - l1_origin_number: 20647540
# - l1_head_number: 20647840
```

### Execute script

Execute the specific l2 block.

```shell
> cargo run --release --bin script -- --method execute --l2-block 124748230 --l1-head-number 20647840
```

## Run the WitnessGenerator

It generates a witness and store it to db (of data located at the `data/witness_store`).

```shell
# endpoint will be set as "127.0.0.1:3030" as default if `--endpoint` was not provided.
> cargo run --bin witness_gen_server --release -- --endpoint 127.0.0.1:3030
```

To test it, mock client can be used.

```shell
> cargo run --bin witness_gen_client --release -- --spec
> cargo run --bin witness_gen_client --release -- --request
> cargo run --bin witness_gen_client --release -- --get
```
