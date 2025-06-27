#!/bin/bash
# Simple Windows build script for rclone-decrypt

set -e

VERSION="v0.1.0"
APP_NAME="rclone-decrypt"

echo "Building $APP_NAME $VERSION for Windows..."

# Create releases directory
mkdir -p releases

# Try to build for Windows x64 using the GNU toolchain (more compatible)
echo "Building for Windows x64 (GNU)..."

# First, let's try a direct build without cross-compilation
echo "Attempting direct cargo build for Windows target..."

if cargo build --release --target x86_64-pc-windows-gnu; then
    echo "✅ Windows build successful!"
    cp target/x86_64-pc-windows-gnu/release/${APP_NAME}.exe releases/${APP_NAME}-${VERSION}-windows-x64.exe
    echo "✅ Created: releases/${APP_NAME}-${VERSION}-windows-x64.exe"
    
    # Show file info
    ls -la releases/${APP_NAME}-${VERSION}-windows-x64.exe
    file releases/${APP_NAME}-${VERSION}-windows-x64.exe
else
    echo "❌ Windows build failed - cross-compilation tools might be needed"
    echo ""
    echo "To build for Windows from macOS, you might need to:"
    echo "1. Install MinGW-w64 cross-compiler:"
    echo "   brew install mingw-w64"
    echo "2. Or use a different approach like Docker or GitHub Actions"
    echo ""
    echo "Alternatively, you can build this on a Windows machine directly with:"
    echo "   cargo build --release"
    exit 1
fi

echo ""
echo "🎉 Windows build completed!"
echo ""
echo "Usage on Windows:"
echo "  .\\${APP_NAME}-${VERSION}-windows-x64.exe --input encrypted.bin --output decrypted.pdf --password \"your-password\" --salt \"your-salt\""
