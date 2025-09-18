#!/bin/bash

# LibTorch Source Build Script for tch-rs 0.20
# This script builds LibTorch from source for maximum compatibility
# Supports macOS (Intel and Apple Silicon), Linux, and Windows (WSL)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PYTORCH_VERSION="v2.7.0"  # Compatible with tch-rs 0.20
LIBTORCH_INSTALL_DIR="/usr/local/libtorch"
BUILD_DIR="$HOME/pytorch_build"
SOURCE_DIR="$BUILD_DIR/pytorch"
JOBS=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
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
        MINGW*|CYGWIN*|MSYS*)
            echo "windows-x86_64"
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
            # Check if Homebrew is installed
            if ! command -v brew &> /dev/null; then
                print_error "Homebrew is required but not installed. Please install Homebrew first."
            fi
            
            # Install dependencies
            brew install cmake ninja python3 git
            
            # Install PyTorch dependencies
            python3 -m pip install --upgrade pip setuptools wheel
            python3 -m pip install numpy pyyaml typing_extensions
            ;;
        linux-*)
            # Detect package manager and install dependencies
            if command -v apt-get &> /dev/null; then
                sudo apt-get update
                sudo apt-get install -y \
                    build-essential \
                    cmake \
                    ninja-build \
                    python3 \
                    python3-pip \
                    python3-dev \
                    git \
                    libblas-dev \
                    liblapack-dev \
                    libopenblas-dev
            elif command -v yum &> /dev/null; then
                sudo yum install -y \
                    gcc \
                    gcc-c++ \
                    cmake \
                    ninja-build \
                    python3 \
                    python3-pip \
                    python3-devel \
                    git \
                    openblas-devel \
                    lapack-devel
            elif command -v pacman &> /dev/null; then
                sudo pacman -S --noconfirm \
                    base-devel \
                    cmake \
                    ninja \
                    python \
                    python-pip \
                    git \
                    blas \
                    lapack \
                    openblas
            else
                print_error "No supported package manager found (apt, yum, or pacman)"
            fi
            
            python3 -m pip install --upgrade pip setuptools wheel
            python3 -m pip install numpy pyyaml typing_extensions
            ;;
        windows-*)
            print_error "Windows build from source not yet supported. Please use WSL."
            ;;
    esac
}

# Function to clone PyTorch repository
clone_pytorch() {
    print_status "Cloning PyTorch repository..."
    
    # Clean up any existing build directory
    if [ -d "$BUILD_DIR" ]; then
        print_warning "Removing existing build directory: $BUILD_DIR"
        rm -rf "$BUILD_DIR"
    fi
    
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    
    # Clone PyTorch with submodules
    git clone --recursive --branch "$PYTORCH_VERSION" --depth 1 https://github.com/pytorch/pytorch.git
    
    if [ ! -d "$SOURCE_DIR" ]; then
        print_error "Failed to clone PyTorch repository"
    fi
    
    cd "$SOURCE_DIR"
    
    # Update submodules
    print_status "Updating submodules..."
    git submodule sync
    git submodule update --init --recursive
}

# Function to patch CMake files for compatibility
patch_cmake_files() {
    print_status "Patching CMake files for CMake 4.x compatibility..."
    
    cd "$SOURCE_DIR"
    
    # Patch main CMakeLists.txt
    if [ -f "CMakeLists.txt" ]; then
        print_status "Patching main CMakeLists.txt..."
        # Replace the cmake_minimum_required version
        sed -i.bak 's/cmake_minimum_required(VERSION [0-9.]*)/cmake_minimum_required(VERSION 3.5)/' CMakeLists.txt
    fi
    
    # Patch protobuf CMakeLists.txt which often causes issues
    if [ -f "third_party/protobuf/CMakeLists.txt" ]; then
        print_status "Patching protobuf CMakeLists.txt..."
        sed -i.bak 's/cmake_minimum_required(VERSION [0-9.]*)/cmake_minimum_required(VERSION 3.5)/' third_party/protobuf/CMakeLists.txt
    fi
    
    # Patch other problematic submodules
    find third_party -name "CMakeLists.txt" -exec grep -l "cmake_minimum_required.*[12]\." {} \; | while read file; do
        print_status "Patching $file..."
        sed -i.bak 's/cmake_minimum_required(VERSION [12]\.[0-9.]*)/cmake_minimum_required(VERSION 3.5)/' "$file"
    done
    
    # Fix CMake syntax issues - 'or' should be 'OR' in if statements
    if [ -f "cmake/Dependencies.cmake" ]; then
        print_status "Fixing CMake syntax in Dependencies.cmake..."
        # Fix lowercase 'or' to uppercase 'OR' in if statements
        sed -i.bak 's/ or / OR /g' "cmake/Dependencies.cmake"
        sed -i.bak 's/ and / AND /g' "cmake/Dependencies.cmake"
    fi
    
    print_success "CMake files patched successfully"
}

