#!/bin/bash

echo "Testing new Unix commands in Sam CLI..."
echo "========================================"

# Build the project first
echo "Building Sam..."
cargo build --release --quiet

# Test the CLI with our new commands
echo ""
echo "1. Testing touch command:"
./target/release/sam_cli_test touch /tmp/test_file.txt

echo ""
echo "2. Testing head command:"
./target/release/sam_cli_test head test_commands.txt

echo ""
echo "3. Testing tail command:"
./target/release/sam_cli_test tail test_commands.txt

echo ""
echo "4. Testing head with -n option:"
./target/release/sam_cli_test head -n 3 test_commands.txt

echo ""
echo "5. Testing tail with -n option:"
./target/release/sam_cli_test tail -n 3 test_commands.txt

echo ""
echo "6. Testing find command:"
./target/release/sam_cli_test find . -name "*.rs"

echo ""
echo "7. Testing chmod command:"
./target/release/sam_cli_test chmod 755 test_commands.txt

echo ""
echo "8. Testing chown command (may fail on some systems):"
./target/release/sam_cli_test chown user:group test_commands.txt

echo ""
echo "All tests completed!"
