# Claude Code Switcher - macOS 应用程序

用于快速切换 Claude Code 配置的桌面应用程序。

## 安装

将 `Claude Code Switcher.app` 拖拽到 `/Applications` 文件夹即可。

## 首次运行

由于应用程序未经过 Apple 公证，首次运行时可能会遇到安全提示：

**方法一**：
1. 右键点击应用程序，选择"打开"
2. 在弹出的对话框中点击"打开"

**方法二**：
1. 前往 系统偏好设置 > 安全性与隐私 > 通用
2. 点击"仍要打开"按钮

**方法三**（命令行）：
```bash
xattr -d com.apple.quarantine "Claude Code Switcher.app"
```

## 功能特性

- 🔄 **配置切换**：一键切换不同的 Claude Code 配置
- 📁 **配置管理**：创建、编辑、删除多个配置文件
- 🎨 **主题切换**：支持深色和浅色主题
- 📝 **JSON 编辑**：内置 JSON 编辑器，支持语法高亮和格式化
- ⚙️ **设置管理**：自定义配置文件目录和应用设置

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

## 系统要求

- **macOS**: 10.15 (Catalina) 或更高版本
- **架构**: Apple Silicon (M1/M2/M3) 或 Intel (通过 Rosetta 2)
- **内存**: 建议 4GB 以上
- **磁盘空间**: 约 25MB

## 卸载

要卸载应用程序，只需删除应用程序包：
```bash
rm -rf "/Applications/Claude Code Switcher.app"
```

## 技术信息

- **版本**: 0.1.2
- **Bundle ID**: com.claudecode.switcher
- **开发语言**: Rust + egui
- **许可证**: MIT OR Apache-2.0

---

**Claude Code Switcher** - 让 Claude Code 配置切换更简单 🚀
