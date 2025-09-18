#!/bin/bash

# Test the coding agent with a simple task
echo "Testing coding agent fix..."

# Clean up any existing test project
rm -rf test_randy 2>/dev/null

# Run the test
cargo run --bin coding_agent -- "create a new rust project called test_randy that generates a random 8 character string"

echo ""
echo "Checking if the file was created correctly:"
if [ -f "test_randy/src/main.rs" ]; then
    echo "✅ File exists at test_randy/src/main.rs"
    echo "Content:"
    cat test_randy/src/main.rs

    echo ""
    echo "Testing if it compiles:"
    cd test_randy && cargo build 2>&1 | grep -E "(Compiling|Finished|error)"

    if [ $? -eq 0 ]; then
        echo ""
        echo "Running the program:"
        cargo run --quiet
    fi
else
    echo "❌ File not found at test_randy/src/main.rs"
fi