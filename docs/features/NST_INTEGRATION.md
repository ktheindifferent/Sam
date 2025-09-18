# Neural Style Transfer (NST) Integration

This document describes the Neural Style Transfer feature implementation in SAM using PyTorch and tch-rs.

## Overview

The NST implementation allows users to apply artistic styles to their images using deep learning techniques. It's based on the seminal paper "Image Style Transfer Using Convolutional Neural Networks" by Gatys et al.

## Architecture

### Dependencies
- **tch-rs 0.20**: Rust bindings for PyTorch C++ API
- **PyTorch 2.7.0**: Built from source for maximum compatibility
- **VGG16**: Pre-trained convolutional neural network for feature extraction

### File Structure
```
src/lib/services/media/image/nst.rs  # Main NST implementation
scripts/build_libtorch.sh             # PyTorch source build script
packages/nst/                         # Style image assets
```

## Installation

### Automatic Installation (Recommended)
```bash
# Build PyTorch 2.7.0 from source (30-60 minutes)
./scripts/build_libtorch.sh

# Build SAM with NST support
cargo build --features nst
```

### Manual Installation
1. Install PyTorch 2.7.0 compatible with tch-rs 0.20
2. Set environment variables:
   ```bash
   export LIBTORCH="/usr/local/libtorch"
   export LD_LIBRARY_PATH="$LIBTORCH/lib:$LD_LIBRARY_PATH"
   export DYLD_LIBRARY_PATH="$LIBTORCH/lib:$DYLD_LIBRARY_PATH"  # macOS only
   ```
3. Build with NST feature: `cargo build --features nst`

## Usage

### API Endpoints

#### Get Available Styles
```http
GET /nst/styles
```
Returns list of available artistic styles.

#### Run Style Transfer
```http
POST /nst/run
Content-Type: application/json

{
  "image_id": "oid:<file_id>",
  "nst_style": "Vincent Van Gogh"
}
```

### Programmatic Usage

```rust
use sam::services::media::image::nst;

// Install required models and styles
nst::install()?;

// Run style transfer
nst::run(
    "/path/to/style.jpg",
    "/path/to/content.jpg", 
    "output_id".to_string(),
    "style_name".to_string()
)?;
```

## Technical Details

### Algorithm Implementation

1. **Feature Extraction**: Uses VGG16 pre-trained on ImageNet
2. **Style Loss**: Computed using Gram matrices of feature maps
3. **Content Loss**: Mean squared error on high-level features
4. **Optimization**: LBFGS optimizer with configurable parameters

### Key Components

- `gram_matrix()`: Computes style representation
- `style_loss()`: Measures style similarity using Gram matrices
- `run()`: Main optimization loop with content and style loss

### Configuration

```rust
const STYLE_WEIGHT: f64 = 1e6;        // Style loss weight
const LEARNING_RATE: f64 = 1e-1;      // Optimizer learning rate
const TOTAL_STEPS: i64 = 10000;       // Optimization iterations
const STYLE_INDEXES: [usize; 5] = [0, 2, 5, 7, 10];  // VGG layers for style
const CONTENT_INDEXES: [usize; 1] = [7];              // VGG layers for content
```

## Included Styles

The system includes several pre-trained artistic styles:

- **Fra Angelico**: Renaissance religious art style
- **Paul Cézanne**: Post-impressionist geometric style  
- **Sassetta**: Early Renaissance Italian style
- **Vincent van Gogh**: Post-impressionist expressive style

## Build System

### PyTorch Source Build

The `build_libtorch.sh` script:
- Clones PyTorch 2.7.0 with all submodules
- Configures build for LibTorch C++ API only
- Optimizes for production use (Release mode)
- Supports cross-platform builds (macOS, Linux)
- Sets up proper environment variables

### Build Configuration

```cmake
-DCMAKE_BUILD_TYPE=Release
-DBUILD_SHARED_LIBS=ON
-DBUILD_PYTHON=OFF
-DBUILD_BINARY=ON
-DBUILD_TEST=OFF
-DUSE_CUDA=OFF
-DUSE_MKLDNN=ON
-DUSE_OPENMP=ON
```

## Performance Notes

- First run downloads and caches VGG16 weights (~500MB)
- Style transfer takes 2-10 minutes depending on image size
- Intermediate results saved every 1000 iterations
- GPU acceleration disabled for compatibility
- CPU optimization enabled with Intel MKL-DNN

## Troubleshooting

### Common Issues

1. **Missing LibTorch**: Ensure `LIBTORCH` environment variable is set
2. **Version Mismatch**: tch-rs 0.20 requires exactly PyTorch 2.7.0
3. **Memory Issues**: Large images may require reducing resolution
4. **Build Failures**: Check C++ compiler and CMake versions

### Debug Mode

Enable detailed logging:
```bash
RUST_LOG=debug cargo run --features nst
```

### Verification

Test the installation:
```bash
cargo test --features nst nst_tests
```

## Future Enhancements

- GPU acceleration support
- Real-time style transfer with smaller models
- Custom style training interface
- Batch processing capabilities
- Style interpolation and mixing

## References

- [Gatys et al. - Image Style Transfer Using Convolutional Neural Networks](https://www.cv-foundation.org/openaccess/content_cvpr_2016/papers/Gatys_Image_Style_Transfer_CVPR_2016_paper.pdf)
- [PyTorch Neural Style Transfer Tutorial](https://pytorch.org/tutorials/advanced/neural_style_tutorial.html)
- [tch-rs Documentation](https://docs.rs/tch/)
