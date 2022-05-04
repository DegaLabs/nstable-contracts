#!/bin/bash
set -e
rustup target add wasm32-unknown-unknown
# RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release
# cd ..
cp ../target/wasm32-unknown-unknown/release/nstable_farming_v2.wasm ../res/nstable_farming_v2_local.wasm
