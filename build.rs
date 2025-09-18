fn main() {
    #[cfg(target_os = "windows")]{
        // Set the path to the directory containing OpenCL.lib
        println!("cargo:rustc-link-search=native=C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.8/lib/x64");
        // Link to the OpenCL library
        println!("cargo:rustc-link-lib=dylib=OpenCL");
        // Optionally, set the path to clang if needed by your build
        println!(r"cargo:clang=C:\Program Files\LLVM\bin");
    }

    // Check for NST feature and provide helpful PyTorch installation guidance
    #[cfg(feature = "nst")]
    {
        // If the user is trying to build with NST feature, provide guidance
        if std::env::var("LIBTORCH").is_err() && std::env::var("LIBTORCH_USE_PYTORCH").is_err() {
            eprintln!("================================================================================");
            eprintln!("🔥 Neural Style Transfer (NST) Feature Enabled");
            eprintln!("================================================================================");
            eprintln!("");
            eprintln!("The NST feature requires PyTorch/LibTorch to be installed.");
            eprintln!("");
            eprintln!("📋 Quick Setup:");
            eprintln!("   1. Run the installation script: ./scripts/install_pytorch.sh");
            eprintln!("   2. Or check current setup: ./scripts/check_pytorch.sh");
            eprintln!("");
            eprintln!("🔧 Manual Setup:");
            eprintln!("   export LIBTORCH=/path/to/libtorch");
            eprintln!("   export LD_LIBRARY_PATH=\"$LIBTORCH/lib:$LD_LIBRARY_PATH\"");
            eprintln!("");
            eprintln!("🐍 Using Python PyTorch:");
            eprintln!("   export LIBTORCH_USE_PYTORCH=1");
            eprintln!("");
            eprintln!("📚 More info: docs/features/NST_MODULE.md");
            eprintln!("================================================================================");
        }
    }
}