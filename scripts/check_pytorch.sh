#!/bin/bash

# Quick PyTorch/LibTorch installation check
# This script verifies if PyTorch is properly installed and configured

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_status "Checking PyTorch/LibTorch installation..."

# Check environment variables
if [[ -n "$LIBTORCH" ]]; then
    print_status "LIBTORCH environment variable set to: $LIBTORCH"
    
    if [[ -d "$LIBTORCH" ]]; then
        print_success "LibTorch directory exists"
        
        # Check for required files
        if [[ -f "$LIBTORCH/lib/libtorch.dylib" ]] || [[ -f "$LIBTORCH/lib/libtorch.so" ]] || [[ -f "$LIBTORCH/lib/torch.dll" ]]; then
            print_success "LibTorch library files found"
        else
            print_error "LibTorch library files not found in $LIBTORCH/lib/"
            exit 1
        fi
        
        # Check for header files in different possible locations
        header_found=false
        header_locations=(
            "$LIBTORCH/include/torch/torch.h"
            "$LIBTORCH/include/torch/csrc/api/include/torch/torch.h"
        )
        
        for header_path in "${header_locations[@]}"; do
            if [[ -f "$header_path" ]]; then
                print_success "LibTorch header files found at: $header_path"
                header_found=true
                break
            fi
        done
        
        if [[ "$header_found" != "true" ]]; then
            print_error "LibTorch header files not found in expected locations"
            print_status "Searched in: ${header_locations[*]}"
            exit 1
        fi
    else
        print_error "LibTorch directory does not exist: $LIBTORCH"
        exit 1
    fi
else
    print_warning "LIBTORCH environment variable not set"
    
    # Check for system-wide installation
    if [[ -f "/usr/local/libtorch/lib/libtorch.dylib" ]] || [[ -f "/usr/local/libtorch/lib/libtorch.so" ]]; then
        print_status "Found system-wide LibTorch installation"
        export LIBTORCH="/usr/local/libtorch"
    elif [[ -f "/usr/lib/libtorch.so" ]]; then
        print_status "Found system LibTorch installation"
        export LIBTORCH="/usr"
    else
        print_error "No LibTorch installation found"
        print_status "Please run: ./scripts/install_pytorch.sh"
        exit 1
    fi
fi

# Check Python PyTorch if LIBTORCH_USE_PYTORCH is set
if [[ "$LIBTORCH_USE_PYTORCH" == "1" ]]; then
    print_status "Checking Python PyTorch installation..."
    
    if command -v python3 &> /dev/null; then
        if python3 -c "import torch; print(f'PyTorch version: {torch.__version__}')" 2>/dev/null; then
            print_success "Python PyTorch found and working"
        else
            print_error "Python PyTorch not found or not working"
            print_status "Install with: pip install torch"
            exit 1
        fi
    else
        print_error "Python3 not found"
        exit 1
    fi
fi

# Test compilation
print_status "Testing Rust compilation with tch-rs..."

if cargo check --features nst --quiet 2>/dev/null; then
    print_success "Rust compilation successful with NST feature"
else
    print_error "Rust compilation failed"
    print_status "Run 'cargo check --features nst' for detailed error information"
    exit 1
fi

print_success "PyTorch/LibTorch installation check completed successfully!"
print_status "You can now build the project with: cargo build --features nst"
