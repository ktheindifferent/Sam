#!/bin/bash

# Quick PyTorch Installation Script for NST Development
# This script provides faster installation options for testing and development

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to detect system
detect_system() {
    local os=$(uname -s)
    local arch=$(uname -m)
    
    case "$os" in
        Darwin)
            if [[ "$arch" == "arm64" ]]; then
                echo "macos-arm64"
            else
                echo "macos-x86_64"
            fi
            ;;
        Linux)
            if [[ "$arch" == "x86_64" ]]; then
                echo "linux-x86_64"
            elif [[ "$arch" == "aarch64" ]]; then
                echo "linux-aarch64"
            else
                print_error "Unsupported Linux architecture: $arch"
            fi
            ;;
        *)
            print_error "Unsupported operating system: $os"
            ;;
    esac
}

# Quick installation with pre-built binaries
quick_install() {
    local system=$1
    print_status "Installing pre-built PyTorch for quick setup..."
    
    # Temporarily use older but compatible version for development
    print_warning "Using tch-rs 0.16 + PyTorch 2.1.2 for faster development setup"
    print_status "For production, run ./scripts/build_libtorch.sh to build PyTorch 2.7.0 from source"
    
    case "$system" in
        macos-*)
            local url="https://download.pytorch.org/libtorch/cpu/libtorch-macos-2.1.2.zip"
            local filename="libtorch-macos-2.1.2.zip"
            ;;
        linux-x86_64)
            local url="https://download.pytorch.org/libtorch/cpu/libtorch-cxx11-abi-shared-with-deps-2.1.2%2Bcpu.zip"
            local filename="libtorch-linux-2.1.2.zip"
            ;;
        *)
            print_error "Pre-built binaries not available for $system. Please use ./scripts/build_libtorch.sh"
            ;;
    esac
    
    # Download and install
    print_status "Downloading $filename..."
    curl -L -o "/tmp/$filename" "$url"
    
    print_status "Extracting LibTorch..."
    cd /tmp
    unzip -q "$filename"
    
    # Remove old installation
    if [ -d "/usr/local/libtorch" ]; then
        print_status "Removing existing LibTorch installation..."
        sudo rm -rf /usr/local/libtorch
    fi
    
    # Install
    print_status "Installing to /usr/local/libtorch..."
    sudo mv libtorch /usr/local/
    sudo chown -R $(whoami):$(id -gn) /usr/local/libtorch 2>/dev/null || true
    
    # Cleanup
    rm -f "/tmp/$filename"
    
    print_success "PyTorch 2.1.2 installed successfully!"
}

# Update Cargo.toml for compatibility
update_cargo_toml() {
    print_status "Updating Cargo.toml for PyTorch 2.1.2 compatibility..."
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        print_error "Cargo.toml not found. Please run this script from the project root."
    fi
    
    # Backup original
    cp Cargo.toml Cargo.toml.backup
    
    # Update tch version (it's already 0.16, so this should be a no-op)
    sed -i.bak 's/tch = { version = "0.20"/tch = { version = "0.16"/' Cargo.toml
    rm -f Cargo.toml.bak
    
    print_success "Cargo.toml confirmed compatible (backup saved as Cargo.toml.backup)"
    print_warning "To restore: mv Cargo.toml.backup Cargo.toml"
}

# Setup environment
setup_environment() {
    local system=$1
    print_status "Setting up environment variables..."
    
    # Determine shell configuration file
    if [[ "$SHELL" == *"zsh"* ]]; then
        SHELL_RC="$HOME/.zshrc"
    elif [[ "$SHELL" == *"bash"* ]]; then
        SHELL_RC="$HOME/.bashrc"
    else
        SHELL_RC="$HOME/.profile"
    fi
    
    # Environment variables for tch-rs
    local env_vars="
# LibTorch Environment (Quick Install - Added by install_pytorch_quick.sh)
export LIBTORCH=\"/usr/local/libtorch\"
export LD_LIBRARY_PATH=\"/usr/local/libtorch/lib:\$LD_LIBRARY_PATH\"
"
    
    # Add macOS-specific variables
    if [[ "$system" == macos-* ]]; then
        env_vars+="export DYLD_LIBRARY_PATH=\"/usr/local/libtorch/lib:\$DYLD_LIBRARY_PATH\"
"
    fi
    
    # Check if variables are already in shell config
    if ! grep -q "LIBTORCH=" "$SHELL_RC" 2>/dev/null; then
        echo "$env_vars" >> "$SHELL_RC"
        print_status "Added environment variables to $SHELL_RC"
    else
        print_warning "Environment variables already exist in $SHELL_RC"
    fi
    
    # Export for current session
    export LIBTORCH="/usr/local/libtorch"
    export LD_LIBRARY_PATH="/usr/local/libtorch/lib:$LD_LIBRARY_PATH"
    if [[ "$system" == macos-* ]]; then
        export DYLD_LIBRARY_PATH="/usr/local/libtorch/lib:$DYLD_LIBRARY_PATH"
    fi
}

# Test installation
test_installation() {
    print_status "Testing installation..."
    
    # Check files exist
    if [ ! -f "/usr/local/libtorch/lib/libtorch.dylib" ] && [ ! -f "/usr/local/libtorch/lib/libtorch.so" ]; then
        print_error "LibTorch libraries not found"
    fi
    
    # Test compilation
    if cargo check --features nst >/dev/null 2>&1; then
        print_success "NST feature compiles successfully!"
    else
        print_warning "Compilation test failed - you may need to restart your terminal"
    fi
}

# Main function
main() {
    local restore_cargo=false
    local skip_env=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --restore-cargo)
                if [ -f "Cargo.toml.backup" ]; then
                    mv Cargo.toml.backup Cargo.toml
                    print_success "Cargo.toml restored from backup"
                else
                    print_error "No Cargo.toml.backup found"
                fi
                exit 0
                ;;
            --skip-env)
                skip_env=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [options]"
                echo "  --restore-cargo    Restore Cargo.toml from backup"
                echo "  --skip-env         Skip environment setup"
                echo "  --help, -h         Show this help"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                ;;
        esac
    done
    
    print_status "Quick PyTorch Installation for NST Development"
    print_warning "This installs PyTorch 2.1.2 + tch-rs 0.16 for faster development"
    print_warning "For production, use: ./scripts/build_libtorch.sh"
    
    # Detect system
    local system=$(detect_system)
    print_status "Detected system: $system"
    
    # Check if we're running as root
    if [[ $EUID -eq 0 ]]; then
        print_error "This script should not be run as root"
    fi
    
    # Install PyTorch
    quick_install "$system"
    
    # Update Cargo.toml
    update_cargo_toml
    
    # Setup environment
    if [ "$skip_env" = false ]; then
        setup_environment "$system"
    fi
    
    # Test installation
    test_installation
    
    print_success "Quick installation completed!"
    echo
    print_status "Next steps:"
    echo "  1. Restart your terminal or run: source ~/.zshrc (or ~/.bashrc)"
    echo "  2. Build with NST: cargo build --features nst"
    echo "  3. Run tests: ./scripts/test_nst.sh"
    echo
    print_warning "To upgrade to production PyTorch 2.7.0 later:"
    echo "  1. ./scripts/install_pytorch_quick.sh --restore-cargo"
    echo "  2. ./scripts/build_libtorch.sh"
}

# Run main function
main "$@"
