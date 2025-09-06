#!/bin/bash
# tch.sh - Build LibTorch from source for torch-sys on macOS
# Usage: source tch.sh [version] [python_build]

set -e

# Download and use CMake 3.31.7 for Intel Mac if not already present
CMAKE_VERSION="3.31.7"
CMAKE_DIR="$HOME/cmake-$CMAKE_VERSION"
CMAKE_TAR="cmake-$CMAKE_VERSION-macos-universal.tar.gz"
CMAKE_URL="https://github.com/Kitware/CMake/releases/download/v$CMAKE_VERSION/$CMAKE_TAR"

if [ ! -d "$CMAKE_DIR" ]; then
  if [ ! -f "$HOME/$CMAKE_TAR" ]; then
    echo "Downloading CMake $CMAKE_VERSION..."
    curl -L "$CMAKE_URL" -o "$HOME/$CMAKE_TAR"
  fi
  echo "Extracting CMake $CMAKE_VERSION..."
  tar -xzf "$HOME/$CMAKE_TAR" -C "$HOME"
  mv "$HOME/cmake-$CMAKE_VERSION-macos-universal" "$CMAKE_DIR"
fi
export PATH="$CMAKE_DIR/CMake.app/Contents/bin:$PATH"
hash -r

echo "Using CMake at: $(which cmake)"
cmake --version

# Install Miniconda if not present (Intel Mac)
CONDA_DIR="$HOME/miniconda3"
if [ ! -d "$CONDA_DIR" ]; then
  echo "Installing Miniconda for Intel Mac..."
  curl -fsSL https://repo.anaconda.com/miniconda/Miniconda3-latest-MacOSX-x86_64.sh -o "$HOME/miniconda.sh"
  bash "$HOME/miniconda.sh" -b -p "$CONDA_DIR"
  rm "$HOME/miniconda.sh"
fi
export PATH="$CONDA_DIR/bin:$PATH"

# Create and activate conda environment
CONDA_ENV_NAME="torch-build"
PYTHON_VERSION="3.10"
if ! conda info --envs | grep -q "$CONDA_ENV_NAME"; then
  conda create -y -n "$CONDA_ENV_NAME" python="$PYTHON_VERSION" pip
fi
source "$CONDA_DIR/bin/activate" "$CONDA_ENV_NAME"

pip install astunparse numpy ninja pyyaml mkl mkl-include setuptools cmake cffi typing_extensions future six requests dataclasses

# Ensure conda's python is used
export PYTHON_EXECUTABLE=$(which python)
echo "Using Python at: $PYTHON_EXECUTABLE"
python --version

# Default version if not provided
LIBTORCH_VERSION="${1:-2.6.0}"
# Optional: build Python (default: y)
PYTHON_BUILD="${2:-y}"
PYTORCH_REPO="https://github.com/pytorch/pytorch.git"
BUILD_DIR="$HOME/libtorch_build"
INSTALL_DIR="$HOME/libtorch"

# Install dependencies
if ! command -v brew >/dev/null; then
  echo "Homebrew not found. Please install Homebrew first."
  exit 1
fi
brew update
brew install ninja python git || true

# Ensure Python 3.13 is linked if installed
if brew list python@3.13 >/dev/null 2>&1; then
    brew link --overwrite python@3.13 || true
fi

# Clone PyTorch repo
if [ ! -d "$BUILD_DIR" ]; then
  git clone --recursive "$PYTORCH_REPO" "$BUILD_DIR"
fi
cd "$BUILD_DIR"
git fetch --all
git checkout "v$LIBTORCH_VERSION"
git submodule sync
git submodule update --init --recursive --jobs 0

# Patch CMakeLists.txt to set CMAKE_POLICY_VERSION_MINIMUM 3.5
if ! grep -q 'CMAKE_POLICY_VERSION_MINIMUM' CMakeLists.txt; then
  sed -i.bak '1s;^;set(CMAKE_POLICY_VERSION_MINIMUM 3.5)\n;' CMakeLists.txt
  echo "Patched CMakeLists.txt with set(CMAKE_POLICY_VERSION_MINIMUM 3.5"
fi

# Clean all build directories and CMake cache before patching
find . -type d \( -name 'build' -o -name 'build-*' -o -name 'CMakeFiles' \) -exec rm -rf {} +
find . -type f -name 'CMakeCache.txt' -exec rm -f {} +

