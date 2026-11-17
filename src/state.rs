use directories::UserDirs;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CLAUDE_DIR_NAME: &str = ".claude";
pub const ACTIVE_CONFIG_NAME: &str = "settings.json";
pub const APP_DIR_NAME: &str = ".claude-code-switcher";
pub const APP_SETTINGS_NAME: &str = "app_settings.json";
pub const PROFILES_META_NAME: &str = "profiles_meta.json";
pub const SETTINGS_SUBDIR: &str = "settings";
pub const BACKUPS_SUBDIR: &str = "backups";
const MAX_BACKUPS: usize = 25;

pub const MANAGED_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_SMALL_FAST_MODEL",
    "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "AWS_REGION",
    "CLOUD_ML_REGION",
    "ANTHROPIC_VERTEX_PROJECT_ID",
    "HTTPS_PROXY",
    "HTTP_PROXY",
];

fn default_sync_mode() -> String {
    "merge".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub config_directory: PathBuf,
    pub theme: String,
    pub default_config_file: String,
    #[serde(default = "default_sync_mode")]
    pub default_sync_mode: String,
    #[serde(default = "default_true")]
    pub auto_backup: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        let default_dir = UserDirs::new()
            .map(|u| u.home_dir().join(APP_DIR_NAME))
            .unwrap_or_else(|| PathBuf::from(APP_DIR_NAME));
        Self {
            config_directory: default_dir,
            theme: "Dark".to_string(),
            default_config_file: String::new(),
            default_sync_mode: default_sync_mode(),
            auto_backup: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMeta {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_sync_mode")]
    pub sync_mode: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub last_latency_ms: Option<u64>,
    #[serde(default)]
    pub last_probe_ok: Option<bool>,
    #[serde(default)]
    pub last_probe_detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilesMetaStore {
    #[serde(default)]
    pub profiles: BTreeMap<String, ProfileMeta>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientSettingsView {
    pub config_directory: String,
    pub claude_config_path: String,
    pub theme: String,
    pub default_config_file: String,
    pub default_sync_mode: String,
    pub auto_backup: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStatusView {
    pub exists: bool,
    pub sync_state: String, // "synced" | "drifted" | "unmanaged" | "missing"
    pub active_host: String,
    pub active_model: String,
    pub preserved_sections: Vec<String>,
    pub live_content: String,
    pub byte_size: usize,
    pub modified_ago: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    pub file_name: String,
    pub display_name: String,
    pub provider: String,
    pub description: String,
    pub sync_mode: String,
    pub pinned: bool,
    pub is_active: bool,
    pub base_host: String,
    pub base_url: String,
    pub auth_kind: String,
    pub masked_key: String,
    pub model_summary: String,
    pub last_latency_ms: Option<u64>,
    pub last_probe_ok: Option<bool>,
    pub last_probe_detail: String,
    pub updated_ago: String,
    pub content: String,
    pub effective_preview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub app_version: String,
    pub platform: String,
    pub settings: ClientSettingsView,
    pub live_status: LiveStatusView,
    pub profiles: Vec<ProfileView>,
    pub backups_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub id: String,
    pub file_name: String,
    pub created_at: String,
    pub timestamp_secs: u64,
    pub label: String,
    pub byte_size: usize,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupEnvelope {
    label: String,
    timestamp_secs: u64,
    content: String,
}

pub struct WorkspaceManager {
    pub app_settings_path: PathBuf,
    pub claude_settings_path: PathBuf,
    pub app_settings: AppSettings,
    pub meta_store: ProfilesMetaStore,
}

impl WorkspaceManager {
    pub fn new_default() -> Self {
        let home = UserDirs::new()
            .map(|u| u.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let app_root = home.join(APP_DIR_NAME);
        let claude_root = home.join(CLAUDE_DIR_NAME);
        Self::with_paths(
            app_root.join(APP_SETTINGS_NAME),
            claude_root.join(ACTIVE_CONFIG_NAME),
            app_root,
        )
    }

    pub fn with_paths(
        app_settings_path: PathBuf,
        claude_settings_path: PathBuf,
        default_config_dir: PathBuf,
    ) -> Self {
        let mut mgr = Self {
            app_settings_path,
            claude_settings_path,
            app_settings: AppSettings {
                config_directory: default_config_dir,
                ..AppSettings::default()
            },
            meta_store: ProfilesMetaStore::default(),
        };
        mgr.load_settings_and_meta();
        mgr.ensure_directories_and_seed();
        mgr
    }

    pub fn settings_subdir(&self) -> PathBuf {
        self.app_settings.config_directory.join(SETTINGS_SUBDIR)
    }

    pub fn backups_subdir(&self) -> PathBuf {
        self.app_settings.config_directory.join(BACKUPS_SUBDIR)
    }

    pub fn meta_path(&self) -> PathBuf {
        self.app_settings.config_directory.join(PROFILES_META_NAME)
    }

    fn load_settings_and_meta(&mut self) {
        if self.app_settings_path.exists() {
            if let Ok(raw) = fs::read_to_string(&self.app_settings_path) {
                if let Ok(parsed) = serde_json::from_str::<AppSettings>(&raw) {
                    self.app_settings = parsed;
                }
            }
        } else {
            let _ = self.save_app_settings();
        }

        let meta_file = self.meta_path();
        if meta_file.exists() {
            if let Ok(raw) = fs::read_to_string(&meta_file) {
                if let Ok(parsed) = serde_json::from_str::<ProfilesMetaStore>(&raw) {
                    self.meta_store = parsed;
                }
            }
        }
    }

    pub fn save_app_settings(&self) -> Result<(), String> {
        if let Some(parent) = self.app_settings_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建应用配置目录失败: {}", e))?;
        }
        let body = serde_json::to_string_pretty(&self.app_settings)
            .map_err(|e| format!("序列化设置失败: {}", e))?;
        fs::write(&self.app_settings_path, body).map_err(|e| format!("写入设置失败: {}", e))
    }

    pub fn save_meta_store(&self) -> Result<(), String> {
        fs::create_dir_all(&self.app_settings.config_directory)
            .map_err(|e| format!("创建目录失败: {}", e))?;
        let body = serde_json::to_string_pretty(&self.meta_store)
            .map_err(|e| format!("序列化元数据失败: {}", e))?;
        fs::write(self.meta_path(), body).map_err(|e| format!("写入元数据失败: {}", e))
    }

    pub fn ensure_directories_and_seed(&mut self) {
        let _ = fs::create_dir_all(self.settings_subdir());
        let _ = fs::create_dir_all(self.backups_subdir());

        // Validate default_config_file still exists
        if !self.app_settings.default_config_file.is_empty() {
            let active_path = self
                .settings_subdir()
                .join(&self.app_settings.default_config_file);
            if !active_path.exists() {
                self.app_settings.default_config_file.clear();
                let _ = self.save_app_settings();
            }
        }

        // Non-destructively seed initial profile if profile library is completely empty
        let existing = self.list_profile_paths();
        if existing.is_empty() {
            if self.claude_settings_path.exists() {
                if let Ok(live_raw) = fs::read_to_string(&self.claude_settings_path) {
                    if serde_json::from_str::<Value>(&live_raw).is_ok() {
                        let extracted = extract_provider_profile_from_live(&live_raw);
                        let seed_name = "Claude 当前配置.json";
                        let seed_path = self.settings_subdir().join(seed_name);
                        if fs::write(&seed_path, &extracted).is_ok() {
                            self.meta_store.profiles.insert(
                                seed_name.to_string(),
                                ProfileMeta {
                                    provider: infer_provider_from_json(&extracted),
                                    description: "首次启动自动从 ~/.claude/settings.json 提取的环境配置"
                                        .to_string(),
                                    sync_mode: "merge".to_string(),
                                    pinned: true,
                                    ..Default::default()
                                },
                            );
                            self.app_settings.default_config_file = seed_name.to_string();
                            let _ = self.save_meta_store();
                            let _ = self.save_app_settings();
                            return;
                        }
                    }
                }
            }

            let starter_name = "Anthropic Official.json";
            let starter_path = self.settings_subdir().join(starter_name);
            let starter_json = serde_json::json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
                    "ANTHROPIC_API_KEY": ""
                },
                "model": "claude-sonnet-4-6"
            });
            if let Ok(pretty) = serde_json::to_string_pretty(&starter_json) {
                let _ = fs::write(&starter_path, pretty);
                self.meta_store.profiles.insert(
                    starter_name.to_string(),
                    ProfileMeta {
                        provider: "anthropic".to_string(),
                        description: "Claude 官方默认环境模板".to_string(),
                        sync_mode: "merge".to_string(),
                        pinned: true,
                        ..Default::default()
                    },
                );
                let _ = self.save_meta_store();
            }
        }
    }

    pub fn list_profile_paths(&self) -> Vec<PathBuf> {
        let subdir = self.settings_subdir();
        let Ok(entries) = fs::read_dir(subdir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension().map_or(false, |ext| ext == "json")
                    && p.file_name().and_then(|s| s.to_str()).unwrap_or_default()
                        != ACTIVE_CONFIG_NAME
            })
            .collect()
    }

    pub fn build_workspace_state(&self) -> WorkspaceState {
        let live_exists = self.claude_settings_path.exists();
        let live_content = if live_exists {
            fs::read_to_string(&self.claude_settings_path).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        };
        let live_modified_ago = file_modified_ago(&self.claude_settings_path);
        let live_val = serde_json::from_str::<Value>(&live_content).ok();

        let mut profiles: Vec<ProfileView> = self
            .list_profile_paths()
            .into_iter()
            .filter_map(|path| {
                let file_name = path.file_name()?.to_str()?.to_string();
                let display_name = file_name.strip_suffix(".json").unwrap_or(&file_name).to_string();
                let content = fs::read_to_string(&path).ok()?;
                let meta = self
                    .meta_store
                    .profiles
                    .get(&file_name)
                    .cloned()
                    .unwrap_or_else(|| ProfileMeta {
                        provider: infer_provider_from_json(&content),
                        description: String::new(),
                        sync_mode: self.app_settings.default_sync_mode.clone(),
                        pinned: false,
                        ..Default::default()
                    });

                let sync_mode = if meta.sync_mode.is_empty() {
                    "merge".to_string()
                } else {
                    meta.sync_mode.clone()
                };
                let provider = if meta.provider.is_empty() {
                    infer_provider_from_json(&content)
                } else {
                    meta.provider.clone()
                };
                let summary = summarize_profile_json(&content);
                let effective_preview =
                    compute_effective_config(&live_content, &content, &sync_mode)
                        .unwrap_or_else(|_| content.clone());
                let is_active = self.app_settings.default_config_file == file_name;

                Some(ProfileView {
                    file_name,
                    display_name,
                    provider,
                    description: meta.description,
                    sync_mode,
                    pinned: meta.pinned,
                    is_active,
                    base_host: summary.base_host,
                    base_url: summary.base_url,
                    auth_kind: summary.auth_kind,
                    masked_key: summary.masked_key,
                    model_summary: summary.model_summary,
                    last_latency_ms: meta.last_latency_ms,
                    last_probe_ok: meta.last_probe_ok,
                    last_probe_detail: meta.last_probe_detail,
                    updated_ago: file_modified_ago(&path),
                    content,
                    effective_preview,
                })
            })
            .collect();

        profiles.sort_by(|a, b| {
            b.is_active
                .cmp(&a.is_active)
                .then_with(|| b.pinned.cmp(&a.pinned))
                .then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
        });

        // Compute Live Sync / Drift status
        let sync_state = if !live_exists {
            "missing".to_string()
        } else if let Some(active_prof) = profiles.iter().find(|p| p.is_active) {
            let eff_val = serde_json::from_str::<Value>(&active_prof.effective_preview).ok();
            match (&live_val, &eff_val) {
                (Some(lv), Some(ev)) if lv == ev => "synced".to_string(),
                _ => "drifted".to_string(),
            }
        } else {
            "unmanaged".to_string()
        };

        let live_summary = summarize_profile_json(&live_content);
        let preserved_sections = extract_preserved_section_names(live_val.as_ref());
        let backups_count = self.list_backups().len();

        let platform = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };

        WorkspaceState {
            app_version: APP_VERSION.to_string(),
            platform: platform.to_string(),
            settings: ClientSettingsView {
                config_directory: self
                    .app_settings
                    .config_directory
                    .to_string_lossy()
                    .to_string(),
                claude_config_path: self.claude_settings_path.to_string_lossy().to_string(),
                theme: self.app_settings.theme.clone(),
                default_config_file: self.app_settings.default_config_file.clone(),
                default_sync_mode: self.app_settings.default_sync_mode.clone(),
                auto_backup: self.app_settings.auto_backup,
            },
            live_status: LiveStatusView {
                exists: live_exists,
                sync_state,
                active_host: live_summary.base_host,
                active_model: live_summary.model_summary,
                preserved_sections,
                byte_size: live_content.len(),
                live_content,
                modified_ago: live_modified_ago,
            },
            profiles,
            backups_count,
        }
    }

    pub fn create_profile(
        &mut self,
        name: &str,
        content: &str,
        provider: &str,
        description: &str,
        sync_mode: &str,
        pinned: bool,
        activate_now: bool,
    ) -> Result<WorkspaceState, String> {
        let clean_name = sanitize_profile_stem(name)?;
        let val: Value =
            serde_json::from_str(content).map_err(|e| format!("JSON 格式无效: {}", e))?;
        let pretty =
            serde_json::to_string_pretty(&val).map_err(|e| format!("JSON 格式化失败: {}", e))?;

        let mut file_name = format!("{}.json", clean_name);
        let mut target_path = self.settings_subdir().join(&file_name);
        let mut counter = 1;
        while target_path.exists() {
            file_name = format!("{}_{}.json", clean_name, counter);
            target_path = self.settings_subdir().join(&file_name);
            counter += 1;
        }

        fs::create_dir_all(self.settings_subdir())
            .map_err(|e| format!("创建配置目录失败: {}", e))?;
        fs::write(&target_path, &pretty).map_err(|e| format!("写入配置文件失败: {}", e))?;

        let mode = if sync_mode.is_empty() {
            self.app_settings.default_sync_mode.clone()
        } else {
            sync_mode.to_string()
        };

        self.meta_store.profiles.insert(
            file_name.clone(),
            ProfileMeta {
                provider: if provider.is_empty() {
                    infer_provider_from_json(&pretty)
                } else {
                    provider.to_string()
                },
                description: description.to_string(),
                sync_mode: mode.clone(),
                pinned,
                ..Default::default()
            },
        );
        self.save_meta_store()?;

        if activate_now {
            self.activate_profile(&file_name, Some(&mode))
        } else {
            Ok(self.build_workspace_state())
        }
    }

    pub fn save_profile(
        &mut self,
        file_name: &str,
        content: &str,
        provider: Option<String>,
        description: Option<String>,
        sync_mode: Option<String>,
        pinned: Option<bool>,
    ) -> Result<WorkspaceState, String> {
        let target_path = self.settings_subdir().join(file_name);
        if !target_path.exists() {
            return Err(format!("配置文件 '{}' 不存在", file_name));
        }

        let val: Value =
            serde_json::from_str(content).map_err(|e| format!("JSON 语法无效: {}", e))?;
        let pretty =
            serde_json::to_string_pretty(&val).map_err(|e| format!("格式化失败: {}", e))?;
        fs::write(&target_path, &pretty).map_err(|e| format!("保存文件失败: {}", e))?;

        let entry = self
            .meta_store
            .profiles
            .entry(file_name.to_string())
            .or_insert_with(|| ProfileMeta {
                provider: infer_provider_from_json(&pretty),
                sync_mode: self.app_settings.default_sync_mode.clone(),
                ..Default::default()
            });

        if let Some(p) = provider {
            entry.provider = p;
        }
        if let Some(d) = description {
            entry.description = d;
        }
        if let Some(m) = sync_mode {
            entry.sync_mode = m;
        }
        if let Some(pin) = pinned {
            entry.pinned = pin;
        }
        let active_mode = entry.sync_mode.clone();
        self.save_meta_store()?;

        if self.app_settings.default_config_file == file_name {
            self.write_effective_to_claude(&pretty, &active_mode, &format!("更新活动配置 {}", file_name))?;
        }

        Ok(self.build_workspace_state())
    }

    pub fn activate_profile(
        &mut self,
        file_name: &str,
        sync_mode_override: Option<&str>,
    ) -> Result<WorkspaceState, String> {
        let path = self.settings_subdir().join(file_name);
        if !path.exists() {
            return Err(format!("未找到配置文件: {}", file_name));
        }
        let profile_content =
            fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {}", e))?;
        let mode = sync_mode_override
            .map(|s| s.to_string())
            .or_else(|| {
                self.meta_store
                    .profiles
                    .get(file_name)
                    .map(|m| m.sync_mode.clone())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| self.app_settings.default_sync_mode.clone());

        let label = format!(
            "切换至「{}」前自动备份",
            file_name.strip_suffix(".json").unwrap_or(file_name)
        );
        self.write_effective_to_claude(&profile_content, &mode, &label)?;

        self.app_settings.default_config_file = file_name.to_string();
        self.save_app_settings()?;

        Ok(self.build_workspace_state())
    }

    fn write_effective_to_claude(
        &self,
        profile_content: &str,
        sync_mode: &str,
        backup_label: &str,
    ) -> Result<(), String> {
        let current_live = if self.claude_settings_path.exists() {
            fs::read_to_string(&self.claude_settings_path).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        };

        let effective = compute_effective_config(&current_live, profile_content, sync_mode)?;

        // Backup existing ~/.claude/settings.json only if it exists and actually changes
        if self.app_settings.auto_backup && self.claude_settings_path.exists() {
            let old_val = serde_json::from_str::<Value>(&current_live).ok();
            let new_val = serde_json::from_str::<Value>(&effective).ok();
            if old_val != new_val && !current_live.trim().is_empty() {
                let _ = self.create_backup_snapshot(&current_live, backup_label);
            }
        }

        if let Some(parent) = self.claude_settings_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建 ~/.claude 目录失败: {}", e))?;
        }
        fs::write(&self.claude_settings_path, effective)
            .map_err(|e| format!("写入 ~/.claude/settings.json 失败: {}", e))
    }

    pub fn toggle_pin(&mut self, file_name: &str) -> Result<WorkspaceState, String> {
        let entry = self
            .meta_store
            .profiles
            .entry(file_name.to_string())
            .or_insert_with(|| ProfileMeta {
                sync_mode: self.app_settings.default_sync_mode.clone(),
                ..Default::default()
            });
        entry.pinned = !entry.pinned;
        self.save_meta_store()?;
        Ok(self.build_workspace_state())
    }

    pub fn duplicate_profile(&mut self, file_name: &str) -> Result<WorkspaceState, String> {
        let src = self.settings_subdir().join(file_name);
        let content = fs::read_to_string(&src).map_err(|e| format!("读取原配置失败: {}", e))?;
        let stem = file_name.strip_suffix(".json").unwrap_or(file_name);
        let meta = self
            .meta_store
            .profiles
            .get(file_name)
            .cloned()
            .unwrap_or_default();
        self.create_profile(
            &format!("{} 副本", stem),
            &content,
            &meta.provider,
            &meta.description,
            &meta.sync_mode,
            false,
            false,
        )
    }

    pub fn rename_profile(
        &mut self,
        old_file_name: &str,
        new_name: &str,
    ) -> Result<WorkspaceState, String> {
        let stem = sanitize_profile_stem(new_name)?;
        let new_file_name = format!("{}.json", stem);
        if new_file_name == old_file_name {
            return Ok(self.build_workspace_state());
        }

        let old_path = self.settings_subdir().join(old_file_name);
        let new_path = self.settings_subdir().join(&new_file_name);
        if !old_path.exists() {
            return Err("原配置文件不存在".to_string());
        }
        if new_path.exists() {
            return Err("目标配置名称已存在".to_string());
        }

        fs::rename(&old_path, &new_path).map_err(|e| format!("重命名失败: {}", e))?;

        if let Some(meta) = self.meta_store.profiles.remove(old_file_name) {
            self.meta_store.profiles.insert(new_file_name.clone(), meta);
            self.save_meta_store()?;
        }
        if self.app_settings.default_config_file == old_file_name {
            self.app_settings.default_config_file = new_file_name;
            self.save_app_settings()?;
        }

        Ok(self.build_workspace_state())
    }

    pub fn delete_profile(&mut self, file_name: &str) -> Result<WorkspaceState, String> {
        let path = self.settings_subdir().join(file_name);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("删除文件失败: {}", e))?;
        }
        self.meta_store.profiles.remove(file_name);
        let _ = self.save_meta_store();

        if self.app_settings.default_config_file == file_name {
            self.app_settings.default_config_file.clear();
            let _ = self.save_app_settings();
        }
        Ok(self.build_workspace_state())
    }

    pub fn import_live_config(&mut self, mode: &str, custom_name: Option<&str>) -> Result<WorkspaceState, String> {
        if !self.claude_settings_path.exists() {
            return Err("未检测到 ~/.claude/settings.json 文件".to_string());
        }
        let raw = fs::read_to_string(&self.claude_settings_path)
            .map_err(|e| format!("读取 Claude 配置失败: {}", e))?;
        let extracted = extract_provider_profile_from_live(&raw);

        if mode == "update_active" && !self.app_settings.default_config_file.is_empty() {
            let active_fn = self.app_settings.default_config_file.clone();
            return self.save_profile(&active_fn, &extracted, None, None, None, None);
        }

        let name = custom_name.unwrap_or("Claude 实时导入配置");
        self.create_profile(
            name,
            &extracted,
            &infer_provider_from_json(&extracted),
            "从 ~/.claude/settings.json 捕获导入",
            "merge",
            false,
            true,
        )
    }

    pub fn record_probe_result(
        &mut self,
        file_name: &str,
        latency_ms: u64,
        ok: bool,
        detail: &str,
    ) {
        if file_name.is_empty() {
            return;
        }
        let entry = self
            .meta_store
            .profiles
            .entry(file_name.to_string())
            .or_insert_with(|| ProfileMeta {
                sync_mode: self.app_settings.default_sync_mode.clone(),
                ..Default::default()
            });
        entry.last_latency_ms = Some(latency_ms);
        entry.last_probe_ok = Some(ok);
        entry.last_probe_detail = detail.to_string();
        let _ = self.save_meta_store();
    }

    pub fn update_settings(
        &mut self,
        theme: Option<String>,
        config_directory: Option<String>,
        default_sync_mode: Option<String>,
        auto_backup: Option<bool>,
    ) -> Result<WorkspaceState, String> {
        if let Some(t) = theme {
            self.app_settings.theme = t;
        }
        if let Some(m) = default_sync_mode {
            self.app_settings.default_sync_mode = m;
        }
        if let Some(b) = auto_backup {
            self.app_settings.auto_backup = b;
        }
        if let Some(dir_str) = config_directory {
            let trimmed = dir_str.trim();
            if !trimmed.is_empty() {
                let next_dir = expand_tilde_path(trimmed);
                if next_dir != self.app_settings.config_directory {
                    self.app_settings.config_directory = next_dir;
                    self.load_meta_from_current_dir();
                    self.ensure_directories_and_seed();
                }
            }
        }
        self.save_app_settings()?;
        Ok(self.build_workspace_state())
    }

    fn load_meta_from_current_dir(&mut self) {
        let meta_file = self.meta_path();
        if meta_file.exists() {
            if let Ok(raw) = fs::read_to_string(&meta_file) {
                if let Ok(parsed) = serde_json::from_str::<ProfilesMetaStore>(&raw) {
                    self.meta_store = parsed;
                    return;
                }
            }
        }
        self.meta_store = ProfilesMetaStore::default();
    }

    // --- Automatic Time-Machine Snapshots ---
    pub fn create_backup_snapshot(&self, content: &str, label: &str) -> Result<String, String> {
        let dir = self.backups_subdir();
        fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {}", e))?;
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let id = format!("snap-{}", now_millis);
        let file_name = format!("{}.json", id);
        let envelope = BackupEnvelope {
            label: label.to_string(),
            timestamp_secs: now_secs,
            content: content.to_string(),
        };
        let body = serde_json::to_string_pretty(&envelope)
            .map_err(|e| format!("序列化快照失败: {}", e))?;
        fs::write(dir.join(file_name), body).map_err(|e| format!("保存快照失败: {}", e))?;
        self.prune_old_backups();
        Ok(id)
    }

    fn prune_old_backups(&self) {
        let mut all = self.list_backups();
        if all.len() <= MAX_BACKUPS {
            return;
        }
        for old in all.drain(MAX_BACKUPS..) {
            let _ = fs::remove_file(self.backups_subdir().join(old.file_name));
        }
    }

    pub fn list_backups(&self) -> Vec<BackupEntry> {
        let dir = self.backups_subdir();
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut list: Vec<BackupEntry> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().map_or(false, |x| x == "json"))
            .filter_map(|p| {
                let file_name = p.file_name()?.to_str()?.to_string();
                let id = file_name.strip_suffix(".json").unwrap_or(&file_name).to_string();
                let raw = fs::read_to_string(&p).ok()?;
                if let Ok(env) = serde_json::from_str::<BackupEnvelope>(&raw) {
                    Some(BackupEntry {
                        id,
                        file_name,
                        created_at: relative_time_from_secs(env.timestamp_secs),
                        timestamp_secs: env.timestamp_secs,
                        label: env.label,
                        byte_size: env.content.len(),
                        content: env.content,
                    })
                } else {
                    None
                }
            })
            .collect();
        list.sort_by(|a, b| b.file_name.cmp(&a.file_name));
        list
    }

    pub fn restore_backup(&mut self, id: &str) -> Result<WorkspaceState, String> {
        let snaps = self.list_backups();
        let target = snaps
            .iter()
            .find(|b| b.id == id)
            .ok_or_else(|| "未找到指定历史快照".to_string())?;

        if self.claude_settings_path.exists() {
            if let Ok(cur) = fs::read_to_string(&self.claude_settings_path) {
                let _ = self.create_backup_snapshot(&cur, "执行回滚前自动保护快照");
            }
        }
        if let Some(parent) = self.claude_settings_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&self.claude_settings_path, &target.content)
            .map_err(|e| format!("恢复快照失败: {}", e))?;

        Ok(self.build_workspace_state())
    }

    pub fn delete_backup(&self, id: &str) -> Vec<BackupEntry> {
        let file_path = self.backups_subdir().join(format!("{}.json", id));
        let _ = fs::remove_file(file_path);
        self.list_backups()
    }
}

