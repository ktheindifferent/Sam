#!/bin/bash

# Test thread_manager with miri to detect undefined behavior
echo "Running Miri tests for thread_manager..."

# Set environment variables for miri
export MIRIFLAGS="-Zmiri-disable-isolation"

# Run miri tests specifically for the thread_manager module
cargo +nightly miri test --lib thread_manager::miri_tests 2>&1 | head -100

echo "Miri test completed. Check output for any undefined behavior."