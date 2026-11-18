#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod probe;
mod state;

use probe::{probe_gateway, ProbeResult};
use resvg::usvg::{self, TreeParsing};
use serde::Deserialize;
use serde_json::Value;
use state::WorkspaceManager;
use std::borrow::Cow;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Icon, Theme, WindowBuilder},
};
use tiny_skia::Pixmap;
use wry::{
    http::{header::CONTENT_TYPE, Request, Response},
    WebViewBuilder,
};

#[cfg(target_os = "macos")]
use tao::{dpi::LogicalPosition, platform::macos::WindowBuilderExtMacOS};

const ASSET_INDEX_HTML: &[u8] = include_bytes!("../ui/index.html");
const ASSET_TOKENS_CSS: &[u8] = include_bytes!("../ui/styles/tokens.css");
const ASSET_APP_CSS: &[u8] = include_bytes!("../ui/styles/app.css");
const ASSET_ICONS_JS: &[u8] = include_bytes!("../ui/scripts/icons.js");
const ASSET_PRESETS_JS: &[u8] = include_bytes!("../ui/scripts/presets.js");
const ASSET_DIFF_JS: &[u8] = include_bytes!("../ui/scripts/diff.js");
const ASSET_APP_JS: &[u8] = include_bytes!("../ui/scripts/app.js");
const ASSET_ICON_SVG: &[u8] = include_bytes!("../resources/icons/icon.svg");

#[derive(Debug, Deserialize)]
struct IpcEnvelope {
    id: u64,
    cmd: String,
    #[serde(default)]
    payload: Value,
}

enum UserEvent {
    IpcRequest(IpcEnvelope),
    ProbeFinished {
        id: u64,
        file_name: String,
        result: ProbeResult,
    },
}

fn tao_theme_from_setting(theme: &str) -> Option<Theme> {
    match theme.trim().to_ascii_lowercase().as_str() {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None, // "System" follows OS native appearance
    }
}

