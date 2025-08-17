#!/bin/bash

echo "🎨 Claude Code Switcher - 图标生成脚本"
echo "========================================"

# 检查 SVG 文件是否存在
if [ ! -f "resources/icons/icon.svg" ]; then
    echo "❌ SVG 文件不存在: resources/icons/icon.svg"
    exit 1
fi

echo "📁 源文件: resources/icons/icon.svg"
echo "📁 输出目录: resources/icons/"
echo

# 方法1: 使用 Rust 工具生成
echo "[1/3] 尝试使用 Rust 工具生成图标..."
if cargo run --bin generate_icons 2>/dev/null; then
    echo "✅ Rust 工具生成成功"
else
    echo "⚠️  Rust 工具生成失败，尝试其他方法..."
    
    # 方法2: 使用 rsvg-convert (如果可用)
    echo "[2/3] 尝试使用 rsvg-convert..."
    if command -v rsvg-convert &> /dev/null; then
        echo "📦 使用 rsvg-convert 生成 PNG 文件..."
        
        # 生成不同尺寸的 PNG
        for size in 16 32 48 64 128 256 512 1024; do
            rsvg-convert -w $size -h $size resources/icons/icon.svg -o "resources/icons/icon_${size}.png"
            echo "✅ 生成 ${size}x${size} PNG"
        done
        
        # 在 macOS 上生成 ICNS
        if [[ "$OSTYPE" == "darwin"* ]] && command -v iconutil &> /dev/null; then
            echo "📦 生成 macOS ICNS 文件..."
            ICONSET_DIR="resources/icons/icon.iconset"
            mkdir -p "$ICONSET_DIR"
            
            # 生成 iconset 所需的文件
            for size in 16 32 64 128 256 512 1024; do
                cp "resources/icons/icon_${size}.png" "$ICONSET_DIR/icon_${size}x${size}.png"
                if [ $size -le 512 ]; then
                    size2x=$((size * 2))
                    if [ -f "resources/icons/icon_${size2x}.png" ]; then
                        cp "resources/icons/icon_${size2x}.png" "$ICONSET_DIR/icon_${size}x${size}@2x.png"
                    fi
                fi
            done
            
            # 生成 ICNS
            iconutil -c icns "$ICONSET_DIR" -o "resources/icons/icon.icns"
            rm -rf "$ICONSET_DIR"
            echo "✅ 生成 macOS ICNS"
        fi
        
        # 生成简单的 ICO (复制最大的 PNG)
        if [ -f "resources/icons/icon_256.png" ]; then
            cp "resources/icons/icon_256.png" "resources/icons/icon.ico"
            echo "✅ 生成 Windows ICO (简化版)"
        fi
        
    else
        echo "❌ rsvg-convert 未找到"
        echo "   Ubuntu/Debian: sudo apt-get install librsvg2-bin"
        echo "   macOS: brew install librsvg"
        echo "   Windows: 请手动转换 SVG 文件"
    fi
fi

# 方法3: 使用 Python 脚本 (如果可用)
if [ -f "scripts/generate_icons.py" ] && command -v python3 &> /dev/null; then
    echo "[3/3] 尝试使用 Python 脚本..."
    if python3 scripts/generate_icons.py 2>/dev/null; then
        echo "✅ Python 脚本生成成功"
    else
        echo "⚠️  Python 脚本需要额外依赖: pip install cairosvg pillow"
    fi
fi

echo
echo "📋 生成的文件:"
ls -la resources/icons/ | grep -E '\.(png|ico|icns)$' || echo "   (没有找到生成的图标文件)"

echo
echo "🎉 图标生成脚本执行完成!"
echo "========================================"