// --- Pure Domain Helpers ---

pub fn compute_effective_config(
    live_json_str: &str,
    profile_json_str: &str,
    sync_mode: &str,
) -> Result<String, String> {
    let profile_val: Value = serde_json::from_str(profile_json_str)
        .map_err(|e| format!("配置文件 JSON 无效: {}", e))?;

    if sync_mode == "replace" {
        return serde_json::to_string_pretty(&profile_val)
            .map_err(|e| format!("格式化失败: {}", e));
    }

    let mut base_map: Map<String, Value> = match serde_json::from_str::<Value>(live_json_str) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    };

    let profile_map = match profile_val {
        Value::Object(m) => m,
        _ => return Err("配置文件根节点必须是 JSON 对象".to_string()),
    };

    let mut env_map = match base_map.remove("env") {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };

    for key in MANAGED_ENV_KEYS {
        env_map.shift_remove(*key);
    }

    if let Some(Value::Object(prof_env)) = profile_map.get("env") {
        for (k, v) in prof_env {
            env_map.insert(k.clone(), v.clone());
        }
    }

    if !env_map.is_empty() {
        base_map.insert("env".to_string(), Value::Object(env_map));
    }

    for (k, v) in &profile_map {
        if k == "env" {
            continue;
        }
        if k == "permissions" {
            if let (Some(Value::Object(existing_perms)), Value::Object(new_perms)) =
                (base_map.get_mut("permissions"), v)
            {
                for (pk, pv) in new_perms {
                    existing_perms.insert(pk.clone(), pv.clone());
                }
                continue;
            }
        }
        base_map.insert(k.clone(), v.clone());
    }

    serde_json::to_string_pretty(&Value::Object(base_map))
        .map_err(|e| format!("序列化合并结果失败: {}", e))
}