# Function to configure build
configure_build() {
    local system=$1
    print_status "Configuring build for $system..."
    
    cd "$SOURCE_DIR"
    
    # Clean any existing build directory in source
    if [ -d "build" ]; then
        print_warning "Removing existing build directory..."
        rm -rf build
    fi
    
    # Set environment variables for LibTorch C++ build
    export BUILD_SHARED_LIBS=ON
    export BUILD_PYTHON=OFF
    export BUILD_BINARY=ON
    export BUILD_TEST=OFF
    export BUILD_CAFFE2_OPS=OFF
    export USE_OPENCV=OFF
    export USE_CUDA=OFF
    export USE_CUDNN=OFF
    export USE_MKLDNN=ON
    export USE_OPENMP=ON
    
    # Set CMake policy environment variables for all submodules
    export CMAKE_POLICY_VERSION_MINIMUM=3.5
    export CMAKE_POLICY_DEFAULT_CMP0091=NEW
    export CMAKE_POLICY_DEFAULT_CMP0092=NEW
    
    # Platform-specific configurations
    case "$system" in
        macos-arm64)
            export MACOSX_DEPLOYMENT_TARGET=11.0
            export CMAKE_OSX_ARCHITECTURES=arm64
            ;;
        macos-x86_64)
            export MACOSX_DEPLOYMENT_TARGET=10.15
            export CMAKE_OSX_ARCHITECTURES=x86_64
            ;;
        linux-*)
            export USE_SYSTEM_EIGEN_INSTALL=OFF
            ;;
    esac
    
    # Create build directory
    mkdir -p build
    cd build
    
    # Configure with CMake
    cmake .. \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="$LIBTORCH_INSTALL_DIR" \
        -DCMAKE_POLICY_VERSION_MINIMUM=3.5 \
        -DCMAKE_POLICY_DEFAULT_CMP0091=NEW \
        -DCMAKE_POLICY_DEFAULT_CMP0092=NEW \
        -DCMAKE_CXX_STANDARD=17 \
        -DCMAKE_C_COMPILER=/usr/bin/clang \
        -DCMAKE_CXX_COMPILER=/usr/bin/clang++ \
        -DPYTHON_EXECUTABLE=/usr/bin/python3 \
        -DPython_EXECUTABLE=/usr/bin/python3 \
        -DPython3_EXECUTABLE=/usr/bin/python3 \
        -DBUILD_SHARED_LIBS=ON \
        -DBUILD_PYTHON=OFF \
        -DBUILD_BINARY=ON \
        -DBUILD_TEST=OFF \
        -DBUILD_CAFFE2_OPS=OFF \
        -DUSE_OPENCV=OFF \
        -DUSE_CUDA=OFF \
        -DUSE_CUDNN=OFF \
        -DUSE_MKLDNN=ON \
        -DUSE_OPENMP=OFF \
        -DUSE_SYSTEM_EIGEN_INSTALL=OFF \
        -DUSE_SYSTEM_XNNPACK=OFF \
        -DUSE_XNNPACK=ON \
        -Dprotobuf_BUILD_TESTS=OFF \
        -GNinja
}

# Function to build LibTorch
build_libtorch() {
    print_status "Building LibTorch (this may take 30-60 minutes)..."
    
    cd "$SOURCE_DIR/build"
    
    # Build with ninja
    ninja -j$JOBS
    
    print_success "LibTorch build completed!"
}

# Function to install LibTorch
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
    
    print_success "LibTorch installed successfully!"
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
# LibTorch Environment (Added by build_libtorch.sh)
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
        print_warning "Environment variables already exist in $SHELL_RC"
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
    local lib_files=()
    if [[ $(detect_system) == macos-* ]]; then
        lib_files=("libtorch.dylib" "libtorch_cpu.dylib" "libc10.dylib")
    else
        lib_files=("libtorch.so" "libtorch_cpu.so" "libc10.so")
    fi
    
    for lib in "${lib_files[@]}"; do
        if [ ! -f "$LIBTORCH_INSTALL_DIR/lib/$lib" ]; then
            print_error "Missing library file: $LIBTORCH_INSTALL_DIR/lib/$lib"
        fi
    done
    
    # Check header files
    if [ ! -f "$LIBTORCH_INSTALL_DIR/include/torch/torch.h" ]; then
        print_error "Missing header file: $LIBTORCH_INSTALL_DIR/include/torch/torch.h"
    fi
    
    # Check CMake files
    if [ ! -f "$LIBTORCH_INSTALL_DIR/share/cmake/Torch/TorchConfig.cmake" ]; then
        print_error "Missing CMake config: $LIBTORCH_INSTALL_DIR/share/cmake/Torch/TorchConfig.cmake"
    fi
    
    print_success "LibTorch installation verified!"
}

# Function to clean up build directory
cleanup() {
    if [ "$1" != "--keep-build" ]; then
        print_status "Cleaning up build directory..."
        rm -rf "$BUILD_DIR"
        print_status "Build directory cleaned up"
    else
        print_status "Keeping build directory at: $BUILD_DIR"
    fi
}

# Main function
main() {
    local keep_build=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --keep-build)
                keep_build=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [--keep-build] [--help]"
                echo "  --keep-build    Keep the build directory after installation"
                echo "  --help, -h      Show this help message"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                ;;
        esac
    done
    
    print_status "Starting LibTorch source build for tch-rs 0.20..."
    
    # Detect system
    local system=$(detect_system)
    print_status "Detected system: $system"
    
    # Check if we're running as root
    if [[ $EUID -eq 0 ]]; then
        print_error "This script should not be run as root"
    fi
    
    # Install dependencies
    install_dependencies "$system"
    
    # Clone PyTorch
    clone_pytorch
    
    # Patch CMake files for compatibility
    patch_cmake_files
    
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
    
    print_success "LibTorch build and installation completed successfully!"
    print_status "You may need to restart your terminal or run 'source ~/.zshrc' (or ~/.bashrc) to load the environment variables."
    print_status "You can now build your Rust project with: cargo build --features nst"
}

# Run main function with all arguments
main "$@"
