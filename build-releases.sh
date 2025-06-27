#!/bin/bash
# Cross-platform build script for rclone-decrypt

set -e

VERSION="v0.1.0"
APP_NAME="rclone-decrypt"

echo "Building $APP_NAME $VERSION for multiple platforms..."

# Create releases directory
mkdir -p releases

# Function to build for a specific target
build_target() {
    local target=$1
    local output_name=$2
    local extension=$3
    
    echo "Building for $target..."
    
    if [[ "$target" == *"windows"* ]]; then
        # For Windows targets, we might need cross-compilation tools
        if command -v x86_64-w64-mingw32-gcc &> /dev/null; then
            echo "Using MinGW cross-compiler for Windows"
            export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
            export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
            export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
        fi
    fi
    
    cargo build --release --target $target
    
    # Copy and rename the binary
    cp target/$target/release/$APP_NAME$extension releases/${APP_NAME}-${VERSION}-${output_name}$extension
    
    echo "✅ Built: releases/${APP_NAME}-${VERSION}-${output_name}$extension"
}

# Build for different platforms
echo "🔨 Building releases..."

# macOS (Intel)
if rustup target list --installed | grep -q "x86_64-apple-darwin"; then
    build_target "x86_64-apple-darwin" "macos-intel" ""
else
    echo "Adding x86_64-apple-darwin target..."
    rustup target add x86_64-apple-darwin
    build_target "x86_64-apple-darwin" "macos-intel" ""
fi

# macOS (ARM - Apple Silicon)
if rustup target list --installed | grep -q "aarch64-apple-darwin"; then
    build_target "aarch64-apple-darwin" "macos-arm64" ""
else
    echo "Adding aarch64-apple-darwin target..."
    rustup target add aarch64-apple-darwin
    build_target "aarch64-apple-darwin" "macos-arm64" ""
fi

# Windows (x64)
if rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    build_target "x86_64-pc-windows-gnu" "windows-x64" ".exe"
else
    echo "x86_64-pc-windows-gnu target not available"
fi

# Linux (x64) - if available
if rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
    build_target "x86_64-unknown-linux-gnu" "linux-x64" ""
else
    echo "Adding x86_64-unknown-linux-gnu target..."
    rustup target add x86_64-unknown-linux-gnu
    build_target "x86_64-unknown-linux-gnu" "linux-x64" ""
fi

echo ""
echo "🎉 Build completed! Release binaries:"
ls -la releases/

echo ""
echo "📦 Release files created:"
for file in releases/*; do
    if [[ -f "$file" ]]; then
        echo "  - $(basename "$file") ($(ls -lh "$file" | awk '{print $5}'))"
    fi
done

echo ""
echo "Usage examples:"
echo "  Windows: .\\${APP_NAME}-${VERSION}-windows-x64.exe --help"
echo "  macOS:   ./${APP_NAME}-${VERSION}-macos-arm64 --help"
echo "  Linux:   ./${APP_NAME}-${VERSION}-linux-x64 --help"
