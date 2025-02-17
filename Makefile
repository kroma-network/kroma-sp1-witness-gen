build-witness-generator:
	cargo build --bin witness-gen-server --release

run-witness-generator:
	END_POINT ?= "0.0.0.0:3030"
	WITNESS_DB ?= "/tmp/witness_db"
	
	$(MAKE) build-witness-generator
	./target/release/witness-gen-server --endpoint $(END_POINT) --data $(WITNESS_DB)
