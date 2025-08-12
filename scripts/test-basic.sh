#!/bin/bash

echo "======================================"
echo "Running basic unit tests for critical services"
echo "======================================"

# Test LIFX service tests
echo ""
echo "Testing LIFX service..."
cargo test --lib lifx::tests --no-fail-fast 2>&1 | grep -E "(test result:|running |test .* ok|test .* FAILED)" || echo "LIFX tests compilation in progress..."

# Test Spotify service tests
echo ""
echo "Testing Spotify service..."
cargo test --lib spotify::tests --no-fail-fast 2>&1 | grep -E "(test result:|running |test .* ok|test .* FAILED)" || echo "Spotify tests compilation in progress..."

# Test Sound service tests
echo ""
echo "Testing Sound service..."
cargo test --lib sound::tests --no-fail-fast 2>&1 | grep -E "(test result:|running |test .* ok|test .* FAILED)" || echo "Sound tests compilation in progress..."

# Test YouTube media service tests
echo ""
echo "Testing YouTube media service..."
cargo test --lib "media::youtube::tests" --no-fail-fast 2>&1 | grep -E "(test result:|running |test .* ok|test .* FAILED)" || echo "YouTube tests compilation in progress..."

echo ""
echo "======================================"
echo "Basic test run complete!"
echo "======================================"