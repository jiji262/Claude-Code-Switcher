#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Final, fully-corrected version of the Claude Code Switcher.
// This version is self-contained, stable, and implements all requested features.

use eframe::{egui, App, Frame};
use serde_json::{from_str, to_string_pretty, Value};
use std::fs;
use std::path::PathBuf;
use directories::UserDirs;
use egui::{Color32, Id, TextEdit, RichText, Layout, Align, SidePanel};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG_DIR_NAME: &str = ".claude";  // Claude AI 配置目录
const ACTIVE_CONFIG_NAME: &str = "settings.json";
const APP_SETTINGS_NAME: &str = "app_settings.json";
const APP_DIR_NAME: &str = ".claude-code-switcher";  // 应用程序目录

const SETTINGS_SUBDIR: &str = "settings";  // 配置文件子目录

// --- Custom Toast Notification System ---
#[derive(Clone)]
enum ToastKind { Success, Error, Warning }

#[derive(Clone)]
struct Toast {
    kind: ToastKind,
    content: String,
    spawn_time: Instant,
    duration_secs: f32,
}

// --- 主题颜色定义 ---
#[derive(Clone, Copy)]
struct ThemeColors {
    base: Color32, mantle: Color32, crust: Color32, surface0: Color32, surface1: Color32,
    text: Color32, lavender: Color32, green: Color32, red: Color32, yellow: Color32,
    // 新增颜色用于更精致的UI
    accent: Color32, muted: Color32, border: Color32, highlight: Color32,
}

const CLAUDE_DARK: ThemeColors = ThemeColors {
    base: Color32::from_rgb(18, 18, 23),       // 更亮的深色背景
    crust: Color32::from_rgb(28, 28, 35),      // 侧边栏背景 - 更亮
    mantle: Color32::from_rgb(22, 22, 28),     // 编辑器背景 - 更亮
    surface0: Color32::from_rgb(40, 40, 48),   // 非交互元素 - 更亮
    surface1: Color32::from_rgb(50, 50, 60),   // 悬停状态 - 更亮
    text: Color32::from_rgb(248, 250, 252),    // 更亮的白色文字
    lavender: Color32::from_rgb(167, 139, 250), // 更亮的紫色主色调
    green: Color32::from_rgb(52, 211, 153),    // 更亮的绿色
    red: Color32::from_rgb(248, 113, 113),     // 更亮的红色
    yellow: Color32::from_rgb(251, 191, 36),   // 更亮的黄色
    accent: Color32::from_rgb(124, 126, 251),  // 更亮的强调色
    muted: Color32::from_rgb(156, 163, 175),   // 次要文字色
    border: Color32::from_rgb(75, 85, 99),     // 更亮的边框色
    highlight: Color32::from_rgb(45, 55, 72),  // 更亮的高亮背景
};

const CLAUDE_LIGHT: ThemeColors = ThemeColors {
    base: Color32::from_rgb(255, 255, 255),    // 纯白背景
    crust: Color32::from_rgb(248, 250, 252),   // 侧边栏背景 - 更亮的浅灰
    mantle: Color32::from_rgb(255, 255, 255),  // 编辑器背景 - 纯白
    surface0: Color32::from_rgb(241, 245, 249), // 非交互元素 - 更亮
    surface1: Color32::from_rgb(226, 232, 240), // 悬停状态 - 更明显
    text: Color32::from_rgb(15, 23, 42),       // 更深的文字色
    lavender: Color32::from_rgb(139, 92, 246), // 鲜艳紫色
    green: Color32::from_rgb(34, 197, 94),     // 鲜艳绿色
    red: Color32::from_rgb(239, 68, 68),       // 鲜艳红色
    yellow: Color32::from_rgb(245, 158, 11),   // 鲜艳黄色
    accent: Color32::from_rgb(99, 102, 241),   // 鲜艳强调色
    muted: Color32::from_rgb(100, 116, 139),   // 更深的次要文字色
    border: Color32::from_rgb(203, 213, 225),  // 更明显的边框色
    highlight: Color32::from_rgb(219, 234, 254), // 更明显的高亮背景
};

#[derive(Clone, Copy, PartialEq)]
enum Theme {
    Dark,
    Light,
}

// --- 应用设置结构 ---
#[derive(Serialize, Deserialize, Clone)]
struct AppSettings {
    config_directory: PathBuf,
    theme: String,
    default_config_file: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let default_dir = if let Some(user_dirs) = UserDirs::new() {
            user_dirs.home_dir().join(APP_DIR_NAME)
        } else {
            PathBuf::from(APP_DIR_NAME)
        };

        Self {
            config_directory: default_dir,
            theme: "Dark".to_string(),
            default_config_file: String::new(),
        }
    }
}

struct ConfigManagerApp {
    config_files: Vec<PathBuf>,
    selected_file: Option<PathBuf>,
    editor_content: String,
    config_dir: PathBuf,
    show_delete_confirmation: bool,
    char_count: usize,
    toasts: VecDeque<Toast>,
    status_text: String,
    show_rename_dialog: bool,
    new_file_name: String,
    current_theme: Theme,
    // 设置相关字段
    app_settings: AppSettings,
    app_settings_path: PathBuf,
    show_settings_dialog: bool,
    new_config_dir_input: String,
    // 新增字段
    is_content_modified: bool,
    original_content: String,
}

impl Default for ConfigManagerApp {
    fn default() -> Self {
        let app_settings = AppSettings::default();
        let app_settings_path = if let Some(user_dirs) = UserDirs::new() {
            user_dirs.home_dir().join(APP_DIR_NAME).join(APP_SETTINGS_NAME)
        } else {
            PathBuf::from(APP_DIR_NAME).join(APP_SETTINGS_NAME)
        };

        Self {
            config_files: Vec::new(),
            selected_file: None,
            editor_content: String::new(),
            status_text: "欢迎使用 Claude 配置管理器!".to_string(),
            config_dir: app_settings.config_directory.clone(),
            show_delete_confirmation: false,
            char_count: 0,
            toasts: VecDeque::new(),
            show_rename_dialog: false,
            new_file_name: String::new(),
            current_theme: Theme::Dark,
            app_settings,
            app_settings_path,
            show_settings_dialog: false,
            new_config_dir_input: String::new(),
            is_content_modified: false,
            original_content: String::new(),
        }
    }
}

