#!/bin/bash

# PyTorch/LibTorch Installation Script for tch-rs 0.16
# This script installs LibTorch compatible with tch-rs 0.16.0
# Supports macOS (Intel and Apple Silicon), Linux, and Windows (WSL)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PYTORCH_VERSION="2.1.2"  # Last version with proper macOS binaries, compatible with tch-rs 0.14-0.15
INSTALL_DIR="/usr/local/libtorch"
TEMP_DIR="/tmp/libtorch_install"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Function to detect OS and architecture
detect_system() {
    OS=""
    ARCH=""
    
    case "$(uname -s)" in
        Darwin)
            OS="macos"
            ;;
        Linux)
            OS="linux"
            ;;
        CYGWIN*|MINGW32*|MSYS*|MINGW*)
            OS="windows"
            ;;
        *)
            print_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
    
    case "$(uname -m)" in
        x86_64)
            ARCH="x64"
            ;;
        arm64|aarch64)
            ARCH="arm64"
            ;;
        *)
            print_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    
    print_status "Detected: $OS-$ARCH"
}

# Function to determine download URL
get_download_url() {
    local base_url="https://download.pytorch.org/libtorch"
    local filename=""
    
    if [[ "$OS" == "macos" ]]; then
        # PyTorch 2.2.2 - use the standard shared-with-deps version for macOS
        filename="libtorch-shared-with-deps-${PYTORCH_VERSION}%2Bcpu.zip"
        DOWNLOAD_URL="${base_url}/cpu/${filename}"
    elif [[ "$OS" == "linux" ]]; then
        filename="libtorch-cxx11-abi-shared-with-deps-${PYTORCH_VERSION}%2Bcpu.zip"
        DOWNLOAD_URL="${base_url}/cpu/${filename}"
    elif [[ "$OS" == "windows" ]]; then
        filename="libtorch-win-shared-with-deps-${PYTORCH_VERSION}%2Bcpu.zip"
        DOWNLOAD_URL="${base_url}/cpu/${filename}"
    fi
    
    print_status "Download URL: $DOWNLOAD_URL"
}

# Function to check if PyTorch is already installed
check_existing_installation() {
    if [[ -d "$INSTALL_DIR" ]]; then
        print_warning "LibTorch directory already exists at $INSTALL_DIR"
        read -p "Do you want to remove it and reinstall? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_status "Removing existing installation..."
            sudo rm -rf "$INSTALL_DIR"
        else
            print_status "Keeping existing installation. Checking version..."
            if [[ -f "$INSTALL_DIR/build-version" ]]; then
                existing_version=$(cat "$INSTALL_DIR/build-version" 2>/dev/null || echo "unknown")
                print_status "Existing version: $existing_version"
            fi
            return 1
        fi
    fi
    return 0
}

# Function to install dependencies
install_dependencies() {
    print_status "Installing dependencies..."
    
    if [[ "$OS" == "macos" ]]; then
        # Check if Homebrew is installed
        if ! command -v brew &> /dev/null; then
            print_warning "Homebrew not found. Installing Homebrew..."
            /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        fi
        
        # Install dependencies
        brew install wget unzip cmake
        
        # For Apple Silicon, ensure we have the right tools
        if [[ "$ARCH" == "arm64" ]]; then
            print_status "Detected Apple Silicon (M1/M2). Installing additional dependencies..."
            # Make sure we have the command line tools
            xcode-select --install 2>/dev/null || true
        fi
        
    elif [[ "$OS" == "linux" ]]; then
        # Detect package manager
        if command -v apt-get &> /dev/null; then
            sudo apt-get update
            sudo apt-get install -y wget unzip cmake build-essential
        elif command -v yum &> /dev/null; then
            sudo yum install -y wget unzip cmake gcc gcc-c++
        elif command -v pacman &> /dev/null; then
            sudo pacman -S wget unzip cmake base-devel
        else
            print_warning "Unknown package manager. Please install wget, unzip, and cmake manually."
        fi
    fi
}

# Function to download and extract LibTorch
download_and_extract() {
    print_status "Creating temporary directory..."
    mkdir -p "$TEMP_DIR"
    cd "$TEMP_DIR"
    
    print_status "Downloading LibTorch ${PYTORCH_VERSION}..."
    if ! wget -O libtorch.zip "$DOWNLOAD_URL"; then
        print_error "Failed to download LibTorch. Trying alternative URL..."
        # Try alternative URL without version encoding
        local alt_url="${DOWNLOAD_URL//%2B/+}"
        if ! wget -O libtorch.zip "$alt_url"; then
            print_error "Failed to download from alternative URL as well."
            print_error "Please check your internet connection and try again."
            exit 1
        fi
    fi
    
    print_status "Extracting LibTorch..."
    unzip -q libtorch.zip
    
    if [[ ! -d "libtorch" ]]; then
        print_error "Extraction failed or libtorch directory not found"
        exit 1
    fi
}

