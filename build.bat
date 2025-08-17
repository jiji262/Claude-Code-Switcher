@echo off
echo ========================================
echo Claude Code Switcher - Build Script
echo ========================================
echo.

echo [1/3] Cleaning previous build...
cargo clean

echo.
echo [2/3] Building release version...
cargo build --release

echo.
echo [3/3] Copying executable...
if not exist "dist" mkdir dist
copy "target\release\claude-code-switcher.exe" "dist\" /Y

echo.
echo ========================================
echo Build completed!
echo Executable location: dist\claude-code-switcher.exe
echo ========================================
pause