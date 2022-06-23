#!/bin/env bash

# Default VXI11 server arguments
: ${VXI11_SERVER_ARGS:="--register"}

##### Setup cargo-llvm-cov #####
source <(cargo llvm-cov show-env --export-prefix)
cargo llvm-cov clean --workspace
cargo build 

##### Run rust tests #####
cargo test

##### Run python tests #####
# Start the server
cargo run --example vxi11 -- --timeout 10000 $VXI11_SERVER_ARGS &
COVERAGE_PID=$!
# Run pytest and tell it to not instantiate its own server
VXI11_TARGET="127.0.0.1" pytest
# Wait for server to exit
wait $COVERAGE_PID

##### Generate and open report #####
cargo llvm-cov --no-run "$@"
