#!/bin/env bash

TIMEOUT=2500
CARGO_ARGS="-q"

##### Setup cargo-llvm-cov #####
source <(cargo llvm-cov show-env --export-prefix)
cargo llvm-cov clean --workspace
cargo build 

##### Run rust tests #####
cargo test

##### Run vxi11 tests #####
# Start the server
./target/debug/examples/vxi11 --timeout $TIMEOUT --register &
VXI11_PID=$!
# Run pytest and tell it to not instantiate its own server
DEBUG_TARGET="localhost" pytest vxi11
# Wait for server to exit
wait $VXI11_PID

##### Run hislip tests #####
# Start the server
./target/debug/examples/hislip --timeout $TIMEOUT &
HISLIP_PID=$!
# Run pytest and tell it to not instantiate its own server
DEBUG_TARGET="localhost" pytest hislip
# Wait for server to exit
wait $HISLIP_PID

##### Run telnet tests #####
# Start the server
./target/debug/examples/telnet --timeout $TIMEOUT &
TELNET_PID=$!
# Run pytest and tell it to not instantiate its own server
DEBUG_TARGET="localhost" pytest telnet
# Wait for server to exit
wait $TELNET_PID

##### Run socket tests #####
# Start the server
./target/debug/examples/raw --timeout $TIMEOUT &
SOCKET_PID=$!
# Run pytest and tell it to not instantiate its own server
DEBUG_TARGET="localhost" pytest raw
# Wait for server to exit
wait $SOCKET_PID

##### Generate and open report #####
cargo llvm-cov report "$@"