fn extract_provider_profile_from_live(live_raw: &str) -> String {
    let Ok(Value::Object(live_map)) = serde_json::from_str::<Value>(live_raw) else {
        return live_raw.to_string();
    };
    let mut out = Map::new();
    if let Some(env_val) = live_map.get("env") {
        out.insert("env".to_string(), env_val.clone());
    }
    if let Some(model_val) = live_map.get("model") {
        out.insert("model".to_string(), model_val.clone());
    }
    if let Some(think_val) = live_map.get("alwaysThinkingEnabled") {
        out.insert("alwaysThinkingEnabled".to_string(), think_val.clone());
    }
    if out.is_empty() {
        out.insert("env".to_string(), Value::Object(Map::new()));
    }
    serde_json::to_string_pretty(&Value::Object(out)).unwrap_or_else(|_| live_raw.to_string())
}

struct ProfileSummary {
    base_host: String,
    base_url: String,
    auth_kind: String,
    masked_key: String,
    model_summary: String,
}

fn summarize_profile_json(raw: &str) -> ProfileSummary {
    let parsed = serde_json::from_str::<Value>(raw).ok();
    let root = parsed.as_ref().and_then(|v| v.as_object());
    let env = root
        .and_then(|m| m.get("env"))
        .and_then(|v| v.as_object());

    let get_env_str = |key: &str| -> String {
        env.and_then(|e| e.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let use_bedrock = get_env_str("CLAUDE_CODE_USE_BEDROCK") == "1";
    let use_vertex = get_env_str("CLAUDE_CODE_USE_VERTEX") == "1";
    let base_url_raw = get_env_str("ANTHROPIC_BASE_URL");
    let auth_token = get_env_str("ANTHROPIC_AUTH_TOKEN");
    let api_key = get_env_str("ANTHROPIC_API_KEY");

    let (auth_kind, secret) = if use_bedrock {
        ("bedrock".to_string(), String::new())
    } else if use_vertex {
        ("vertex".to_string(), String::new())
    } else if !auth_token.is_empty() {
        ("auth_token".to_string(), auth_token)
    } else if !api_key.is_empty() {
        ("api_key".to_string(), api_key)
    } else {
        ("oauth_none".to_string(), String::new())
    };

    let masked_key = if !secret.is_empty() {
        mask_secret(&secret)
    } else if use_bedrock || use_vertex {
        "IAM / Cloud".to_string()
    } else {
        "OAuth / 订阅".to_string()
    };

    let base_url = if base_url_raw.is_empty() {
        "https://api.anthropic.com".to_string()
    } else {
        base_url_raw.clone()
    };

    let base_host = if use_bedrock {
        let reg = get_env_str("AWS_REGION");
        if reg.is_empty() {
            "bedrock.aws".to_string()
        } else {
            format!("bedrock.{}", reg)
        }
    } else if use_vertex {
        "vertex.googleapis.com".to_string()
    } else if base_url_raw.is_empty() {
        "api.anthropic.com".to_string()
    } else {
        Url::parse(&base_url_raw)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or(base_url_raw)
    };

    let top_model = root
        .and_then(|m| m.get("model"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let env_model = get_env_str("ANTHROPIC_MODEL");
    let model_summary = if !top_model.is_empty() {
        top_model.to_string()
    } else if !env_model.is_empty() {
        env_model
    } else {
        "默认模型".to_string()
    };

    ProfileSummary {
        base_host,
        base_url,
        auth_kind,
        masked_key,
        model_summary,
    }
}

fn infer_provider_from_json(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("claude_code_use_bedrock") {
        "bedrock".to_string()
    } else if lower.contains("claude_code_use_vertex") {
        "vertex".to_string()
    } else if lower.contains("openrouter.ai") {
        "openrouter".to_string()
    } else if lower.contains("deepseek.com") {
        "deepseek".to_string()
    } else if lower.contains("bigmodel.cn") {
        "glm".to_string()
    } else if lower.contains("moonshot.cn") {
        "kimi".to_string()
    } else if lower.contains("siliconflow.cn") {
        "siliconflow".to_string()
    } else if lower.contains("api.anthropic.com") || !lower.contains("anthropic_base_url") {
        "anthropic".to_string()
    } else {
        "custom".to_string()
    }
}

fn extract_preserved_section_names(live_val: Option<&Value>) -> Vec<String> {
    let Some(Value::Object(map)) = live_val else {
        return Vec::new();
    };
    ["permissions", "hooks", "enabledPlugins", "statusLine", "mcpServers"]
        .iter()
        .filter(|k| map.contains_key(**k))
        .map(|s| (*s).to_string())
        .collect()
}

fn mask_secret(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 10 {
        "••••••••".to_string()
    } else {
        let prefix: String = chars[..6].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{}••••{}", prefix, suffix)
    }
}

fn sanitize_profile_stem(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    let without_ext = trimmed.strip_suffix(".json").unwrap_or(trimmed).trim();
    let cleaned: String = without_ext
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    if cleaned.trim().is_empty() {
        return Err("配置名称不能为空".to_string());
    }
    Ok(cleaned.trim().to_string())
}

fn expand_tilde_path(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(u) = UserDirs::new() {
            return u.home_dir().join(rest);
        }
    }
    PathBuf::from(raw)
}

fn file_modified_ago(path: &Path) -> String {
    let Ok(meta) = fs::metadata(path) else {
        return "未知".to_string();
    };
    let Ok(modified) = meta.modified() else {
        return "未知".to_string();
    };
    let Ok(elapsed) = modified.elapsed() else {
        return "刚刚".to_string();
    };
    format_elapsed_secs(elapsed.as_secs())
}

fn relative_time_from_secs(ts_secs: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(ts_secs);
    format_elapsed_secs(now.saturating_sub(ts_secs))
}

fn format_elapsed_secs(secs: u64) -> String {
    if secs < 60 {
        "刚刚".to_string()
    } else if secs < 3600 {
        format!("{} 分钟前", secs / 60)
    } else if secs < 86400 {
        format!("{} 小时前", secs / 3600)
    } else {
        format!("{} 天前", secs / 86400)
    }
}

// --- Headless CLI Companion (`--status`, `--list`, `--use <profile>`) ---
pub fn run_cli_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return None;
    }

    match args[0].as_str() {
        "-h" | "--help" => {
            println!("Claude Code Switcher v{}", APP_VERSION);
            println!("用法:");
            println!("  claude-code-switcher               启动桌面图形界面");
            println!("  claude-code-switcher --status      查看当前 Claude Code 环境同步状态");
            println!("  claude-code-switcher --list        列出所有已保存的环境配置");
            println!("  claude-code-switcher --use <名称>  立即激活指定配置 (默认智能合并)");
            Some(0)
        }
        "-v" | "--version" => {
            println!("claude-code-switcher {}", APP_VERSION);
            Some(0)
        }
        "--status" => {
            let mgr = WorkspaceManager::new_default();
            let st = mgr.build_workspace_state();
            println!("Claude Code Switcher v{}", st.app_version);
            println!("当前配置: {}", if st.settings.default_config_file.is_empty() { "(未指定)" } else { &st.settings.default_config_file });
            println!("同步状态: {}", st.live_status.sync_state);
            println!("活动网关: {}", st.live_status.active_host);
            println!("活动模型: {}", st.live_status.active_model);
            if !st.live_status.preserved_sections.is_empty() {
                println!("保护字段: {}", st.live_status.preserved_sections.join(", "));
            }
            Some(0)
        }
        "--list" => {
            let mgr = WorkspaceManager::new_default();
            let st = mgr.build_workspace_state();
            for p in st.profiles {
                let marker = if p.is_active { "●" } else { " " };
                println!(
                    "{} {:<26} {:<24} [{}]",
                    marker, p.display_name, p.base_host, p.sync_mode
                );
            }
            Some(0)
        }
        "--use" => {
            let Some(query) = args.get(1) else {
                eprintln!("错误: 请指定配置名称，例如 --use \"Anthropic Official\"");
                return Some(1);
            };
            let mut mgr = WorkspaceManager::new_default();
            let st = mgr.build_workspace_state();
            let q_lower = query.to_lowercase();
            let matched = st.profiles.iter().find(|p| {
                p.file_name.to_lowercase() == q_lower
                    || p.display_name.to_lowercase() == q_lower
                    || p.display_name.to_lowercase().contains(&q_lower)
            });
            match matched {
                Some(target) => {
                    let fname = target.file_name.clone();
                    let dname = target.display_name.clone();
                    match mgr.activate_profile(&fname, None) {
                        Ok(_) => {
                            println!("✓ 已切换并激活环境「{}」", dname);
                            Some(0)
                        }
                        Err(e) => {
                            eprintln!("✗ 激活失败: {}", e);
                            Some(1)
                        }
                    }
                }
                None => {
                    eprintln!("✗ 未找到匹配「{}」的配置", query);
                    Some(1)
                }
            }
        }
        // Ignore macOS Finder `-psn_...` launch arguments so GUI opens normally
        arg if arg.starts_with("-psn_") => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn smart_merge_preserves_hooks_permissions_and_plugins() {
        let live = r#"{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
    "ANTHROPIC_API_KEY": "sk-ant-old",
    "MY_CUSTOM_FLAG": "keep-me"
  },
  "model": "claude-sonnet-4-5",
  "permissions": {
    "allow": ["Read(**)"],
    "defaultMode": "acceptEdits"
  },
  "hooks": {
    "Stop": [{"matcher": "", "hooks": []}]
  },
  "enabledPlugins": ["rust-analyzer"]
}"#;

        let deepseek_profile = r#"{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
    "ANTHROPIC_AUTH_TOKEN": "sk-ds-new",
    "ANTHROPIC_MODEL": "deepseek-chat"
  }
}"#;

        let merged_str = compute_effective_config(live, deepseek_profile, "merge").unwrap();
        let merged: Value = serde_json::from_str(&merged_str).unwrap();

        // Stale API key removed, new token and base URL applied, custom user env preserved
        assert_eq!(merged["env"].get("ANTHROPIC_API_KEY"), None);
        assert_eq!(merged["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-ds-new");
        assert_eq!(
            merged["env"]["ANTHROPIC_BASE_URL"],
            "https://api.deepseek.com/anthropic"
        );
        assert_eq!(merged["env"]["MY_CUSTOM_FLAG"], "keep-me");

        // Hooks, permissions, and enabledPlugins remain 100% intact
        assert!(merged["hooks"]["Stop"].is_array());
        assert_eq!(merged["permissions"]["allow"][0], "Read(**)");
        assert_eq!(merged["enabledPlugins"][0], "rust-analyzer");
    }

    #[test]
    fn full_replace_mode_replaces_entire_document() {
        let live = r#"{"hooks": {"Stop": []}, "env": {"FOO": "1"}}"#;
        let profile = r#"{"env": {"ANTHROPIC_API_KEY": "sk-clean"}}"#;
        let replaced_str = compute_effective_config(live, profile, "replace").unwrap();
        let replaced: Value = serde_json::from_str(&replaced_str).unwrap();
        assert!(replaced.get("hooks").is_none());
        assert_eq!(replaced["env"]["ANTHROPIC_API_KEY"], "sk-clean");
    }

    #[test]
    fn workspace_lifecycle_sync_drift_and_backup_rollback() {
        let tmp = TempDir::new().unwrap();
        let app_dir = tmp.path().join(".claude-code-switcher");
        let claude_file = tmp.path().join(".claude").join("settings.json");
        fs::create_dir_all(claude_file.parent().unwrap()).unwrap();
        fs::write(
            &claude_file,
            r#"{"env":{"ANTHROPIC_API_KEY":"sk-initial"},"hooks":{"Stop":[]}}"#,
        )
        .unwrap();

        let mut mgr = WorkspaceManager::with_paths(
            app_dir.join("app_settings.json"),
            claude_file.clone(),
            app_dir.clone(),
        );

        let st0 = mgr.build_workspace_state();
        assert_eq!(st0.profiles.len(), 1);
        assert_eq!(st0.live_status.sync_state, "synced");

        // Create and activate a second profile
        let st1 = mgr
            .create_profile(
                "OpenRouter Gateway",
                r#"{"env":{"ANTHROPIC_BASE_URL":"https://openrouter.ai/api","ANTHROPIC_AUTH_TOKEN":"sk-or-123"}}"#,
                "openrouter",
                "Relay",
                "merge",
                true,
                true,
            )
            .unwrap();

        assert_eq!(st1.live_status.sync_state, "synced");
        assert_eq!(st1.live_status.active_host, "openrouter.ai");
        assert_eq!(st1.backups_count, 1);

        // Simulate external modification to ~/.claude/settings.json -> Drift detected!
        fs::write(
            &claude_file,
            r#"{"env":{"ANTHROPIC_BASE_URL":"https://external.example.com"}}"#,
        )
        .unwrap();
        let st_drift = mgr.build_workspace_state();
        assert_eq!(st_drift.live_status.sync_state, "drifted");

        // Rollback to snapshot 0 restores initial config including hooks
        let backups = mgr.list_backups();
        let snap_id = backups[0].id.clone();
        let _ = mgr.restore_backup(&snap_id).unwrap();
        let restored = fs::read_to_string(&claude_file).unwrap();
        assert!(restored.contains("sk-initial"));
        assert!(restored.contains("hooks"));
    }
}
