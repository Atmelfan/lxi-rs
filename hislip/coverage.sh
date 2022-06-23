#!/bin/env bash

# Default VXI11 server arguments
: ${HISLIP_SERVER_ARGS:=""}

##### Setup cargo-llvm-cov #####
source <(cargo llvm-cov show-env --export-prefix)
cargo llvm-cov clean --workspace
cargo build 

##### Run rust tests #####
cargo test

##### Run python tests #####
# Start the server ()
cargo run --example server -- --timeout 10000 $HISLIP_SERVER_ARGS &
COVERAGE_PID=$!
# Run pytest and tell it to not instantiate its own server
HISLIP_TARGET="127.0.0.1:4880" pytest
# Wait for server to exit
wait $COVERAGE_PID

##### Generate and open report #####
cargo llvm-cov --no-run "$@"
