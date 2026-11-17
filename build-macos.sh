#!/bin/bash
set -euo pipefail

echo "=================================================="
echo "Claude Code Switcher v2.0 - macOS Release Builder"
echo "=================================================="
echo

if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo not found. Install Rust via https://rustup.rs/"
    exit 1
fi

TARGET_ARCH="aarch64-apple-darwin"
if [[ "$(uname -m)" == "x86_64" ]]; then
    TARGET_ARCH="x86_64-apple-darwin"
fi

if ! rustup target list --installed | grep -q "${TARGET_ARCH}"; then
    echo "[setup] Installing ${TARGET_ARCH} target..."
    rustup target add "${TARGET_ARCH}"
fi

if [[ "${1:-}" == "--clean" ]]; then
    echo "[clean] Running cargo clean..."
    cargo clean
fi

echo "[1/5] Building release binary (${TARGET_ARCH})..."
cargo build --release --bin claude-code-switcher --target "${TARGET_ARCH}"

BIN_SRC="target/${TARGET_ARCH}/release/claude-code-switcher"
APP_NAME="Claude Code Switcher"
APP_DIR="dist/${APP_NAME}.app"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "[2/5] Preparing dist/ and macOS .app bundle..."
rm -rf "${APP_DIR}" "dist/dist" "dist/resources"
mkdir -p "${MACOS_DIR}" "${RESOURCES_DIR}"

cp "${BIN_SRC}" "dist/claude-code-switcher-macos"
cp "${BIN_SRC}" "${MACOS_DIR}/claude-code-switcher"
cp "resources/Info.plist" "${CONTENTS_DIR}/Info.plist"
chmod +x "dist/claude-code-switcher-macos" "${MACOS_DIR}/claude-code-switcher"

echo "[3/5] Packaging macOS icon resources..."
if [ -f "resources/icons/icon.icns" ]; then
    cp "resources/icons/icon.icns" "${RESOURCES_DIR}/icon.icns"
elif [ -f "resources/icons/icon.svg" ] && command -v rsvg-convert &> /dev/null && command -v iconutil &> /dev/null; then
    ICONSET_DIR="${RESOURCES_DIR}/icon.iconset"
    mkdir -p "${ICONSET_DIR}"
    for size in 16 32 128 256 512; do
        rsvg-convert -w "$size" -h "$size" "resources/icons/icon.svg" -o "${ICONSET_DIR}/icon_${size}x${size}.png"
        size2x=$((size * 2))
        rsvg-convert -w "$size2x" -h "$size2x" "resources/icons/icon.svg" -o "${ICONSET_DIR}/icon_${size}x${size}@2x.png"
    done
    iconutil -c icns "${ICONSET_DIR}" -o "${RESOURCES_DIR}/icon.icns"
    rm -rf "${ICONSET_DIR}"
fi

if [ -f "resources/icons/icon_1024.png" ]; then
    cp "resources/icons/icon_1024.png" "${RESOURCES_DIR}/icon.png"
fi

echo "[4/5] Applying ad-hoc code signature for Apple Silicon..."
if command -v codesign &> /dev/null; then
    codesign --force --sign - "dist/claude-code-switcher-macos"
    codesign --force --deep --sign - "${APP_DIR}"
fi

echo "[5/5] Verifying build output..."
echo "  Target:      ${TARGET_ARCH}"
echo "  Version:     $(./dist/claude-code-switcher-macos --version)"
echo "  Executable:  dist/claude-code-switcher-macos ($(du -h dist/claude-code-switcher-macos | cut -f1))"
echo "  App Bundle:  ${APP_DIR} ($(du -sh "${APP_DIR}" | cut -f1))"
echo
echo "Build completed successfully."