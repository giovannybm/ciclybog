#!/usr/bin/env bash
set -euo pipefail

mkdir -p public/wasm
cargo build --release --target wasm32-unknown-unknown --manifest-path rust/Cargo.toml -p ciclybog-router-wasm
wasm-bindgen rust/target/wasm32-unknown-unknown/release/ciclybog_router_wasm.wasm --target web --out-dir public/wasm
