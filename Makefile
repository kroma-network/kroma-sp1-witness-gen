generate-rollup-config:
	cargo run --bin script --release -- --method config

build-native-host-runner:
	cargo build --workspace --bin native_host_runner --target-dir target/native_host_runner --release

build-native-program: 
	cargo build --workspace --bin fault-proof --profile release-client-lto --features kroma

build-witness-generator: 
	$(MAKE) generate-rollup-config
	$(MAKE) build-native-host-runner
	$(MAKE) build-native-program
	cargo build --bin witness-gen-server --release

run-witness-generator:
	END_POINT ?= "127.0.0.1:3030"
	WITNESS_DB ?= "/tmp/witness_db"
	
	$(MAKE) build-witness-generator
	./target/release/witness-gen-server --endpoint $(END_POINT) --data $(WITNESS_DB)