impl ConfigManagerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("my_font".to_owned(), egui::FontData::from_static(include_bytes!("font.ttf")));
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) { family.insert(0, "my_font".to_owned()); }
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) { family.insert(0, "my_font".to_owned()); }
        cc.egui_ctx.set_fonts(fonts);

        let mut app = Self::default();
        app.load_app_settings();
        app.update_theme_style(cc);

        app.ensure_config_directory();
        app.new_config_dir_input = app.config_dir.to_string_lossy().to_string();
        app.refresh_file_list();
        app.sync_with_claude_config();
        app
    }

    fn show_toast(&mut self, content: impl Into<String>, kind: ToastKind) {
        self.toasts.push_back(Toast { kind, content: content.into(), spawn_time: Instant::now(), duration_secs: 3.0 });
    }
    fn set_status(&mut self, text: impl Into<String>) { self.status_text = text.into(); }

    // --- 应用设置相关方法 ---
    fn load_app_settings(&mut self) {
        if self.app_settings_path.exists() {
            match fs::read_to_string(&self.app_settings_path) {
                Ok(content) => {
                    match from_str::<AppSettings>(&content) {
                        Ok(settings) => {
                            self.app_settings = settings;
                            self.config_dir = self.app_settings.config_directory.clone();
                            // 根据保存的主题设置更新当前主题
                            self.current_theme = match self.app_settings.theme.as_str() {
                                "Light" => Theme::Light,
                                _ => Theme::Dark,
                            };
                        }
                        Err(e) => {
                            self.show_toast(format!("解析应用设置时出错: {}", e), ToastKind::Warning);
                        }
                    }
                }
                Err(_) => {
                    // 设置文件不存在或无法读取，使用默认设置
                    self.save_app_settings();
                }
            }
        } else {
            // 首次运行，保存默认设置
            self.save_app_settings();
        }
    }

    fn save_app_settings(&mut self) {
        // 更新设置中的主题
        self.app_settings.theme = match self.current_theme {
            Theme::Dark => "Dark".to_string(),
            Theme::Light => "Light".to_string(),
        };

        match to_string_pretty(&self.app_settings) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.app_settings_path, content) {
                    self.show_toast(format!("保存应用设置时出错: {}", e), ToastKind::Error);
                }
            }
            Err(e) => {
                self.show_toast(format!("序列化应用设置时出错: {}", e), ToastKind::Error);
            }
        }
    }

    fn ensure_config_directory(&mut self) {
        // 确保主配置目录存在
        if !self.config_dir.exists() {
            if let Err(e) = fs::create_dir_all(&self.config_dir) {
                self.show_toast(format!("创建配置目录时出错: {}", e), ToastKind::Error);
                return;
            }
        }

        // 确保 settings 子目录存在
        let settings_subdir = self.config_dir.join(SETTINGS_SUBDIR);
        if !settings_subdir.exists() {
            if let Err(e) = fs::create_dir_all(&settings_subdir) {
                self.show_toast(format!("创建 settings 子目录时出错: {}", e), ToastKind::Error);
                return;
            }
        }

        // 确保应用设置文件的父目录存在
        if let Some(parent) = self.app_settings_path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    self.show_toast(format!("创建应用设置目录时出错: {}", e), ToastKind::Error);
                }
            }
        }

        // 创建默认的 settings.json 文件在 settings 子目录中
        let settings_path = settings_subdir.join(ACTIVE_CONFIG_NAME);
        if !settings_path.exists() {
            let default_content = "{\n\t\"env\": {\n\t\t\"ANTHROPIC_API_KEY\": \"sk-ant-XXX\",\n\t\t\"ANTHROPIC_BASE_URL\": \"https://api.anthropic.com\"\n\t}\n}";
            if let Err(e) = fs::write(&settings_path, default_content) {
                self.show_toast(format!("创建默认 settings.json 时出错: {}", e), ToastKind::Error);
            }
        }
    }

    fn change_config_directory(&mut self, new_dir: PathBuf) {
        self.config_dir = new_dir.clone();
        self.app_settings.config_directory = new_dir;
        self.ensure_config_directory();
        self.refresh_file_list();
        self.selected_file = None;
        self.editor_content = String::new();
        self.save_app_settings();
        self.show_toast("配置目录已更改", ToastKind::Success);
    }
    
    fn get_theme_colors(&self) -> ThemeColors {
        match self.current_theme {
            Theme::Dark => CLAUDE_DARK,
            Theme::Light => CLAUDE_LIGHT,
        }
    }
    
    fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.current_theme = match self.current_theme {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
        self.update_theme_style_with_ctx(ctx);
        self.save_app_settings(); // 保存主题设置
    }
    
    fn create_custom_style(&self) -> egui::Style {
        let mut style = egui::Style::default();
        style.visuals = self.get_custom_visuals();
        
        // 更现代的间距设置
        style.spacing.item_spacing = egui::vec2(12.0, 8.0);
        style.spacing.button_padding = egui::vec2(16.0, 8.0);
        style.spacing.indent = 20.0;
        style.spacing.menu_margin = egui::Margin::same(8.0);
        style.spacing.combo_width = 120.0;
        
        // 现代化的圆角设计
        let rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.noninteractive.rounding = rounding;
        style.visuals.widgets.inactive.rounding = rounding;
        style.visuals.widgets.hovered.rounding = rounding;
        style.visuals.widgets.active.rounding = rounding;
        style.visuals.menu_rounding = rounding;
        style.visuals.window_rounding = egui::Rounding::same(8.0);
        
        // 文字大小调整
        style.override_text_style = Some(egui::TextStyle::Body);
        
        style
    }

    fn update_theme_style(&self, cc: &eframe::CreationContext<'_>) {
        cc.egui_ctx.set_style(self.create_custom_style());
    }

    fn update_theme_style_with_ctx(&self, ctx: &egui::Context) {
        ctx.set_style(self.create_custom_style());
    }
    
    fn get_button_color(&self, button_type: &str) -> Color32 {
        let colors = self.get_theme_colors();
        match button_type {
            "add" | "new" => colors.green,
            "save" => colors.accent,
            "rename" | "refresh" => colors.accent,
            "warning" | "format" | "settings" | "reset" => colors.yellow,
            "delete" | "danger" => colors.red,
            "switch" | "toggle" => colors.lavender,
            "default" => Color32::from_rgb(245, 158, 11), // 金色，突出重要性
            "primary" => colors.accent,
            "secondary" => colors.muted,
            _ => colors.muted,
        }
    }

    // 新增：获取按钮背景色的方法
    fn get_button_bg_color(&self, button_type: &str, is_hovered: bool, is_active: bool) -> Color32 {
        let colors = self.get_theme_colors();
        let base_color = self.get_button_color(button_type);
        
        if is_active {
            // 激活状态：颜色更深
            Color32::from_rgb(
                (base_color.r() as f32 * 0.8) as u8,
                (base_color.g() as f32 * 0.8) as u8,
                (base_color.b() as f32 * 0.8) as u8,
            )
        } else if is_hovered {
            // 悬停状态：颜色稍浅
            match self.current_theme {
                Theme::Dark => Color32::from_rgb(
                    (base_color.r() as f32 * 0.2 + colors.surface1.r() as f32 * 0.8) as u8,
                    (base_color.g() as f32 * 0.2 + colors.surface1.g() as f32 * 0.8) as u8,
                    (base_color.b() as f32 * 0.2 + colors.surface1.b() as f32 * 0.8) as u8,
                ),
                Theme::Light => Color32::from_rgb(
                    (base_color.r() as f32 * 0.1 + colors.surface1.r() as f32 * 0.9) as u8,
                    (base_color.g() as f32 * 0.1 + colors.surface1.g() as f32 * 0.9) as u8,
                    (base_color.b() as f32 * 0.1 + colors.surface1.b() as f32 * 0.9) as u8,
                ),
            }
        } else {
            // 默认状态：透明背景
            match self.current_theme {
                Theme::Dark => colors.surface0,
                Theme::Light => colors.base,
            }
        }
    }

    fn get_custom_visuals(&self) -> egui::Visuals {
        let colors = self.get_theme_colors();
        let mut visuals = match self.current_theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        };
        
        visuals.override_text_color = Some(colors.text);
        visuals.window_rounding = egui::Rounding::same(8.0);
        visuals.window_stroke = egui::Stroke::new(1.0, colors.border);
        visuals.window_shadow = egui::epaint::Shadow {
            extrusion: 16.0,
            color: match self.current_theme {
                Theme::Dark => Color32::from_black_alpha(100),
                Theme::Light => Color32::from_black_alpha(25),
            },
        };
        
        let rounding = egui::Rounding::same(6.0);
        
        // 非交互元素
        visuals.widgets.noninteractive.rounding = rounding;
        visuals.widgets.noninteractive.bg_fill = colors.surface0;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, colors.border);
        
        // 非激活状态
        visuals.widgets.inactive = egui::style::WidgetVisuals {
            bg_fill: colors.surface0,
            weak_bg_fill: colors.surface0,
            bg_stroke: egui::Stroke::new(1.0, colors.border),
            fg_stroke: egui::Stroke::new(1.0, colors.text),
            rounding,
            expansion: 0.0,
        };
        
        // 悬停状态 - 更明显的效果
        visuals.widgets.hovered = egui::style::WidgetVisuals {
            bg_fill: match self.current_theme {
                Theme::Dark => Color32::from_rgb(60, 60, 70),  // 更亮的悬停背景
                Theme::Light => Color32::from_rgb(219, 234, 254), // 更明显的浅色悬停
            },
            weak_bg_fill: colors.surface1,
            bg_stroke: egui::Stroke::new(2.0, colors.accent),
            fg_stroke: egui::Stroke::new(1.0, colors.text),
            rounding,
            expansion: 2.0,  // 增加扩展效果
        };
        
        // 激活状态
        visuals.widgets.active = egui::style::WidgetVisuals {
            bg_fill: colors.accent,
            weak_bg_fill: colors.accent,
            bg_stroke: egui::Stroke::new(2.0, colors.accent),
            fg_stroke: egui::Stroke::new(1.0, colors.base),
            rounding,
            expansion: 0.0,
        };
        
        // 选择样式
        visuals.selection.bg_fill = colors.highlight;
        visuals.selection.stroke = egui::Stroke::new(1.0, colors.accent);
        
        // 其他颜色
        visuals.hyperlink_color = colors.accent;
        visuals.error_fg_color = colors.red;
        visuals.warn_fg_color = colors.yellow;
        visuals.extreme_bg_color = colors.base;
        visuals.code_bg_color = colors.surface0;
        
        visuals
    }



    fn refresh_file_list(&mut self) {
        self.refresh_file_list_with_sort(true);
    }

    fn refresh_file_list_with_sort(&mut self, should_sort: bool) {
        self.config_files.clear();
        let settings_subdir = self.config_dir.join(SETTINGS_SUBDIR);
        
        if settings_subdir.exists() {
            if let Ok(entries) = fs::read_dir(&settings_subdir) {
                self.config_files = entries
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_file() &&
                        p.extension().map_or(false, |ext| ext == "json") &&
                        p.file_name().and_then(|s| s.to_str()).unwrap_or_default() != ACTIVE_CONFIG_NAME
                    })
                    .collect();

                if should_sort {
                    self.config_files.sort_by(|a, b| {
                        let a_name = a.file_name().and_then(|s| s.to_str()).unwrap_or_default();
                        let b_name = b.file_name().and_then(|s| s.to_str()).unwrap_or_default();
                        a_name.cmp(b_name)
                    });
                }

            } else {
                self.show_toast("无法读取配置目录。", ToastKind::Error);
            }
        }
    }

    // 保持当前文件顺序的刷新方法
    fn refresh_file_list_preserve_order(&mut self) {
        let current_order: Vec<String> = self.config_files
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
            .collect();
        
        // 先获取所有文件
        self.refresh_file_list_with_sort(false);
        
        // 如果有之前的顺序，尝试保持
        if !current_order.is_empty() {
            let mut ordered_files = Vec::new();
            let mut remaining_files = self.config_files.clone();
            
            // 按原顺序添加存在的文件
            for old_name in &current_order {
                if let Some(pos) = remaining_files.iter().position(|p| {
                    p.file_name().and_then(|n| n.to_str()) == Some(old_name)
                }) {
                    ordered_files.push(remaining_files.remove(pos));
                }
            }
            
            // 添加新文件到末尾
            ordered_files.extend(remaining_files);
            
            self.config_files = ordered_files;
        }
    }

    fn sync_with_claude_config(&mut self) {
        // 获取 Claude 配置文件路径
        let claude_config_dir = if let Some(user_dirs) = UserDirs::new() {
            user_dirs.home_dir().join(DEFAULT_CONFIG_DIR_NAME)
        } else {
            PathBuf::from(DEFAULT_CONFIG_DIR_NAME)
        };

        let claude_settings_path = claude_config_dir.join(ACTIVE_CONFIG_NAME);

        // 如果 Claude 配置文件不存在，跳过同步
        if !claude_settings_path.exists() {
            return;
        }

        // 读取 Claude 配置文件内容
        let claude_content = match fs::read_to_string(&claude_settings_path) {
            Ok(content) => content,
            Err(_) => return,
        };

        // 解析 Claude 配置为 JSON
        let claude_json: Value = match from_str(&claude_content) {
            Ok(json) => json,
            Err(_) => return,
        };

        // 检查是否有配置文件与 Claude 配置相同
        let mut found_matching_config = false;
        for file_path in &self.config_files {
            if let Ok(content) = fs::read_to_string(file_path) {
                if let Ok(json_val) = from_str::<Value>(&content) {
                    if json_val == claude_json {
                        // 找到匹配的配置，设为默认
                        let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
                        self.app_settings.default_config_file = file_name;
                        self.save_app_settings();
                        found_matching_config = true;
                        self.show_toast("已找到与 Claude 配置匹配的文件并设为默认", ToastKind::Success);
                        break;
                    }
                }
            }
        }

        // 如果没有找到匹配的配置，创建新的配置文件
        if !found_matching_config {
            let settings_subdir = self.config_dir.join(SETTINGS_SUBDIR);
            let mut new_path = settings_subdir.join("Claude默认配置.json");
            let mut i = 1;
            while new_path.exists() {
                new_path = settings_subdir.join(format!("Claude默认配置_{}.json", i));
                i += 1;
            }

            // 创建新配置文件，内容为 Claude 的配置
            match fs::write(&new_path, &claude_content) {
                Ok(_) => {
                    let file_name = new_path.file_name().unwrap().to_str().unwrap().to_string();
                    self.app_settings.default_config_file = file_name.clone();
                    self.save_app_settings();
                    self.refresh_file_list();
                    self.show_toast(format!("已创建新配置文件 '{}' 并设为默认", file_name), ToastKind::Success);
                }
                Err(e) => {
                    self.show_toast(format!("创建配置文件时出错: {}", e), ToastKind::Error);
                }
            }
        }
    }

    fn load_file_content(&mut self) {
        if let Some(path) = &self.selected_file {
            match fs::read_to_string(path) {
                Ok(content) => {
                    self.editor_content = content.clone();
                    self.original_content = content;
                    self.is_content_modified = false;
                    self.set_status(&format!("已加载 {}", path.file_name().unwrap_or_default().to_str().unwrap_or_default()));
                },
                Err(e) => {
                    self.show_toast(format!("读取文件时出错: {}", e), ToastKind::Error);
                    self.editor_content = String::new();
                    self.original_content = String::new();
                    self.is_content_modified = false;
                }
            }
        } else {
            self.editor_content = String::new();
            self.original_content = String::new();
            self.is_content_modified = false;
        }
    }

    fn save_current_file(&mut self) {
        if let Some(path) = &self.selected_file.clone() {
            match from_str::<Value>(&self.editor_content) {
                Ok(json_val) => {
                    let pretty_content = to_string_pretty(&json_val).unwrap_or_else(|_| self.editor_content.clone());
                    match fs::write(path, &pretty_content) {
                        Ok(_) => {
                            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
                            self.show_toast(format!("成功保存 {}", file_name), ToastKind::Success);
                            self.editor_content = pretty_content.clone();
                            self.original_content = pretty_content.clone();
                            self.is_content_modified = false;
                            
                            // 如果保存的是默认配置文件，同时更新到 Claude 配置文件
                            if self.app_settings.default_config_file == file_name {
                                let claude_config_dir = if let Some(user_dirs) = UserDirs::new() {
                                    user_dirs.home_dir().join(DEFAULT_CONFIG_DIR_NAME)
                                } else {
                                    PathBuf::from(DEFAULT_CONFIG_DIR_NAME)
                                };
                                
                                // 确保 Claude 配置目录存在
                                if !claude_config_dir.exists() {
                                    if let Err(e) = fs::create_dir_all(&claude_config_dir) {
                                        self.show_toast(format!("创建 Claude 配置目录时出错: {}", e), ToastKind::Error);
                                        return;
                                    }
                                }
                                
                                let claude_settings_path = claude_config_dir.join(ACTIVE_CONFIG_NAME);
                                
                                // 将内容写入 Claude 配置文件
                                match fs::write(&claude_settings_path, &pretty_content) {
                                    Ok(_) => {
                                        self.show_toast("已同步更新到 Claude 配置文件", ToastKind::Success);
                                    }
                                    Err(e) => {
                                        self.show_toast(format!("同步到 Claude 配置文件时出错: {}", e), ToastKind::Error);
                                    }
                                }
                            }
                        }
                        Err(e) => self.show_toast(format!("保存文件时出错: {}", e), ToastKind::Error),
                    }
                }
                Err(_e) => self.show_toast("JSON 格式无效.", ToastKind::Error),
            }
        }
    }

    fn format_json(&mut self) {
        if self.editor_content.trim().is_empty() {
            self.show_toast("编辑器内容为空", ToastKind::Warning);
            return;
        }

        match from_str::<Value>(&self.editor_content) {
            Ok(json_val) => {
                match to_string_pretty(&json_val) {
                    Ok(formatted) => {
                        self.editor_content = formatted;
                        self.show_toast("JSON 格式化成功", ToastKind::Success);
                    }
                    Err(e) => {
                        self.show_toast(format!("格式化失败: {}", e), ToastKind::Error);
                    }
                }
            }
            Err(e) => {
                self.show_toast(format!("JSON 格式无效: {}", e), ToastKind::Error);
            }
        }
    }

    fn add_new_config(&mut self) {
        let settings_subdir = self.config_dir.join(SETTINGS_SUBDIR);
        let mut new_path = settings_subdir.join("新配置.json");
        let mut i = 1;
        while new_path.exists() {
            new_path = settings_subdir.join(format!("新配置_{}.json", i));
            i += 1;
        }

        match fs::write(&new_path, "{\n\t\"env\": {\n\t\t\"ANTHROPIC_API_KEY\": \"sk-ant-XXX\",\n\t\t\"ANTHROPIC_BASE_URL\": \"https://api.anthropic.com\"\n\t}\n}") {
            Ok(_) => {
                let file_name = new_path.file_name().unwrap().to_str().unwrap().to_string();
                self.show_toast(format!("已创建新文件: {}", file_name), ToastKind::Success);
                self.refresh_file_list();
                self.selected_file = Some(new_path);
                self.load_file_content();
            }
            Err(e) => self.show_toast(format!("创建新文件时出错: {}", e), ToastKind::Error),
        }
    }

    fn delete_selected_file(&mut self) {
        if let Some(path) = self.selected_file.clone() {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            match fs::remove_file(&path) {
                Ok(_) => {
                    // 如果删除的是默认配置文件，清除默认设置
                    if self.app_settings.default_config_file == file_name {
                        self.app_settings.default_config_file = String::new();
                        self.save_app_settings();
                    }
                    self.show_toast(format!("已删除 {}", file_name), ToastKind::Success);
                    self.selected_file = None;
                    self.editor_content = String::new();
                    self.refresh_file_list();
                }
                Err(e) => self.show_toast(format!("删除文件时出错: {}", e), ToastKind::Error),
            }
        }
    }

    fn rename_selected_file(&mut self) {
        if let Some(selected_path) = self.selected_file.clone() {
            let old_file_name = selected_path.file_name().unwrap().to_str().unwrap();
            let mut new_name = self.new_file_name.trim().to_string();
            if !new_name.ends_with(".json") { new_name.push_str(".json"); }
            if new_name.is_empty() || new_name == ".json" { self.show_toast("文件名不能为空.", ToastKind::Error); return; }

            // 新路径需要在同一个 settings 子目录中
            let new_path = selected_path.parent().unwrap().join(&new_name);
            if new_path.exists() { self.show_toast("文件名已存在.", ToastKind::Error); return; }
            match fs::rename(&selected_path, &new_path) {
                Ok(_) => {
                    // 如果重命名的是默认配置文件，更新默认设置
                    if self.app_settings.default_config_file == old_file_name {
                        self.app_settings.default_config_file = new_name.clone();
                        self.save_app_settings();
                    }
                    self.show_toast(format!("文件已重命名为 \"{}\"", new_name), ToastKind::Success);
                    self.selected_file = Some(new_path);
                    self.show_rename_dialog = false;
                    self.refresh_file_list_preserve_order();
                }
                Err(e) => self.show_toast(format!("重命名文件时出错: {}", e), ToastKind::Error),
            }
        }
    }



    fn set_as_default(&mut self, file_path: PathBuf) {
        let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
        
        // 更新应用设置中的默认配置文件
        self.app_settings.default_config_file = file_name.clone();
        self.save_app_settings();
        
        // 将配置文件内容复制到 Claude 配置文件
        let claude_config_dir = if let Some(user_dirs) = UserDirs::new() {
            user_dirs.home_dir().join(DEFAULT_CONFIG_DIR_NAME)
        } else {
            PathBuf::from(DEFAULT_CONFIG_DIR_NAME)
        };
        
        // 确保 Claude 配置目录存在
        if !claude_config_dir.exists() {
            if let Err(e) = fs::create_dir_all(&claude_config_dir) {
                self.show_toast(format!("创建 Claude 配置目录时出错: {}", e), ToastKind::Error);
                return;
            }
        }
        
        let claude_settings_path = claude_config_dir.join(ACTIVE_CONFIG_NAME);
        
        // 从 settings 子目录读取配置文件
        match fs::read_to_string(&file_path) {
            Ok(content) => {
                match fs::write(&claude_settings_path, &content) {
                    Ok(_) => {
                        self.show_toast(format!("已将 '{}' 设为默认配置并复制到 Claude 配置文件", file_name), ToastKind::Success);
                        self.refresh_file_list();
                    }
                    Err(e) => self.show_toast(format!("写入 Claude 配置文件时出错: {}", e), ToastKind::Error),
                }
            }
            Err(e) => self.show_toast(format!("读取配置文件时出错: {}", e), ToastKind::Error),
        }
    }
}

