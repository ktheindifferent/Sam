#!/bin/bash

# Test coverage script for SAM services
set -e

echo "======================================"
echo "SAM Service Test Coverage Report"
echo "======================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if cargo-tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo -e "${YELLOW}cargo-tarpaulin not found. Installing...${NC}"
    cargo install cargo-tarpaulin
fi

# Clean previous coverage reports
echo "Cleaning previous coverage reports..."
rm -rf target/coverage
mkdir -p target/coverage

# Run unit tests with coverage
echo ""
echo "Running unit tests with coverage..."
echo "======================================"
cargo tarpaulin --config tarpaulin.toml --profile default

# Generate coverage report for critical services
echo ""
echo "Generating coverage report for critical services..."
echo "======================================"
cargo tarpaulin --config tarpaulin.toml --profile critical_services || true

# Run property-based tests
echo ""
echo "Running property-based tests..."
echo "======================================"
cargo test --lib -- --test-threads=4

# Run integration tests
echo ""
echo "Running integration tests..."
echo "======================================"
cargo test --test '*' -- --test-threads=1

# Generate HTML report
echo ""
echo "Coverage reports generated in target/coverage/"
echo ""

# Display coverage summary
if [ -f "target/coverage/lcov.info" ]; then
    echo "Coverage Summary:"
    echo "=================="
    
    # Extract coverage percentage from lcov.info
    if command -v lcov &> /dev/null; then
        lcov --summary target/coverage/lcov.info 2>&1 | grep -E "lines|functions|branches"
    else
        echo "Install lcov for detailed summary"
    fi
fi

# Check if coverage meets threshold
COVERAGE_THRESHOLD=80
if [ -f "target/coverage/coverage.json" ]; then
    # Parse JSON to get coverage percentage (requires jq)
    if command -v jq &> /dev/null; then
        COVERAGE=$(jq '.coverage' target/coverage/coverage.json 2>/dev/null || echo "0")
        echo ""
        echo "Overall Coverage: ${COVERAGE}%"
        
        if (( $(echo "$COVERAGE >= $COVERAGE_THRESHOLD" | bc -l) )); then
            echo -e "${GREEN}✓ Coverage meets threshold (${COVERAGE_THRESHOLD}%)${NC}"
        else
            echo -e "${YELLOW}⚠ Coverage below threshold (${COVERAGE_THRESHOLD}%)${NC}"
        fi
    fi
fi

echo ""
echo "======================================"
echo "Test coverage report complete!"
echo "Open target/coverage/index.html to view detailed HTML report"
echo "======================================" 