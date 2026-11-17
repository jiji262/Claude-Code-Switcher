@echo off
setlocal

echo ==================================================
echo Claude Code Switcher v2.0 - Windows Release Builder
echo ==================================================
echo.

where cargo >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo Error: Cargo not found. Install Rust via https://rustup.rs/
    exit /b 1
)

echo [1/2] Building standalone release executable...
cargo build --release --bin claude-code-switcher
if %ERRORLEVEL% NEQ 0 (
    echo Error: Build failed!
    exit /b 1
)

echo.
echo [2/2] Copying executable to dist\...
if not exist "dist" mkdir "dist"
copy "target\release\claude-code-switcher.exe" "dist\Claude-Code-Switcher-Windows-x64.exe" /Y >nul
copy "target\release\claude-code-switcher.exe" "dist\claude-code-switcher.exe" /Y >nul

echo.
echo ==================================================
echo Build completed successfully!
echo Standalone executable: dist\Claude-Code-Switcher-Windows-x64.exe
echo ==================================================