# Function to install LibTorch
install_libtorch() {
    print_status "Installing LibTorch to $INSTALL_DIR..."
    
    # Create install directory
    sudo mkdir -p "$INSTALL_DIR"
    
    # Copy files
    sudo cp -r libtorch/* "$INSTALL_DIR/"
    
    # Set proper permissions
    sudo chmod -R 755 "$INSTALL_DIR"
    
    # Create version file
    echo "$PYTORCH_VERSION" | sudo tee "$INSTALL_DIR/build-version" > /dev/null
}

# Function to set up environment variables
setup_environment() {
    print_status "Setting up environment variables..."
    
    local shell_rc=""
    if [[ "$SHELL" == *"zsh"* ]]; then
        shell_rc="$HOME/.zshrc"
    elif [[ "$SHELL" == *"bash"* ]]; then
        shell_rc="$HOME/.bashrc"
    else
        shell_rc="$HOME/.profile"
    fi
    
    # Create backup of shell rc
    if [[ -f "$shell_rc" ]]; then
        cp "$shell_rc" "${shell_rc}.backup.$(date +%Y%m%d_%H%M%S)"
    fi
    
    # Add environment variables
    cat >> "$shell_rc" << EOF

# LibTorch environment variables (added by PyTorch install script)
export LIBTORCH="$INSTALL_DIR"
export LD_LIBRARY_PATH="\$LIBTORCH/lib:\$LD_LIBRARY_PATH"
EOF

    if [[ "$OS" == "macos" ]]; then
        cat >> "$shell_rc" << EOF
export DYLD_LIBRARY_PATH="\$LIBTORCH/lib:\$DYLD_LIBRARY_PATH"
EOF
    fi
    
    # Export for current session
    export LIBTORCH="$INSTALL_DIR"
    export LD_LIBRARY_PATH="$LIBTORCH/lib:$LD_LIBRARY_PATH"
    
    if [[ "$OS" == "macos" ]]; then
        export DYLD_LIBRARY_PATH="$LIBTORCH/lib:$DYLD_LIBRARY_PATH"
    fi
    
    print_success "Environment variables added to $shell_rc"
    print_warning "Please run 'source $shell_rc' or restart your terminal to apply changes"
}

# Function to verify installation
verify_installation() {
    print_status "Verifying installation..."
    
    # Check if directory exists
    if [[ ! -d "$INSTALL_DIR" ]]; then
        print_error "Installation directory not found"
        return 1
    fi
    
    # Check for required files - modern PyTorch uses .so files even on macOS
    local required_files=(
        "lib/libtorch.so"
        "lib/libtorch_cpu.so" 
        "lib/libc10.so"
        "include/torch/csrc/api/include/torch/torch.h"
    )
    
    if [[ "$OS" == "windows" ]]; then
        required_files=(
            "lib/torch.dll"
            "lib/torch_cpu.dll"
            "lib/c10.dll"
            "include/torch/csrc/api/include/torch/torch.h"
        )
    fi
    
    local missing_files=()
    for file in "${required_files[@]}"; do
        if [[ ! -f "$INSTALL_DIR/$file" ]]; then
            missing_files+=("$file")
        fi
    done
    
    if [[ ${#missing_files[@]} -gt 0 ]]; then
        print_error "Missing required files:"
        printf '%s\n' "${missing_files[@]}"
        return 1
    fi
    
    # On macOS, create .dylib symlinks for compatibility with older build systems
    if [[ "$OS" == "macos" ]]; then
        print_status "Creating macOS compatibility symlinks..."
        cd "$INSTALL_DIR/lib"
        
        # Create .dylib symlinks if they don't exist
        local dylib_links=(
            "libtorch.dylib:libtorch.so"
            "libtorch_cpu.dylib:libtorch_cpu.so"
            "libc10.dylib:libc10.so"
        )
        
        for link_pair in "${dylib_links[@]}"; do
            local dylib_name="${link_pair%%:*}"
            local so_name="${link_pair##*:}"
            
            if [[ -f "$so_name" && ! -f "$dylib_name" ]]; then
                sudo ln -sf "$so_name" "$dylib_name"
                print_status "Created symlink: $dylib_name -> $so_name"
            fi
        done
    fi
    
    print_success "Installation verified successfully!"
    return 0
}

# Function to clean up
cleanup() {
    print_status "Cleaning up temporary files..."
    rm -rf "$TEMP_DIR"
}

# Function to show usage information
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -v, --version VERSION    PyTorch version to install (default: $PYTORCH_VERSION)"
    echo "  -d, --dir DIRECTORY      Installation directory (default: $INSTALL_DIR)"
    echo "  -h, --help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                      # Install with defaults"
    echo "  $0 -v 2.2.0            # Install specific version"
    echo "  $0 -d /opt/libtorch     # Install to custom directory"
}

# Main installation function
main() {
    print_status "PyTorch/LibTorch Installation Script for tch-rs 0.16"
    print_status "=================================================="
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--version)
                PYTORCH_VERSION="$2"
                shift 2
                ;;
            -d|--dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    # Perform installation steps
    detect_system
    get_download_url
    
    if ! check_existing_installation; then
        print_status "Skipping installation - using existing LibTorch"
        setup_environment
        verify_installation && print_success "Setup completed successfully!"
        exit 0
    fi
    
    install_dependencies
    download_and_extract
    install_libtorch
    setup_environment
    
    if verify_installation; then
        print_success "LibTorch ${PYTORCH_VERSION} installed successfully!"
        print_status "Installation directory: $INSTALL_DIR"
        print_warning "Please restart your terminal or run 'source ~/.zshrc' to apply environment changes"
        print_status "You can now build your Rust project with tch-rs support"
    else
        print_error "Installation verification failed"
        exit 1
    fi
    
    cleanup
}

# Trap to ensure cleanup on script exit
trap cleanup EXIT

# Run main function
main "$@"
