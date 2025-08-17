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
echo [3/5] Generating Windows icon...
if exist "resources\icons\icon.svg" (
    echo Converting SVG to ICO...
    REM 这里可以添加 SVG 到 ICO 的转换逻辑
    REM 目前先检查是否已有 ICO 文件
    if not exist "resources\icons\icon.ico" (
        echo Warning: icon.ico not found. Please generate it manually.
        echo You can use online tools or ImageMagick to convert SVG to ICO.
    )
) else (
    echo Warning: icon.svg not found at resources\icons\icon.svg
)

echo.
echo [4/5] Copying executable...
if not exist "dist" mkdir dist
copy "target\release\claude-code-switcher.exe" "dist\" /Y

echo.
echo [5/5] Copying resources...
if not exist "dist\resources" mkdir "dist\resources"
if exist "resources\icons" (
    if not exist "dist\resources\icons" mkdir "dist\resources\icons"
    copy "resources\icons\*" "dist\resources\icons\" /Y >nul 2>&1
)

echo.
echo ========================================
echo Build completed!
echo Executable location: dist\claude-code-switcher.exe
echo Resources copied to: dist\resources\
echo ========================================
pause