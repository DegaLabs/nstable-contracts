#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release
# cd ..
cp ../target/wasm32-unknown-unknown/release/ve_nft.wasm ../res/ve_nft_local.wasm