impl App for ConfigManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let colors = self.get_theme_colors();

        // 处理快捷键
        if ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command) {
            if self.selected_file.is_some() {
                self.save_current_file();
            }
        }

        egui::CentralPanel::default().frame(egui::Frame::default().fill(colors.base)).show(ctx, |ui| {
            egui::TopBottomPanel::bottom("status_bar").frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(10.0, 5.0)).fill(colors.crust)).show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status_text);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(format!("字符: {}", self.char_count));
                    });
                });
            });
            SidePanel::left("file_list_panel")
                .frame(egui::Frame::default()
                    .fill(colors.crust)
                    .stroke(egui::Stroke::new(1.0, colors.surface0))
                )
                .exact_width(200.0)
                .resizable(false)
                .show_inside(ui, |ui| {
                // 顶部标题栏 - 与右侧高度保持一致
                egui::TopBottomPanel::top("side_panel_title").frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(12.0, 8.0)).outer_margin(egui::Margin::ZERO).fill(colors.crust).stroke(egui::Stroke::NONE)).show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.label(RichText::new("◈ 配置文件").size(15.0).color(colors.text).strong());
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // 添加透明占位按钮，确保与右侧面板高度一致
                            ui.add_enabled(false, egui::Button::new(RichText::new("").size(12.0)).fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE));
                            ui.add_enabled(false, egui::Button::new(RichText::new("").size(12.0)).fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::NONE));
                        });
                    });

                });

                // 文件列表区域（包含按钮）
                let mut selection_changed = false;
                let mut selected_path = self.selected_file.clone();
                let mut actions_to_perform = Vec::new();
                
                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    ui.vertical(|ui| {
                        // 新增配置和刷新按钮 - 使用最简单的方法
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0; // 移除水平间距
                            
                            // 背景色填充
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, egui::Rounding::ZERO, colors.crust);
                            
                            // 重新定位到矩形内部进行按钮布局
                            ui.allocate_ui_at_rect(rect.shrink2(egui::vec2(12.0, 8.0)), |ui| {
                                ui.horizontal(|ui| {
                                    let button_width = (ui.available_width() - 8.0) / 2.0;
                                    
                                    if ui.add_sized([button_width, 28.0], 
                                        egui::Button::new(RichText::new("+ 新建").color(self.get_button_color("add")).size(13.0))
                                    ).clicked() {
                                        self.add_new_config();
                                    }
                                    
                                    ui.add_space(8.0);
                                    
                                    if ui.add_sized([button_width, 28.0], 
                                        egui::Button::new(RichText::new("↻ 刷新").color(self.get_button_color("refresh")).size(13.0))
                                    ).clicked() {
                                        self.refresh_file_list();
                                        self.sync_with_claude_config();
                                        self.show_toast("文件列表已刷新", ToastKind::Success);
                                    }
                                });
                            });
                        });

                        // 分隔线
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                        for (index, path) in self.config_files.iter().map(|p| p.as_path()).enumerate() {
                            let file_name = path.file_name().unwrap().to_str().unwrap();
                            let is_default_file = self.app_settings.default_config_file == file_name;

                            // 隔行背景色 - 更明显的区分
                            let bg_color = if index % 2 == 0 {
                                colors.crust
                            } else {
                                match self.current_theme {
                                    Theme::Dark => Color32::from_rgb(32, 35, 42),  // 更深的对比色
                                    Theme::Light => Color32::from_rgb(241, 245, 249), // 更明显的浅色
                                }
                            };

                            // 文件项容器
                            egui::Frame::default()
                                .fill(bg_color)
                                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        // 文件名部分
                                        let file_text = if is_default_file {
                                            RichText::new(format!("★ {} (默认)", file_name)).color(self.get_button_color("default")).strong().size(13.5)
                                        } else {
                                            RichText::new(file_name).size(13.0).color(colors.text)
                                        };

                                        if ui.selectable_label(self.selected_file.as_deref() == Some(path), file_text).clicked() {
                                            selected_path = Some(path.to_path_buf());
                                            selection_changed = true;
                                        }

                                        // 操作按钮区域 - 居中对齐
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                                // 使用可用空间来居中按钮
                                                let total_button_width = 180.0; // 三个按钮的总宽度估算
                                                let available_width = 200.0; // 固定面板宽度
                                                let padding = (available_width - total_button_width) / 2.0;
                                                
                                                if padding > 0.0 {
                                                    ui.add_space(padding);
                                                }
                                                
                                                if ui.button(RichText::new("✎ 重命名").color(self.get_button_color("rename")).size(11.0)).clicked() {
                                                    actions_to_perform.push(('r', index));
                                                }
                                                ui.add_space(6.0);
                                                if ui.button(RichText::new("× 删除").color(self.get_button_color("delete")).size(11.0)).clicked() {
                                                    actions_to_perform.push(('d', index));
                                                }
                                                ui.add_space(6.0);
                                                if ui.add_enabled(!is_default_file, egui::Button::new(RichText::new("★ 设为默认").color(self.get_button_color("default")).size(11.0))).clicked() {
                                                    actions_to_perform.push(('s', index));
                                                }
                                            });
                                        });
                                    });
                                });

                            ui.add_space(8.0); // 文件之间的间距
                        }
                    });
                });
                
                // 处理收集的操作
                for (action_type, index) in actions_to_perform {
                    if let Some(path) = self.config_files.get(index) {
                        match action_type {
                            'd' => {
                                self.selected_file = Some(path.clone());
                                self.show_delete_confirmation = true;
                            }
                            'r' => {
                                self.selected_file = Some(path.clone());
                                self.new_file_name = path.file_name().unwrap().to_str().unwrap().to_string();
                                self.show_rename_dialog = true;
                            }
                            's' => {
                                self.set_as_default(path.clone());
                            }
                            _ => {}
                        }
                    }
                }
                
                if selection_changed {
                    self.selected_file = selected_path;
                    self.load_file_content();
                }


            });
            egui::CentralPanel::default().frame(egui::Frame::default().fill(colors.mantle)).show_inside(ui, |ui| {
                // 顶部标题栏 - 与左侧高度保持一致
                egui::TopBottomPanel::top("editor_title_panel")
                    .frame(egui::Frame::default()
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .fill(colors.crust)
                        .stroke(egui::Stroke::NONE)
                    )
                    .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            if let Some(path) = &self.selected_file {
                                let file_name = path.file_name().unwrap().to_str().unwrap();
                                let display_text = if self.is_content_modified {
                                    format!("◉ {} (未保存)", file_name)
                                } else {
                                    format!("◇ {}", file_name)
                                };
                                let text_color = if self.is_content_modified {
                                    self.get_button_color("warning")
                                } else {
                                    colors.text
                                };
                                ui.label(RichText::new(display_text).size(15.0).color(text_color).strong());
                            } else {
                                ui.label(RichText::new("请选择文件").size(14.0).color(colors.text));
                            }
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // 添加设置按钮
                            if ui.button(RichText::new("⚙ 设置").size(12.0).color(self.get_button_color("settings"))).clicked() {
                                self.show_settings_dialog = true;
                            }
                            ui.separator();
                            if ui.add_enabled(self.selected_file.is_some(), egui::Button::new(RichText::new("💾 保存").color(self.get_button_color("save")).size(12.0))).clicked() {
                                self.save_current_file();
                            }
                            if ui.add_enabled(self.selected_file.is_some(), egui::Button::new(RichText::new("◊ 格式化").color(self.get_button_color("format")).size(12.0))).clicked() {
                                self.format_json();
                            }
                        });

                    });
                });

                // 编辑器内容区域
                egui::CentralPanel::default().frame(egui::Frame::default().fill(colors.mantle).inner_margin(egui::Margin { left: 12.0, right: 12.0, top: 10.0, bottom: 10.0 })).show_inside(ui, |ui| {
                    if self.selected_file.is_some() {
                        // 使用水平布局来分离行号和编辑器
                        ui.horizontal_top(|ui| {
                            // 计算行数和行号宽度
                            let line_count = self.editor_content.lines().count().max(1);
                            let line_number_width = 48.0;  // 调整为原来的80% (60 * 0.8 = 48)

                            // 行号区域 - 固定宽度
                            ui.allocate_ui_with_layout(
                                egui::Vec2::new(line_number_width, ui.available_height()),
                                egui::Layout::top_down(egui::Align::RIGHT),
                                |ui| {
                                    // 行号背景 - 与代码区域背景一致
                                    let rect = ui.available_rect_before_wrap();
                                    ui.painter().rect_filled(rect, egui::Rounding::ZERO, colors.mantle);

                                    // 简单的行号显示，不使用滚动区域
                                    ui.add_space(10.0);
                                    for line_num in 1..=line_count {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                            ui.add_space(8.0);
                                            ui.label(
                                                RichText::new(format!("{}", line_num))
                                                    .font(egui::FontId::monospace(13.0))
                                                    .color(Color32::from_rgb(120, 120, 120))
                                            );
                                        });
                                    }
                                }
                            );

                            // 编辑器区域
                            ui.allocate_ui_with_layout(
                                egui::Vec2::new(ui.available_width(), ui.available_height()),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    let scroll_area = egui::ScrollArea::vertical()
                                        .auto_shrink([false; 2])
                                        .id_source("main_editor_scroll");

                                    scroll_area.show(ui, |ui| {

                                        // 自定义JSON语法高亮（不包含行号）
                                        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                                            let mut job = egui::text::LayoutJob::default();
                                            job.wrap.max_width = wrap_width;

                                            // 定义颜色
                                            let (key_color, string_color, number_color, keyword_color, punctuation_color) =
                                                if matches!(self.current_theme, Theme::Dark) {
                                                    (
                                                        Color32::from_rgb(156, 220, 254), // 浅蓝色 - 键名
                                                        Color32::from_rgb(206, 145, 120), // 橙色 - 字符串值
                                                        Color32::from_rgb(181, 206, 168), // 浅绿色 - 数字
                                                        Color32::from_rgb(197, 134, 192), // 紫色 - 关键字
                                                        Color32::from_rgb(212, 212, 212), // 浅灰色 - 标点
                                                    )
                                                } else {
                                                    (
                                                        Color32::from_rgb(0, 92, 197),    // 深蓝色 - 键名
                                                        Color32::from_rgb(163, 21, 21),   // 深红色 - 字符串值
                                                        Color32::from_rgb(9, 134, 88),    // 深绿色 - 数字
                                                        Color32::from_rgb(111, 66, 193),  // 深紫色 - 关键字
                                                        Color32::from_rgb(80, 80, 80),    // 深灰色 - 标点
                                                    )
                                                };

                                            let mut chars = string.char_indices().peekable();
                                            let mut in_string = false;
                                            let mut in_key = false;
                                            let mut current_start = 0;
                                            let mut current_color = colors.text;

                                            while let Some((i, ch)) = chars.next() {
                                                match ch {
                                                    '"' => {
                                                        // 添加当前段落
                                                        if i > current_start {
                                                            job.append(&string[current_start..i], 0.0, egui::TextFormat {
                                                                color: current_color,
                                                                font_id: egui::FontId::monospace(14.0),
                                                                ..Default::default()
                                                            });
                                                        }

                                                        if !in_string {
                                                            // 开始字符串，检查是否是键名
                                                            in_string = true;
                                                            // 向前查找冒号来判断是否是键名
                                                            let rest_of_string = &string[i..];
                                                            if let Some(quote_end) = rest_of_string[1..].find('"') {
                                                                let after_quote = &rest_of_string[quote_end + 2..];
                                                                in_key = after_quote.trim_start().starts_with(':');
                                                            }
                                                            current_color = if in_key { key_color } else { string_color };
                                                        } else {
                                                            // 结束字符串
                                                            in_string = false;
                                                            in_key = false;
                                                            current_color = colors.text;
                                                        }

                                                        // 添加引号
                                                        job.append(&ch.to_string(), 0.0, egui::TextFormat {
                                                            color: current_color,
                                                            font_id: egui::FontId::monospace(14.0),
                                                            ..Default::default()
                                                        });

                                                        current_start = i + ch.len_utf8();
                                                    }
                                                    '{' | '}' | '[' | ']' | ',' | ':' if !in_string => {
                                                        // 添加当前段落
                                                        if i > current_start {
                                                            job.append(&string[current_start..i], 0.0, egui::TextFormat {
                                                                color: current_color,
                                                                font_id: egui::FontId::monospace(14.0),
                                                                ..Default::default()
                                                            });
                                                        }

                                                        // 添加标点符号
                                                        job.append(&ch.to_string(), 0.0, egui::TextFormat {
                                                            color: punctuation_color,
                                                            font_id: egui::FontId::monospace(14.0),
                                                            ..Default::default()
                                                        });

                                                        current_start = i + ch.len_utf8();
                                                        current_color = colors.text;
                                                    }
                                                    _ if !in_string && (ch.is_ascii_digit() || ch == '-') => {
                                                        // 数字开始
                                                        if current_color != number_color {
                                                            // 添加当前段落
                                                            if i > current_start {
                                                                job.append(&string[current_start..i], 0.0, egui::TextFormat {
                                                                    color: current_color,
                                                                    font_id: egui::FontId::monospace(14.0),
                                                                    ..Default::default()
                                                                });
                                                            }
                                                            current_start = i;
                                                            current_color = number_color;
                                                        }
                                                    }
                                                    _ if !in_string && ch.is_ascii_alphabetic() => {
                                                        // 检查关键字 (true, false, null)
                                                        let word_start = i;
                                                        let mut word_end = i + ch.len_utf8();

                                                        // 找到单词结束
                                                        let remaining = &string[word_end..];
                                                        for (_offset, next_ch) in remaining.char_indices() {
                                                            if next_ch.is_ascii_alphanumeric() {
                                                                word_end += next_ch.len_utf8();
                                                            } else {
                                                                break;
                                                            }
                                                        }

                                                        let word = &string[word_start..word_end];
                                                        let is_keyword = matches!(word, "true" | "false" | "null");

                                                        if is_keyword && current_color != keyword_color {
                                                            // 添加当前段落
                                                            if i > current_start {
                                                                job.append(&string[current_start..i], 0.0, egui::TextFormat {
                                                                    color: current_color,
                                                                    font_id: egui::FontId::monospace(14.0),
                                                                    ..Default::default()
                                                                });
                                                            }
                                                            current_start = i;
                                                            current_color = keyword_color;
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }

                                            // 添加剩余部分
                                            if current_start < string.len() {
                                                job.append(&string[current_start..], 0.0, egui::TextFormat {
                                                    color: current_color,
                                                    font_id: egui::FontId::monospace(14.0),
                                                    ..Default::default()
                                                });
                                            }

                                            ui.fonts(|f| f.layout_job(job))
                                        };

                                        ui.add_space(10.0);
                                        let editor = TextEdit::multiline(&mut self.editor_content)
                                            .id(Id::new("main_editor"))
                                            .font(egui::FontId::monospace(14.0))
                                            .code_editor()
                                            .desired_width(f32::INFINITY)
                                            .frame(false)
                                            .layouter(&mut layouter);
                                        let response = ui.add(editor);

                                        // 检测内容是否修改
                                        if response.changed() {
                                            self.is_content_modified = self.editor_content != self.original_content;
                                        }
                                    });
                                }
                            );
                        });

                        self.char_count = self.editor_content.chars().count();
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("请选择一个配置文件进行编辑").size(16.0).color(colors.text.linear_multiply(0.7)));
                        });
                    }
                });
            });

            egui::Area::new("toast_area").anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -10.0)).show(ctx, |ui| {
                self.toasts.retain(|toast| toast.spawn_time.elapsed().as_secs_f32() < toast.duration_secs);
                for toast in self.toasts.iter() {
                    let (label_text, color) = match toast.kind {
                        ToastKind::Success => ("[成功]", colors.green),
                        ToastKind::Error => ("[错误]", colors.red),
                        ToastKind::Warning => ("[警告]", colors.yellow),
                    };
                    let frame = egui::Frame::default().inner_margin(8.0).rounding(6.0).fill(colors.crust).stroke(egui::Stroke::new(1.0, color));
                    frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(label_text).color(color).strong());
                            ui.add_space(5.0);
                            ui.label(&toast.content);
                        });
                    });
                    ui.add_space(5.0);
                }
            });
            ctx.request_repaint_after(Duration::from_millis(100));
        });

        // 模态弹框 - 简单有效的方案
        if self.show_rename_dialog || self.show_delete_confirmation || self.show_settings_dialog {
            // 全屏半透明背景
            ctx.layer_painter(egui::LayerId::background()).rect_filled(
                ctx.screen_rect(),
                egui::Rounding::ZERO,
                egui::Color32::from_black_alpha(100)
            );
        }

        if self.show_rename_dialog {
            egui::Window::new("重命名文件")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("请输入新的文件名:");
                    ui.text_edit_singleline(&mut self.new_file_name);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("✓ 确认").color(self.get_button_color("primary"))).clicked() { self.rename_selected_file(); }
                        if ui.button(RichText::new("✗ 取消").color(self.get_button_color("secondary"))).clicked() { self.show_rename_dialog = false; }
                    });
                });
        }

        if self.show_delete_confirmation {
            egui::Window::new("确认删除")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("您确定要删除这个配置文件吗？此操作无法撤销。");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("× 确认删除").color(colors.red)).clicked() { self.delete_selected_file(); self.show_delete_confirmation = false; }
                        if ui.button(RichText::new("✗ 取消").color(self.get_button_color("secondary"))).clicked() { self.show_delete_confirmation = false; }
                    });
                });
        }

        if self.show_settings_dialog {
            egui::Window::new("应用设置")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.add_space(5.0);

                        // 配置目录设置
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("配置文件目录").strong());
                                ui.add_space(5.0);

                                ui.horizontal(|ui| {
                                    ui.label("当前目录:");
                                    ui.label(RichText::new(self.config_dir.to_string_lossy()).color(colors.lavender));
                                });

                                ui.add_space(5.0);
                                ui.label("新目录路径:");
                                ui.text_edit_singleline(&mut self.new_config_dir_input);

                                ui.add_space(5.0);
                                ui.horizontal(|ui| {
                                    if ui.button(RichText::new("□ 选择文件夹").color(colors.green)).clicked() {
                                        // 使用rfd打开文件夹选择对话框
                                        let current_dir = if self.config_dir.exists() {
                                            Some(self.config_dir.clone())
                                        } else if let Some(user_dirs) = UserDirs::new() {
                                            Some(user_dirs.home_dir().to_path_buf())
                                        } else {
                                            None
                                        };

                                        if let Some(folder) = rfd::FileDialog::new()
                                            .set_title("选择配置文件目录")
                                            .set_directory(current_dir.unwrap_or_else(|| PathBuf::from(".")))
                                            .pick_folder() {
                                            self.new_config_dir_input = folder.to_string_lossy().to_string();
                                        }
                                    }

                                    if ui.button(RichText::new("↻ 重置").color(self.get_button_color("reset"))).clicked() {
                                        if let Some(user_dirs) = UserDirs::new() {
                                            self.new_config_dir_input = user_dirs.home_dir().join(APP_DIR_NAME).to_string_lossy().to_string();
                                        }
                                    }
                                });
                            });
                        });

                        ui.add_space(10.0);

                        // 主题设置
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("主题设置").strong());
                                ui.add_space(5.0);

                                ui.horizontal(|ui| {
                                    ui.label("当前主题:");
                                    let theme_name = match self.current_theme {
                                        Theme::Dark => "深色主题",
                                        Theme::Light => "浅色主题",
                                    };
                                    ui.label(RichText::new(theme_name).color(colors.lavender));
                                });

                                ui.add_space(5.0);
                                if ui.button(RichText::new("◐ 切换主题").color(self.get_button_color("toggle"))).clicked() {
                                    self.toggle_theme(ctx);
                                }
                            });
                        });

                        ui.add_space(15.0);

                        // 按钮区域
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("✓ 应用更改").color(colors.green)).clicked() {
                                let new_path = PathBuf::from(&self.new_config_dir_input);
                                if new_path != self.config_dir {
                                    self.change_config_directory(new_path);
                                }
                                self.show_settings_dialog = false;
                            }

                            if ui.button(RichText::new("✗ 取消").color(self.get_button_color("secondary"))).clicked() {
                                self.new_config_dir_input = self.config_dir.to_string_lossy().to_string();
                                self.show_settings_dialog = false;
                            }
                        });
                    });
                });
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    // 加载应用图标
    let icon_data = include_bytes!("../resources/icons/icon.svg");
    let icon_image = load_icon_from_svg(icon_data);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(icon_image),
        ..Default::default()
    };
    eframe::run_native("Claude Code Switcher", options, Box::new(|cc| Box::new(ConfigManagerApp::new(cc))))
}

