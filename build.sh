#!/bin/bash

echo "========================================"
echo "Claude Code Switcher - Build Script"
echo "========================================"
echo

echo "[1/3] Cleaning previous build..."
cargo clean

echo
echo "[2/3] Building release version..."
cargo build --release

echo
echo "[3/3] Copying executable..."
mkdir -p dist
cp target/release/claude-code-switcher dist/ 2>/dev/null || true

echo
echo "========================================"
echo "Build completed!"
echo "Executable location: dist/claude-code-switcher"
echo "========================================"