#[cfg(windows)]
fn attach_windows_parent_console_if_cli() {
    if std::env::args().len() > 1 {
        extern "system" {
            fn AttachConsole(dw_process_id: u32) -> i32;
        }
        const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

fn main() {
    #[cfg(windows)]
    attach_windows_parent_console_if_cli();

    if let Some(code) = state::run_cli_if_requested() {
        std::process::exit(code);
    }

    if let Err(e) = run_desktop_app() {
        eprintln!("Claude Code Switcher 启动失败: {}", e);
        std::process::exit(1);
    }
}

fn run_desktop_app() -> wry::Result<()> {
    let manager = Arc::new(Mutex::new(WorkspaceManager::new_default()));
    let initial_theme_pref = manager
        .lock()
        .map(|m| m.app_settings.theme.clone())
        .unwrap_or_else(|_| "Dark".to_string());
    let initial_tao_theme = tao_theme_from_setting(&initial_theme_pref);

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    #[cfg(target_os = "macos")]
    let _menu = build_macos_native_menu();

    let mut win_builder = WindowBuilder::new()
        .with_title("Claude Code Switcher")
        .with_inner_size(LogicalSize::new(1060.0, 720.0))
        .with_min_inner_size(LogicalSize::new(760.0, 520.0))
        .with_theme(initial_tao_theme)
        .with_window_icon(load_window_icon(ASSET_ICON_SVG));

    #[cfg(target_os = "macos")]
    {
        win_builder = win_builder
            .with_titlebar_transparent(true)
            .with_title_hidden(true)
            .with_fullsize_content_view(true)
            .with_traffic_light_inset(LogicalPosition::new(15.0, 15.0))
            .with_automatic_window_tabbing(false);
    }

    let window = win_builder.build(&event_loop).expect("创建应用窗口失败");

    let resolved_light = match initial_tao_theme {
        Some(Theme::Light) => true,
        Some(_) => false,
        None => matches!(window.theme(), Theme::Light),
    };
    let initial_bg = if resolved_light {
        (248, 246, 241, 255)
    } else {
        (21, 22, 26, 255)
    };

    let ipc_proxy = proxy.clone();
    let webview_builder = WebViewBuilder::new()
        .with_background_color(initial_bg)
        .with_accept_first_mouse(true)
        .with_custom_protocol("ccs".into(), move |_webview_id, request| {
            serve_embedded_asset(request)
        })
        .with_ipc_handler(move |req: Request<String>| {
            if let Ok(env) = serde_json::from_str::<IpcEnvelope>(req.body()) {
                let _ = ipc_proxy.send_event(UserEvent::IpcRequest(env));
            }
        })
        .with_url("ccs://localhost/index.html");

    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let webview = webview_builder.build(&window)?;

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("GTK vbox unavailable");
        webview_builder.build_gtk(vbox)?
    };

    let mut webview_slot = Some(webview);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                let _ = webview_slot.take();
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(user_ev) => match user_ev {
                UserEvent::IpcRequest(env) => {
                    if env.cmd == "window_action" {
                        let act = env
                            .payload
                            .get("action")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        match act {
                            "drag" => {
                                let _ = window.drag_window();
                            }
                            "toggle_maximize" => {
                                window.set_maximized(!window.is_maximized());
                            }
                            "minimize" => {
                                window.set_minimized(true);
                            }
                            "set_theme" => {
                                let theme_str = str_field(&env.payload, "theme");
                                window.set_theme(tao_theme_from_setting(&theme_str));
                            }
                            "close" => {
                                let _ = webview_slot.take();
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                            _ => {}
                        }
                        if let Some(wv) = webview_slot.as_ref() {
                            resolve_ipc(wv, env.id, Ok(serde_json::json!({"ok": true})));
                        }
                        return;
                    }

                    if env.cmd == "probe_endpoint" {
                        let file_name = str_field(&env.payload, "fileName");
                        let base_url = str_field(&env.payload, "baseUrl");
                        let api_key = str_field(&env.payload, "apiKey");
                        let auth_token = str_field(&env.payload, "authToken");
                        let https_proxy = str_field(&env.payload, "httpsProxy");
                        let thread_proxy = proxy.clone();
                        let req_id = env.id;

                        std::thread::spawn(move || {
                            let result =
                                probe_gateway(&base_url, &api_key, &auth_token, &https_proxy);
                            let _ = thread_proxy.send_event(UserEvent::ProbeFinished {
                                id: req_id,
                                file_name,
                                result,
                            });
                        });
                        return;
                    }

                    let is_update_settings = env.cmd == "update_settings";
                    let outcome = handle_sync_ipc(&manager, &env.cmd, &env.payload);
                    if is_update_settings && outcome.is_ok() {
                        if let Ok(mgr) = manager.lock() {
                            window.set_theme(tao_theme_from_setting(&mgr.app_settings.theme));
                        }
                    }
                    if let Some(wv) = webview_slot.as_ref() {
                        resolve_ipc(wv, env.id, outcome);
                    }
                }
                UserEvent::ProbeFinished {
                    id,
                    file_name,
                    result,
                } => {
                    if let Ok(mut mgr) = manager.lock() {
                        mgr.record_probe_result(
                            &file_name,
                            result.latency_ms,
                            result.reachable,
                            &result.detail,
                        );
                    }
                    if let Some(wv) = webview_slot.as_ref() {
                        let val = serde_json::to_value(&result).unwrap_or(Value::Null);
                        resolve_ipc(wv, id, Ok(val));
                    }
                }
            },
            _ => {}
        }
    });
}