# Patch to remove -Werror and silence VLA warnings in C++
find . -name 'CMakeLists.txt' -exec sed -i.bak 's/-Werror//g' {} +
# Also patch fbgemm CMakeLists.txt specifically
find ./third_party/fbgemm -name 'CMakeLists.txt' -exec sed -i.bak 's/-Werror//g' {} +
# Add -Wno-vla-cxx-extension to CMAKE_CXX_FLAGS in fbgemm's CMakeLists.txt
find ./third_party/fbgemm -name 'CMakeLists.txt' -exec sed -i.bak '/CMAKE_CXX_FLAGS/s/"$/ -Wno-vla-cxx-extension"/' {} +
export CXXFLAGS="${CXXFLAGS} -Wno-vla-cxx-extension"

# Build LibTorch C++ library with CMake
mkdir -p build-libtorch-cpp
cd build-libtorch-cpp
cmake .. \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=ON \
  -DBUILD_PYTHON=ON \
  -DBUILD_TEST=OFF \
  -DBUILD_CAFFE2_OPS=OFF \
  -DCMAKE_INSTALL_PREFIX="$INSTALL_DIR" \
  -DCMAKE_CXX_FLAGS="-Wno-vla-cxx-extension"
cmake --build . --target install -- -j$(sysctl -n hw.ncpu)
cd ..

# Optionally build and install Python library
echo "Build Python library (BUILD_PYTHON=$PYTHON_BUILD)? [y/n]"
read -r XYZ
if [[ "$XYZ" =~ ^[Yy]$ ]]; then
    PYTHON_BUILD="y"
else
    PYTHON_BUILD="n"
fi
if [[ "$PYTHON_BUILD" =~ ^[Yy]$ ]]; then
  python -m pip install --upgrade pip setuptools wheel typing_extensions pyyaml numpy six || true
  # python3.13 -m pip install six --break-system-packages || true

  export CMAKE_PREFIX_PATH=$(python -c 'import sysconfig; print(sysconfig.get_paths()["purelib"])')

  python setup.py clean
  python setup.py install


    # Find and copy built libtorch to install dir
    LIBTORCH_BUILD_PATH=$(find build -type d -name libtorch | head -n 1)
    if [ -z "$LIBTORCH_BUILD_PATH" ]; then
        LIBTORCH_BUILD_PATH=$(find dist -type d -name libtorch | head -n 1)
    fi
    if [ -z "$LIBTORCH_BUILD_PATH" ]; then
        echo "Error: Could not find built libtorch directory after build."
    exit 1
    fi
    if [ -d "$INSTALL_DIR" ]; then
        rm -rf "$INSTALL_DIR"
    fi
        cp -r "$LIBTORCH_BUILD_PATH" "$INSTALL_DIR"


else
  echo "Python build step skipped. Copying C++ libtorch from CMake build directory."
  if [ -d "$INSTALL_DIR" ]; then
    rm -rf "$INSTALL_DIR"
  fi
  if [ -d "$BUILD_DIR/build-libtorch-cpp/lib" ]; then
    cp -r "$BUILD_DIR/build-libtorch-cpp/lib" "$INSTALL_DIR"
    echo "Copied C++ libtorch to $INSTALL_DIR."
  else
    echo "Error: Could not find C++ libtorch at $BUILD_DIR/build-libtorch-cpp/libtorch."
    exit 1
  fi
fi

# Export environment variables
export LIBTORCH="$INSTALL_DIR"
export LIBTORCH_INCLUDE="$INSTALL_DIR/include"
export LIBTORCH_LIB="$INSTALL_DIR/lib"
export DYLD_LIBRARY_PATH="$INSTALL_DIR/lib:$DYLD_LIBRARY_PATH"
export CXX="clang++"

# Print instructions for persistent use
cat <<EOF

LibTorch $LIBTORCH_VERSION is built and set up for torch-sys!
To use these variables in your shell, run:

  source $PWD/tch.sh [version] [python_build]

Or add the following to your ~/.zshrc or ~/.bash_profile:

  export LIBTORCH="$INSTALL_DIR"
  export LIBTORCH_INCLUDE="$INSTALL_DIR/include"
  export LIBTORCH_LIB="$INSTALL_DIR/lib"
  export DYLD_LIBRARY_PATH="$INSTALL_DIR/lib:$DYLD_LIBRARY_PATH"
  export CXX="clang++"

EOF
