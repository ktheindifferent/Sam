# Neural Style Transfer (NST) Module

This module provides Neural Style Transfer functionality using PyTorch and the tch-rs crate. Neural Style Transfer allows you to combine the content of one image with the artistic style of another image using deep learning.

## Features

- **Real-time Style Transfer**: Apply artistic styles to images in real-time
- **Multiple Pre-installed Styles**: Comes with several classical art styles:
  - Fra Angelico
  - Paul Cézanne
  - Sassetta
  - Vincent van Gogh
- **VGG16 Backend**: Uses pre-trained VGG16 model for feature extraction
- **Configurable Parameters**: Adjustable style weight, learning rate, and iteration steps
- **Background Processing**: Long-running transfers are processed in background threads
- **Progress Tracking**: Intermediate results saved during processing

## Prerequisites

### System Requirements

- **Operating System**: macOS, Linux, or Windows (WSL recommended)
- **Memory**: At least 4GB RAM (8GB+ recommended for large images)
- **Storage**: ~500MB for models and dependencies
- **Network**: Internet connection for initial model download

### Dependencies

The NST module requires PyTorch/LibTorch to be installed. We provide an automated installation script for this.

## Installation

### Step 1: Install PyTorch/LibTorch

Run the automated installation script:

```bash
# Make the script executable (if not already)
chmod +x scripts/install_pytorch.sh

# Run the installation script
./scripts/install_pytorch.sh
```

#### Script Options

The installation script supports several options:

```bash
# Install specific PyTorch version
./scripts/install_pytorch.sh -v 2.2.0

# Install to custom directory
./scripts/install_pytorch.sh -d /opt/libtorch

# Show help
./scripts/install_pytorch.sh -h
```

#### Manual Installation (Alternative)

If the script doesn't work for your system, you can install LibTorch manually:

