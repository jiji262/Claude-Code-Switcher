# Claude Code Switcher

用于快速切换 Claude Code 配置的桌面应用程序。

## 功能特性

- 🔄 **配置切换**：一键切换不同的 Claude Code 配置
- 📁 **配置管理**：创建、编辑、删除多个配置文件
- 🎨 **主题切换**：支持深色和浅色主题
- 📝 **JSON 编辑**：内置 JSON 编辑器，支持语法高亮和格式化
- ⚙️ **设置管理**：自定义配置文件目录和应用设置
- 🎯 **统一图标**：使用 `resources/icons/icon.svg` 作为应用图标，在所有平台和位置统一显示

## 快速开始

### 从源码构建

#### 前置要求
- Rust 1.70+ (推荐使用 [rustup](https://rustup.rs/) 安装)

#### 构建步骤
```bash
# 克隆或下载源码后进入项目目录
cd claude-code-switcher

# 生成应用图标 (可选，已预生成)
./generate_icons.sh

# 构建 release 版本
cargo build --release

# 运行程序
cargo run --release
```

#### 平台特定构建

**Windows:**
```bash
# 使用提供的构建脚本
./build.bat

# 或手动构建
cargo build --release
```

**macOS (Apple Silicon):**
```bash
# 使用提供的构建脚本
./build-macos.sh

# 或手动构建
cargo build --release --target aarch64-apple-darwin
```

**Linux:**
```bash
# 使用提供的构建脚本
./build.sh

# 或手动构建
cargo build --release
```

## 使用说明

### 配置文件管理
- **默认配置目录**: `~/.claude/`
- **主配置文件**: `settings.json` (Claude Code 使用的活动配置)
- **备用配置**: 可创建多个 `.json` 配置文件进行管理

### 界面操作
- **左侧面板**: 配置文件列表和管理按钮
- **右侧面板**: JSON 编辑器和工具栏
- **状态栏**: 显示当前状态和字符计数

### 快捷键和按钮
- `[+] 新增`: 创建新的配置文件
- `[R] 重命名`: 重命名选中的配置文件
- `[-] 删除`: 删除选中的配置文件
- `[*] 切换配置`: 将选中配置应用为活动配置
- `[S] 保存`: 保存当前编辑的文件
- `[F] 美化JSON`: 格式化 JSON 内容
- `[R] 重新加载`: 重新加载文件内容
- `[⚙] 设置`: 打开应用设置

## 技术栈

- **Rust** - 系统编程语言，保证性能和安全性
- **egui** - 跨平台 GUI 框架
- **serde** - JSON 序列化/反序列化
- **rfd** - 原生文件对话框
- **directories** - 跨平台目录管理

## 系统要求

- **Windows**: Windows 10+
- **macOS**: macOS 10.15+ (Catalina 或更高版本)
- **Linux**: 支持现代 Linux 发行版
- **内存**: 建议 4GB 以上

## 故障排除

### macOS 权限问题
如果遇到"无法打开，因为无法验证开发者"的错误：
```bash
# 方法一：通过系统偏好设置允许
# 系统偏好设置 > 安全性与隐私 > 通用 > 允许从以下位置下载的应用

# 方法二：使用命令行移除隔离属性
xattr -d com.apple.quarantine ./claude-code-switcher
```

### 配置文件位置
默认配置文件位于 `~/.claude/settings.json`，如果 Claude Code 使用不同路径，可在应用设置中修改。

## 许可证

- **应用程序**: MIT OR Apache-2.0
- **字体文件**: SIL Open Font License 1.1

---

**Claude Code Switcher** - 让 Claude Code 配置切换更简单 🚀
