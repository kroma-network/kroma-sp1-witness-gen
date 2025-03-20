#!/usr/bin/env sh

set dotenv-load

default:
    @just --list

check-env-var name env_file=".env":
    #!/usr/bin/env sh
    # find the non-commented line for the specified variable in the .env file and extract its value.
    value=$(grep -E "^\s*{{name}}=" "{{env_file}}" | grep -v '^\s*#' | head -n 1 | cut -d '=' -f2-)

    if [ -z "$value" ]; then
        echo "[env variable checker]: {{name}} should be set in .env"
        exit 1
    fi

check-env-vars:
    #!/usr/bin/env sh
    just check-env-var L1_RPC
    just check-env-var L1_BEACON_RPC
    just check-env-var L2_RPC
    just check-env-var L2_NODE_RPC

# Only unittest
test *args="-E '!test(test_online)'":
    #!/usr/bin/env sh
    cargo nextest run --release --workspace --all --all-features {{args}}

# Run all tests including online tests
test-all: 
    #!/usr/bin/env sh
    just check-env-vars

    cargo nextest run --release --workspace --all --all-features

