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
echo "[3/5] Processing icons..."
if [ -f "resources/icons/icon.svg" ]; then
    # 尝试使用 rsvg-convert 生成 PNG
    if command -v rsvg-convert &> /dev/null; then
        echo "Converting SVG to PNG using rsvg-convert..."
        mkdir -p resources/icons
        rsvg-convert -w 64 -h 64 resources/icons/icon.svg -o resources/icons/icon_64.png
        rsvg-convert -w 128 -h 128 resources/icons/icon.svg -o resources/icons/icon_128.png
        rsvg-convert -w 256 -h 256 resources/icons/icon.svg -o resources/icons/icon_256.png
        echo "✅ PNG icons generated"
    else
        echo "⚠️  rsvg-convert not found. Install with: sudo apt-get install librsvg2-bin"
    fi
else
    echo "⚠️  SVG icon not found at resources/icons/icon.svg"
fi

echo
echo "[4/5] Copying executable..."
mkdir -p dist
cp target/release/claude-code-switcher dist/ 2>/dev/null || true

echo
echo "[5/5] Copying resources..."
mkdir -p dist/resources
if [ -d "resources/icons" ]; then
    cp -r resources/icons dist/resources/
fi

echo
echo "========================================"
echo "Build completed!"
echo "Executable location: dist/claude-code-switcher"
echo "Resources copied to: dist/resources/"
echo "========================================"