# 图标配置说明

本项目已成功配置了 `resources/icons/icon.svg` 作为应用程序的图标，在各个显示位置都使用这个图标。

## 🎨 图标文件

### 源文件
- **SVG 图标**: `resources/icons/icon.svg` - 主要的矢量图标文件
- **设计**: 现代化的代码主题图标，包含代码括号、斜杠和装饰元素

### 生成的图标文件
运行图标生成脚本后，会自动生成以下文件：

- **PNG 图标**: `resources/icons/icon_*.png` (16x16 到 1024x1024 多种尺寸)
- **Windows ICO**: `resources/icons/icon.ico` - Windows 应用程序图标
- **macOS ICNS**: `resources/icons/icon.icns` - macOS 应用程序图标

## 🔧 图标配置位置

### 1. 应用程序窗口图标 (所有平台)
- **文件**: `src/main.rs`
- **配置**: 在 `main()` 函数中通过 `egui::ViewportBuilder::with_icon()` 设置
- **实现**: 使用 `load_icon_from_svg()` 函数从 SVG 渲染为 RGBA 位图

### 2. macOS App Bundle 图标
- **Info.plist**: `resources/Info.plist` - 指定 `CFBundleIconFile` 为 `icon`
- **ICNS 文件**: 构建时自动复制到 `Contents/Resources/icon.icns`
- **显示位置**: Dock、Finder、应用程序文件夹、Spotlight 搜索等

### 3. Windows 应用程序图标
- **资源文件**: `resources/app.rc` - 定义应用程序图标资源
- **ICO 文件**: `resources/icons/icon.ico`
- **显示位置**: 任务栏、文件资源管理器、开始菜单等

### 4. Linux 应用程序图标
- **PNG 文件**: 多种尺寸的 PNG 文件
- **显示位置**: 应用程序启动器、任务栏等

## 🚀 使用方法

### 生成图标文件
```bash
# 方法1: 使用统一脚本 (推荐)
./generate_icons.sh

# 方法2: 使用 Rust 工具
cargo run --bin generate_icons

# 方法3: 手动使用 rsvg-convert (需要安装 librsvg)
rsvg-convert -w 256 -h 256 resources/icons/icon.svg -o resources/icons/icon_256.png
```

### 构建应用程序

#### macOS
```bash
./build-macos.sh
# 生成: dist/Claude Code Switcher.app (包含图标)
```

#### Windows
```bash
./build.bat
# 生成: dist/claude-code-switcher.exe (包含图标)
```

#### Linux
```bash
./build.sh
# 生成: dist/claude-code-switcher (包含图标资源)
```

## 📋 依赖要求

### 图标生成依赖
- **macOS**: `brew install librsvg` (用于 rsvg-convert 和 iconutil)
- **Ubuntu/Debian**: `sudo apt-get install librsvg2-bin`
- **Windows**: 手动转换或使用在线工具

### Rust 依赖
项目已包含以下依赖用于图标处理：
- `resvg` - SVG 渲染
- `tiny-skia` - 2D 图形渲染

## 🔄 更新图标

如果需要更新应用程序图标：

1. **替换 SVG 文件**: 更新 `resources/icons/icon.svg`
2. **重新生成图标**: 运行 `./generate_icons.sh`
3. **重新构建应用**: 运行相应平台的构建脚本

## ✅ 验证图标配置

### 检查生成的文件
```bash
ls -la resources/icons/
# 应该看到: icon.svg, icon.ico, icon.icns, icon_*.png
```

### 检查 macOS App Bundle
```bash
ls -la "dist/Claude Code Switcher.app/Contents/Resources/"
# 应该看到: icon.icns
```

### 运行应用程序
- **macOS**: `open "dist/Claude Code Switcher.app"`
- **Windows**: `dist/claude-code-switcher.exe`
- **Linux**: `./dist/claude-code-switcher`

应用程序窗口和系统中都应该显示正确的图标。

## 🎯 图标设计说明

当前图标设计特点：
- **主题**: 代码编程主题
- **颜色**: 深色背景 (#2E3440) 配合蓝绿色渐变
- **元素**: 代码括号 `< >` 和斜杠 `/`，象征代码和配置
- **风格**: 现代化、简洁、专业

图标在不同尺寸下都保持良好的可读性和识别度。