1. **Download LibTorch** from [PyTorch.org](https://pytorch.org/get-started/locally/):
   - Select "LibTorch" as package
   - Choose your OS (Linux/macOS/Windows)
   - Select "CPU" for compute platform (or "CUDA" if you have GPU support)
   - Download the stable release

2. **Extract and Install**:
   ```bash
   # Extract the downloaded archive
   unzip libtorch-*.zip
   
   # Move to installation directory
   sudo mv libtorch /usr/local/
   
   # Set environment variables
   export LIBTORCH="/usr/local/libtorch"
   export LD_LIBRARY_PATH="$LIBTORCH/lib:$LD_LIBRARY_PATH"
   ```

3. **Add to Shell Profile**:
   ```bash
   # For zsh users
   echo 'export LIBTORCH="/usr/local/libtorch"' >> ~/.zshrc
   echo 'export LD_LIBRARY_PATH="$LIBTORCH/lib:$LD_LIBRARY_PATH"' >> ~/.zshrc
   
   # For bash users
   echo 'export LIBTORCH="/usr/local/libtorch"' >> ~/.bashrc
   echo 'export LD_LIBRARY_PATH="$LIBTORCH/lib:$LD_LIBRARY_PATH"' >> ~/.bashrc
   ```

### Step 2: Build with NST Support

Build the project with NST feature enabled:

```bash
# Build with NST feature
cargo build --features nst

# Or for release build
cargo build --release --features nst
```

### Step 3: Install NST Models

After building, install the required models:

```rust
// In your Rust code
use sam::services::media::image::nst;

// Install VGG16 model and style images
nst::install().expect("Failed to install NST models");
```

## Usage

### API Endpoints

#### List Available Styles

```http
GET /nst/styles
```

Returns a JSON array of available styles:

```json
[
  {
    "name": "Fra Angelico",
    "file_path": "/opt/sam/models/nst/fra_angelico.jpg"
  },
  {
    "name": "Vincent Van Gogh", 
    "file_path": "/opt/sam/models/nst/vincent_van_gogh.jpg"
  }
]
```

#### Apply Style Transfer

```http
POST /nst/run
Content-Type: application/json

{
  "image_id": "oid:your_image_id",
  "nst_style": "Vincent Van Gogh"
}
```

### Programmatic Usage

```rust
use sam::services::media::image::nst;

// Apply style transfer
let result = nst::run(
    "/path/to/style/image.jpg",      // Style image path
    "/path/to/content/image.jpg",    // Content image path  
    "output_id".to_string(),         // Output identifier
    "Style Name".to_string()         // Style name for naming
);

match result {
    Ok(_) => println!("Style transfer completed successfully"),
    Err(e) => eprintln!("Style transfer failed: {}", e),
}
```

## Configuration

### Style Transfer Parameters

The following constants control the style transfer process:

```rust
const STYLE_WEIGHT: f64 = 1e6;      // How strongly to apply style vs content
const LEARNING_RATE: f64 = 1e-1;    // Optimization learning rate  
const TOTAL_STEPS: i64 = 10000;     // Number of optimization steps
const STYLE_INDEXES: [usize; 5] = [0, 2, 5, 7, 10];  // VGG layers for style
const CONTENT_INDEXES: [usize; 1] = [7];              // VGG layers for content
```

### Adding Custom Styles

1. **Add Style Image**: Place your style image in `/opt/sam/models/nst/`
2. **Use Descriptive Names**: Name files like `artist_name.jpg` (underscores will be converted to spaces)
3. **Restart Application**: The styles list is generated at runtime

### Memory and Performance

- **Reduce Image Size**: Smaller images process faster and use less memory
- **Adjust Steps**: Reduce `TOTAL_STEPS` for faster (but potentially lower quality) results
- **Monitor Progress**: Intermediate results are saved every 1000 steps
- **GPU Support**: If you have CUDA/Metal support, LibTorch will automatically use GPU acceleration

## Troubleshooting

### Common Issues

#### "NST feature not enabled"
- **Solution**: Build with `--features nst` flag
- **Example**: `cargo build --features nst`

#### "VGG16 model not found"
- **Solution**: Run the install function or manually download the model
- **Manual Download**: `wget -O /opt/sam/models/vgg16.ot https://github.com/LaurentMazare/tch-rs/releases/download/mw/vgg16.ot`

#### "Failed to load style/content image"
- **Solution**: Ensure image files exist and are readable
- **Supported Formats**: JPEG, PNG (through the `image` crate)

#### LibTorch Library Not Found
- **Linux/macOS**: Check `LD_LIBRARY_PATH` or `DYLD_LIBRARY_PATH`
- **Windows**: Ensure LibTorch DLLs are in PATH
- **Solution**: Re-run environment setup from installation script

#### Permission Denied Errors
- **Solution**: Ensure write permissions to `/opt/sam/models/` directory
- **Alternative**: Change installation directory using script options

### Apple Silicon (M1/M2) Notes

- Use CPU-only LibTorch builds (GPU support for Apple Silicon is limited)
- The installation script handles architecture detection automatically
- If you encounter linking errors, ensure you're using the correct LibTorch build

### Performance Optimization

1. **Use GPU if Available**: CUDA/Metal acceleration significantly improves performance
2. **Resize Images**: Process smaller images (512x512 or 1024x1024) for faster results
3. **Reduce Iterations**: Lower `TOTAL_STEPS` for quicker processing
4. **Background Processing**: The module automatically processes transfers in background threads

## Technical Details

### Algorithm

The NST implementation is based on the seminal paper "A Neural Algorithm of Artistic Style" by Gatys et al. It uses:

1. **VGG16 Network**: Pre-trained convolutional neural network for feature extraction
2. **Gram Matrices**: Capture style information through feature correlations
3. **Content Loss**: Preserves structural content of the original image
4. **Style Loss**: Matches statistical properties of the style image
5. **Optimization**: Iteratively optimizes the output image to minimize combined loss

### File Structure

```
src/lib/services/media/image/nst.rs    # Main NST implementation
scripts/install_pytorch.sh             # PyTorch installation script
packages/nst/                          # Style image assets
  ├── fra_angelico.jpg
  ├── paul_cézanne.jpg  
  ├── sassetta.jpg
  └── vincent_van_gogh.jpg
```

### Dependencies

- **tch**: PyTorch bindings for Rust
- **image**: Image loading and processing
- **serde**: JSON serialization
- **tokio**: Async runtime for background processing

## Contributing

When adding new features to the NST module:

1. **Feature Gates**: Use `#[cfg(feature = "nst")]` for PyTorch-dependent code
2. **Error Handling**: Provide clear error messages for missing dependencies
3. **Logging**: Use structured logging for debugging and monitoring
4. **Testing**: Add tests with feature gates for NST functionality
5. **Documentation**: Update this README with new features

## License

This module is part of the SAM project and is licensed under the same terms. The neural style transfer algorithm is based on academic research and is free for research and educational use.