fn handle_sync_ipc(
    manager: &Arc<Mutex<WorkspaceManager>>,
    cmd: &str,
    payload: &Value,
) -> Result<Value, String> {
    if cmd == "pick_directory" {
        let initial_dir = manager
            .lock()
            .ok()
            .map(|m| m.app_settings.config_directory.clone());
        let mut dlg = rfd::FileDialog::new().set_title("选择配置文件存储目录");
        if let Some(dir) = initial_dir {
            if dir.exists() {
                dlg = dlg.set_directory(dir);
            }
        }
        let chosen = dlg
            .pick_folder()
            .map(|p| p.to_string_lossy().to_string());
        return Ok(serde_json::json!({ "path": chosen }));
    }

    let mut mgr = manager
        .lock()
        .map_err(|_| "内部状态锁繁忙".to_string())?;

    match cmd {
        "get_state" => serde_json::to_value(mgr.build_workspace_state()).map_err(|e| e.to_string()),
        "create_profile" => {
            let st = mgr.create_profile(
                &str_field(payload, "name"),
                &str_field(payload, "content"),
                &str_field(payload, "provider"),
                &str_field(payload, "description"),
                &str_field(payload, "syncMode"),
                bool_field(payload, "pinned", false),
                bool_field(payload, "activateNow", true),
            )?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "save_profile" => {
            let st = mgr.save_profile(
                &str_field(payload, "fileName"),
                &str_field(payload, "content"),
                opt_str_field(payload, "provider"),
                opt_str_field(payload, "description"),
                opt_str_field(payload, "syncMode"),
                payload.get("pinned").and_then(|v| v.as_bool()),
            )?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "activate_profile" => {
            let mode_override = opt_str_field(payload, "syncModeOverride");
            let st = mgr.activate_profile(
                &str_field(payload, "fileName"),
                mode_override.as_deref(),
            )?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "toggle_pin" => {
            let st = mgr.toggle_pin(&str_field(payload, "fileName"))?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "duplicate_profile" => {
            let st = mgr.duplicate_profile(&str_field(payload, "fileName"))?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "rename_profile" => {
            let st = mgr.rename_profile(
                &str_field(payload, "oldFileName"),
                &str_field(payload, "newName"),
            )?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "delete_profile" => {
            let st = mgr.delete_profile(&str_field(payload, "fileName"))?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "import_live_config" => {
            let name = opt_str_field(payload, "name");
            let st = mgr.import_live_config(&str_field(payload, "mode"), name.as_deref())?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "list_backups" => serde_json::to_value(mgr.list_backups()).map_err(|e| e.to_string()),
        "restore_backup" => {
            let st = mgr.restore_backup(&str_field(payload, "id"))?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "delete_backup" => {
            let list = mgr.delete_backup(&str_field(payload, "id"));
            serde_json::to_value(list).map_err(|e| e.to_string())
        }
        "update_settings" => {
            let st = mgr.update_settings(
                opt_str_field(payload, "theme"),
                opt_str_field(payload, "configDirectory"),
                opt_str_field(payload, "defaultSyncMode"),
                payload.get("autoBackup").and_then(|v| v.as_bool()),
            )?;
            serde_json::to_value(st).map_err(|e| e.to_string())
        }
        "reveal_path" => {
            let target = str_field(payload, "target");
            let path = match target.as_str() {
                "claude_config" => mgr.claude_settings_path.clone(),
                "backups_dir" => mgr.backups_subdir(),
                _ => mgr.settings_subdir(),
            };
            reveal_in_file_manager(&path);
            Ok(serde_json::json!({ "ok": true }))
        }
        other => Err(format!("未知 IPC 指令: {}", other)),
    }
}

fn resolve_ipc(webview: &wry::WebView, id: u64, outcome: Result<Value, String>) {
    let script = match outcome {
        Ok(val) => {
            let serialized = serde_json::to_string(&val).unwrap_or_else(|_| "null".to_string());
            format!("window.__ccs_resolve({}, true, {});", id, serialized)
        }
        Err(err) => {
            let serialized = serde_json::to_string(&err).unwrap_or_else(|_| "\"Error\"".to_string());
            format!("window.__ccs_resolve({}, false, {});", id, serialized)
        }
    };
    let _ = webview.evaluate_script(&script);
}

fn serve_embedded_asset(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path();
    let (bytes, mime) = match path {
        "/" | "/index.html" => (ASSET_INDEX_HTML, "text/html; charset=utf-8"),
        "/styles/tokens.css" => (ASSET_TOKENS_CSS, "text/css; charset=utf-8"),
        "/styles/app.css" => (ASSET_APP_CSS, "text/css; charset=utf-8"),
        "/scripts/icons.js" => (ASSET_ICONS_JS, "text/javascript; charset=utf-8"),
        "/scripts/presets.js" => (ASSET_PRESETS_JS, "text/javascript; charset=utf-8"),
        "/scripts/diff.js" => (ASSET_DIFF_JS, "text/javascript; charset=utf-8"),
        "/scripts/app.js" => (ASSET_APP_JS, "text/javascript; charset=utf-8"),
        "/icon.svg" => (ASSET_ICON_SVG, "image/svg+xml"),
        _ => {
            return Response::builder()
                .status(404)
                .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(Cow::Borrowed(b"Not Found" as &[u8]))
                .unwrap();
        }
    };

    Response::builder()
        .status(200)
        .header(CONTENT_TYPE, mime)
        .body(Cow::Borrowed(bytes))
        .unwrap()
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn opt_str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn bool_field(v: &Value, key: &str, default_val: bool) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(default_val)
}

fn reveal_in_file_manager(path: &Path) {
    #[cfg(target_os = "macos")]
    {
        if path.is_file() {
            let _ = Command::new("open").arg("-R").arg(path).spawn();
        } else {
            let _ = Command::new("open").arg(path).spawn();
        }
    }
    #[cfg(target_os = "windows")]
    {
        if path.is_file() {
            let _ = Command::new("explorer")
                .arg(format!("/select,{}", path.display()))
                .spawn();
        } else {
            let _ = Command::new("explorer").arg(path).spawn();
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let dir = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        let _ = Command::new("xdg-open").arg(dir).spawn();
    }
}

#[cfg(target_os = "macos")]
fn build_macos_native_menu() -> muda::Menu {
    use muda::{AboutMetadata, Menu, PredefinedMenuItem, Submenu};

    let menu = Menu::new();

    let app_menu = Submenu::new("Claude Code Switcher", true);
    let _ = app_menu.append_items(&[
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("Claude Code Switcher".to_string()),
                version: Some(state::APP_VERSION.to_string()),
                comments: Some("Claude Code 环境、网关与模型路由切换台".to_string()),
                ..Default::default()
            }),
        ),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::services(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::hide(None),
        &PredefinedMenuItem::hide_others(None),
        &PredefinedMenuItem::show_all(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::quit(None),
    ]);

    let edit_menu = Submenu::new("编辑", true);
    let _ = edit_menu.append_items(&[
        &PredefinedMenuItem::undo(None),
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::select_all(None),
    ]);

    let window_menu = Submenu::new("窗口", true);
    let _ = window_menu.append_items(&[
        &PredefinedMenuItem::minimize(None),
        &PredefinedMenuItem::maximize(None),
        &PredefinedMenuItem::fullscreen(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::close_window(None),
    ]);

    let _ = menu.append_items(&[&app_menu, &edit_menu, &window_menu]);
    menu.init_for_nsapp();
    menu
}

fn load_window_icon(svg_data: &[u8]) -> Option<Icon> {
    let size = 64u32;
    let svg_str = std::str::from_utf8(svg_data).ok()?;
    let options = usvg::Options::default();
    let usvg_tree = usvg::Tree::from_str(svg_str, &options).ok()?;
    let mut pixmap = Pixmap::new(size, size)?;

    let transform = tiny_skia::Transform::from_scale(
        size as f32 / usvg_tree.size.width(),
        size as f32 / usvg_tree.size.height(),
    );
    let resvg_tree = resvg::Tree::from_usvg(&usvg_tree);
    resvg_tree.render(transform, &mut pixmap.as_mut());

    // tiny-skia Pixmap already stores premultiplied RGBA bytes
    let rgba = pixmap.data().to_vec();
    Icon::from_rgba(rgba, size, size).ok()
}
