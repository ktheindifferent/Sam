#!/bin/bash

# Alternative LibTorch Build Script for tch-rs 0.20
# Uses PyTorch 2.5.1 which has better CMake compatibility

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

# Configuration
PYTORCH_VERSION="v2.5.1"  # More stable version with better CMake compatibility
BUILD_DIR="$HOME/pytorch_build"
SOURCE_DIR="$BUILD_DIR/pytorch"
LIBTORCH_INSTALL_DIR="/usr/local/libtorch"
JOBS=$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4)

print_status "Alternative LibTorch build using PyTorch $PYTORCH_VERSION"

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

# Function to install dependencies
install_dependencies() {
    local system=$1
    print_status "Installing build dependencies for $system..."
    
    case "$system" in
        macos-*)
            # Install with Homebrew
            if ! command -v brew &> /dev/null; then
                print_error "Homebrew is required. Install from https://brew.sh/"
            fi
            
            brew install cmake ninja python@3.11 git
            # Use Python 3.11 for better compatibility
            ;;
        linux-*)
            # Install with apt (Ubuntu/Debian) or yum (CentOS/RHEL)
            if command -v apt &> /dev/null; then
                sudo apt update
                sudo apt install -y build-essential cmake ninja-build python3 python3-pip git
            elif command -v yum &> /dev/null; then
                sudo yum install -y gcc gcc-c++ cmake ninja-build python3 python3-pip git
            elif command -v dnf &> /dev/null; then
                sudo dnf install -y gcc gcc-c++ cmake ninja-build python3 python3-pip git
            else
                print_error "Unsupported Linux distribution"
            fi
            ;;
    esac
    
    # Install Python dependencies
    python3 -m pip install --user --upgrade pip setuptools wheel
    python3 -m pip install --user numpy pyyaml typing_extensions
}

# Function to clone PyTorch
clone_pytorch() {
    print_status "Cloning PyTorch $PYTORCH_VERSION..."
    
    # Clean up existing build
    if [ -d "$BUILD_DIR" ]; then
        print_warning "Removing existing build directory: $BUILD_DIR"
        rm -rf "$BUILD_DIR"
    fi
    
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    
    # Clone with shallow history for faster download
    git clone --branch "$PYTORCH_VERSION" --depth 1 https://github.com/pytorch/pytorch.git
    cd "$SOURCE_DIR"
    
    # Initialize only essential submodules
    print_status "Initializing essential submodules..."
    git submodule update --init --recursive third_party/cpuinfo
    git submodule update --init --recursive third_party/clog
    git submodule update --init --recursive third_party/pthreadpool
    git submodule update --init --recursive third_party/FXdiv
    git submodule update --init --recursive third_party/psimd
    git submodule update --init --recursive third_party/FP16
    git submodule update --init --recursive third_party/eigen
    git submodule update --init --recursive third_party/googletest
    git submodule update --init --recursive third_party/benchmark
    git submodule update --init --recursive third_party/protobuf
    git submodule update --init --recursive third_party/ios-cmake
    git submodule update --init --recursive third_party/NNPACK
    git submodule update --init --recursive third_party/fmt
}

# Function to configure build
configure_build() {
    local system=$1
    print_status "Configuring build for $system..."
    
    cd "$SOURCE_DIR"
    
    # Clean build directory
    rm -rf build
    mkdir -p build
    cd build
    
    # Set environment variables
    export BUILD_SHARED_LIBS=ON
    export BUILD_PYTHON=OFF
    export BUILD_TEST=OFF
    export USE_CUDA=OFF
    export USE_MKLDNN=ON
    export USE_OPENMP=ON
    
    # Platform-specific settings
    case "$system" in
        macos-arm64)
            export MACOSX_DEPLOYMENT_TARGET=11.0
            ;;
        macos-x86_64)
            export MACOSX_DEPLOYMENT_TARGET=10.15
            ;;
    esac
    
    # Configure with minimal options for faster build
    cmake .. \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="$LIBTORCH_INSTALL_DIR" \
        -DBUILD_SHARED_LIBS=ON \
        -DBUILD_PYTHON=OFF \
        -DBUILD_TEST=OFF \
        -DBUILD_CAFFE2=OFF \
        -DUSE_CUDA=OFF \
        -DUSE_CUDNN=OFF \
        -DUSE_MKLDNN=ON \
        -DUSE_OPENMP=ON \
        -DUSE_DISTRIBUTED=OFF \
        -DUSE_MPI=OFF \
        -DUSE_NCCL=OFF \
        -DUSE_NNPACK=OFF \
        -DUSE_QNNPACK=OFF \
        -DUSE_XNNPACK=OFF \
        -GNinja
}

# Function to build
build_libtorch() {
    print_status "Building LibTorch (this may take 30-45 minutes)..."
    
    cd "$SOURCE_DIR/build"
    
    # Build with limited parallelism to avoid memory issues
    local limited_jobs=$((JOBS < 4 ? JOBS : 4))
    ninja -j$limited_jobs
    
    print_success "LibTorch build completed!"
}

