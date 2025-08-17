#!/bin/bash

echo "========================================"
echo "Claude Code Switcher - macOS Build Script"
echo "========================================"
echo

# 检查 Rust 环境
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Cargo not found. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# 检查 macOS 目标是否安装
if ! rustup target list --installed | grep -q "aarch64-apple-darwin"; then
    echo "📦 Installing aarch64-apple-darwin target..."
    rustup target add aarch64-apple-darwin
fi

echo "[1/4] Cleaning previous build..."
cargo clean

echo
echo "[2/4] Building release version for macOS (Apple Silicon)..."
cargo build --release --target aarch64-apple-darwin

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo
echo "[3/4] Creating distribution directory..."
mkdir -p dist

echo
echo "[4/4] Copying executable..."
cp target/aarch64-apple-darwin/release/claude-code-switcher dist/claude-code-switcher-macos

# 检查文件信息
echo
echo "📋 Build Information:"
echo "Target: aarch64-apple-darwin (Apple Silicon)"
echo "File type: $(file dist/claude-code-switcher-macos)"
echo "File size: $(du -h dist/claude-code-switcher-macos | cut -f1)"

echo
echo "========================================"
echo "✅ macOS Build completed successfully!"
echo "📁 Executable location: dist/claude-code-switcher-macos"
echo "🚀 You can now run: ./dist/claude-code-switcher-macos"
echo "========================================"