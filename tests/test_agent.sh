#!/bin/bash
export RUST_LOG=info
cargo run --bin sam 2>&1 | grep -A5 -B5 "coding"