# Function to install
install_libtorch() {
    print_status "Installing LibTorch to $LIBTORCH_INSTALL_DIR..."
    
    cd "$SOURCE_DIR/build"
    
    # Remove existing installation
    if [ -d "$LIBTORCH_INSTALL_DIR" ]; then
        print_warning "Removing existing LibTorch installation..."
        sudo rm -rf "$LIBTORCH_INSTALL_DIR"
    fi
    
    # Install
    sudo ninja install
    
    # Fix permissions
    sudo chown -R $(whoami):$(id -gn) "$LIBTORCH_INSTALL_DIR" 2>/dev/null || true
    
    print_success "LibTorch installed to $LIBTORCH_INSTALL_DIR"
}

# Function to setup environment
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
# LibTorch Environment (PyTorch $PYTORCH_VERSION - Added by build_libtorch_alt.sh)
export LIBTORCH=\"$LIBTORCH_INSTALL_DIR\"
export LD_LIBRARY_PATH=\"$LIBTORCH_INSTALL_DIR/lib:\$LD_LIBRARY_PATH\"
"
    
    # Add macOS-specific variables
    if [[ "$system" == macos-* ]]; then
        env_vars+="export DYLD_LIBRARY_PATH=\"$LIBTORCH_INSTALL_DIR/lib:\$DYLD_LIBRARY_PATH\"
"
    fi
    
    # Check if variables are already in shell config
    if ! grep -q "LIBTORCH=" "$SHELL_RC" 2>/dev/null; then
        echo "$env_vars" >> "$SHELL_RC"
        print_status "Added environment variables to $SHELL_RC"
    else
        # Update existing entries
        print_warning "Updating existing environment variables in $SHELL_RC"
        sed -i.bak '/# LibTorch Environment/,+3d' "$SHELL_RC"
        echo "$env_vars" >> "$SHELL_RC"
    fi
    
    # Export for current session
    export LIBTORCH="$LIBTORCH_INSTALL_DIR"
    export LD_LIBRARY_PATH="$LIBTORCH_INSTALL_DIR/lib:$LD_LIBRARY_PATH"
    if [[ "$system" == macos-* ]]; then
        export DYLD_LIBRARY_PATH="$LIBTORCH_INSTALL_DIR/lib:$DYLD_LIBRARY_PATH"
    fi
}

# Function to verify installation
verify_installation() {
    print_status "Verifying LibTorch installation..."
    
    # Check if library files exist
    if [[ "$SYSTEM" == macos-* ]]; then
        LIB_EXT="dylib"
    else
        LIB_EXT="so"
    fi
    
    if [ ! -f "$LIBTORCH_INSTALL_DIR/lib/libtorch.$LIB_EXT" ]; then
        print_error "LibTorch library not found at $LIBTORCH_INSTALL_DIR/lib/libtorch.$LIB_EXT"
    fi
    
    if [ ! -f "$LIBTORCH_INSTALL_DIR/include/torch/torch.h" ]; then
        print_error "LibTorch headers not found at $LIBTORCH_INSTALL_DIR/include/torch/torch.h"
    fi
    
    print_success "LibTorch installation verified!"
}

# Function to cleanup
cleanup() {
    if [[ "$1" != "--keep-build" ]]; then
        print_status "Cleaning up build directory..."
        rm -rf "$BUILD_DIR"
    fi
}

# Main function
main() {
    local keep_build=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --keep-build)
                keep_build=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [options]"
                echo "  --keep-build       Keep the build directory after installation"
                echo "  --help, -h         Show this help"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                ;;
        esac
    done
    
    print_status "Alternative LibTorch source build for tch-rs 0.20..."
    print_status "Using PyTorch $PYTORCH_VERSION for better CMake compatibility"
    
    # Detect system
    local system=$(detect_system)
    export SYSTEM="$system"
    print_status "Detected system: $system"
    
    # Check for existing installation
    if [ -d "$LIBTORCH_INSTALL_DIR" ]; then
        print_warning "Existing LibTorch installation found at $LIBTORCH_INSTALL_DIR"
        read -p "Do you want to continue and replace it? (y/N): " confirm
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            print_status "Installation cancelled."
            exit 0
        fi
    fi
    
    # Install dependencies
    install_dependencies "$system"
    
    # Clone PyTorch
    clone_pytorch
    
    # Configure build
    configure_build "$system"
    
    # Build LibTorch
    build_libtorch
    
    # Install LibTorch
    install_libtorch
    
    # Setup environment
    setup_environment "$system"
    
    # Verify installation
    verify_installation
    
    # Cleanup
    if [ "$keep_build" = true ]; then
        cleanup --keep-build
    else
        cleanup
    fi
    
    print_success "Alternative LibTorch build completed successfully!"
    print_status "You may need to restart your terminal or run 'source ~/.zshrc' to load environment variables."
    print_status "You can now build your Rust project with: cargo build --features nst"
}

# Run main function
main "$@"