// 从 SVG 数据加载图标
fn load_icon_from_svg(svg_data: &[u8]) -> egui::IconData {
    use resvg::usvg::{self, TreeParsing};
    use tiny_skia::Pixmap;

    let size = 64; // 使用 64x64 像素的图标

    // 尝试解析 SVG 并渲染为位图
    if let Ok(svg_str) = std::str::from_utf8(svg_data) {
        let options = usvg::Options::default();
        if let Ok(usvg_tree) = usvg::Tree::from_str(svg_str, &options) {
            if let Some(mut pixmap) = Pixmap::new(size, size) {
                let transform = tiny_skia::Transform::from_scale(
                    size as f32 / usvg_tree.size.width(),
                    size as f32 / usvg_tree.size.height(),
                );

                let resvg_tree = resvg::Tree::from_usvg(&usvg_tree);
                resvg_tree.render(transform, &mut pixmap.as_mut());

                // 转换为 RGBA 格式
                let pixels = pixmap.data();
                let mut rgba = Vec::with_capacity(pixels.len());

                // tiny-skia 使用 BGRA 格式，需要转换为 RGBA
                for chunk in pixels.chunks_exact(4) {
                    rgba.push(chunk[2]); // R
                    rgba.push(chunk[1]); // G
                    rgba.push(chunk[0]); // B
                    rgba.push(chunk[3]); // A
                }

                return egui::IconData {
                    rgba,
                    width: size,
                    height: size,
                };
            }
        }
    }

    // 如果 SVG 解析失败，创建一个简单的备用图标
    create_fallback_icon(size)
}

// 创建备用图标
fn create_fallback_icon(size: u32) -> egui::IconData {
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    // 创建一个简单的渐变图标
    for y in 0..size {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            let center_x = size as f32 / 2.0;
            let center_y = size as f32 / 2.0;
            let distance = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
            let max_distance = (center_x.powi(2) + center_y.powi(2)).sqrt();
            let intensity = (1.0 - distance / max_distance).max(0.0);

            // 使用类似 SVG 中的颜色 (#2E3440 到 #5E81AC)
            rgba[i] = (46 as f32 * (1.0 - intensity) + 94 as f32 * intensity) as u8; // R
            rgba[i + 1] = (52 as f32 * (1.0 - intensity) + 129 as f32 * intensity) as u8; // G
            rgba[i + 2] = (64 as f32 * (1.0 - intensity) + 172 as f32 * intensity) as u8; // B
            rgba[i + 3] = 255; // A
        }
    }

    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}
