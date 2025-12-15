#!/bin/sh
cargo build --target=wasm32-unknown-unknown
wasm-bindgen --target web --out-dir docs --no-typescript target/wasm32-unknown-unknown/debug/langame.wasm
