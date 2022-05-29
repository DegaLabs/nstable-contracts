#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release
# cd ..
cp ../target/wasm32-unknown-unknown/release/np2p.wasm ../res/np2p_local.wasm
