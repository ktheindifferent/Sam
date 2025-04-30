fn main() {
    // Set the path to the directory containing OpenCL.lib
    println!("cargo:rustc-link-search=native=C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.8/lib/x64");
    // Link to the OpenCL library
    println!("cargo:rustc-link-lib=dylib=OpenCL");
    // Optionally, set the path to clang if needed by your build
    println!(r"cargo:clang=C:\Program Files\LLVM\bin");
}