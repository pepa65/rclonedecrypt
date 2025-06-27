#!/bin/bash

# Build script for cross-platform rclone-decrypt
# This script builds binaries for macOS (both Intel and Apple Silicon) and Windows

set -e

echo "Building rclone-decrypt for multiple platforms..."

# Function to install cross-compilation target if not present
install_target() {
    local target=$1
    if ! rustup target list --installed | grep -q "$target"; then
        echo "Installing target: $target"
        rustup target add "$target"
    else
        echo "Target $target already installed"
    fi
}

# Create output directory
mkdir -p dist

# Build for current platform (macOS)
echo "Building for current platform..."
cargo build --release
cp target/release/rclone-decrypt dist/rclone-decrypt-$(uname -m)-apple-darwin

# Install cross-compilation targets
if [[ "$(uname)" == "Darwin" ]]; then
    # On macOS, build for both architectures
    echo "Installing macOS targets..."
    install_target "x86_64-apple-darwin"
    install_target "aarch64-apple-darwin"
    
    echo "Building for x86_64-apple-darwin..."
    cargo build --release --target x86_64-apple-darwin
    cp target/x86_64-apple-darwin/release/rclone-decrypt dist/rclone-decrypt-x86_64-apple-darwin
    
    echo "Building for aarch64-apple-darwin..."
    cargo build --release --target aarch64-apple-darwin
    cp target/aarch64-apple-darwin/release/rclone-decrypt dist/rclone-decrypt-aarch64-apple-darwin
    
    # Try to build for Windows (requires mingw-w64)
    if command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
        echo "Installing Windows target..."
        install_target "x86_64-pc-windows-gnu"
        
        echo "Building for x86_64-pc-windows-gnu..."
        cargo build --release --target x86_64-pc-windows-gnu
        cp target/x86_64-pc-windows-gnu/release/rclone-decrypt.exe dist/rclone-decrypt-x86_64-pc-windows-gnu.exe
    else
        echo "Warning: mingw-w64 not found. Install it with 'brew install mingw-w64' to build Windows binaries."
    fi
fi

echo ""
echo "Build complete! Binaries are available in the dist/ directory:"
ls -la dist/

echo ""
echo "To test the binary, you can use:"
echo "./dist/rclone-decrypt-* --help"
