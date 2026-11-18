use directories::UserDirs;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
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
pub const ORIGINAL_BACKUP_ID: &str = "original-claude-settings";
const MAX_ROLLING_BACKUPS: usize = 25;

static ATOMIC_SEQ: AtomicU64 = AtomicU64::new(1);

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
    pub sync_state: String, // "synced" | "drifted" | "unmanaged" | "missing" | "corrupted"
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub id: String,
    pub file_name: String,
    pub created_at: String,
    pub timestamp_secs: u64,
    #[serde(default)]
    pub timestamp_ms: u128,
    pub label: String,
    pub byte_size: usize,
    pub content: String,
    #[serde(default)]
    pub is_original: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupEnvelope {
    label: String,
    timestamp_secs: u64,
    #[serde(default)]
    timestamp_ms: u128,
    #[serde(default)]
    is_original: bool,
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

    pub fn original_backup_path(&self) -> PathBuf {
        self.backups_subdir()
            .join(format!("{}.json", ORIGINAL_BACKUP_ID))
    }

    fn resolve_profile_path(&self, file_name: &str) -> Result<PathBuf, String> {
        let clean = validate_safe_file_name(file_name)?;
        Ok(self.settings_subdir().join(clean))
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
        let body = serde_json::to_string_pretty(&self.app_settings)
            .map_err(|e| format!("序列化设置失败: {}", e))?;
        atomic_write_string(&self.app_settings_path, &body)
    }

    pub fn save_meta_store(&self) -> Result<(), String> {
        let body = serde_json::to_string_pretty(&self.meta_store)
            .map_err(|e| format!("序列化元数据失败: {}", e))?;
        atomic_write_string(&self.meta_path(), &body)
    }

    /// Permanently archive the user's untouched original `~/.claude/settings.json` once.
    /// This baseline file is never pruned by rolling backup rotation.
    pub fn ensure_original_baseline_backup(&self) {
        let orig_path = self.original_backup_path();
        if orig_path.exists() || !self.claude_settings_path.exists() {
            return;
        }
        if let Ok(raw) = fs::read_to_string(&self.claude_settings_path) {
            if !raw.trim().is_empty() {
                let _ = self.write_original_baseline_str(&raw);
            }
        }
    }

    fn write_original_baseline_str(&self, raw: &str) -> Result<(), String> {
        let orig_path = self.original_backup_path();
        if orig_path.exists() || raw.trim().is_empty() {
            return Ok(());
        }
        let (now_secs, now_millis) = current_epoch_times();
        let envelope = BackupEnvelope {
            label: "初始原版 ~/.claude/settings.json (永久保护)".to_string(),
            timestamp_secs: now_secs,
            timestamp_ms: now_millis,
            is_original: true,
            content: raw.to_string(),
        };
        let body = serde_json::to_string_pretty(&envelope)
            .map_err(|e| format!("序列化初始备份失败: {}", e))?;
        atomic_write_string(&orig_path, &body)
    }

    pub fn ensure_directories_and_seed(&mut self) {
        let _ = fs::create_dir_all(self.settings_subdir());
        let _ = fs::create_dir_all(self.backups_subdir());

        // Capture untouched original ~/.claude/settings.json before any operation
        self.ensure_original_baseline_backup();

        // Validate default_config_file still exists
        if !self.app_settings.default_config_file.is_empty() {
            let valid = self
                .resolve_profile_path(&self.app_settings.default_config_file)
                .map(|p| p.exists())
                .unwrap_or(false);
            if !valid {
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
                        if atomic_write_string(&seed_path, &extracted).is_ok() {
                            self.meta_store.profiles.insert(
                                seed_name.to_string(),
                                ProfileMeta {
                                    provider: infer_provider_from_json(&extracted),
                                    description:
                                        "首次启动自动从 ~/.claude/settings.json 提取的环境配置"
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
                let _ = atomic_write_string(&starter_path, &pretty);
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
                let Some(fname) = p.file_name().and_then(|s| s.to_str()) else {
                    return false;
                };
                p.is_file()
                    && !fname.starts_with('.')
                    && p.extension().map_or(false, |ext| ext == "json")
                    && fname != ACTIVE_CONFIG_NAME
            })
            .collect()
    }

    pub fn build_workspace_state(&self) -> WorkspaceState {
        self.build_workspace_state_with_target(None)
    }

    fn build_workspace_state_with_target(&self, target_file: Option<String>) -> WorkspaceState {
        let live_exists = self.claude_settings_path.exists();
        let live_content = if live_exists {
            fs::read_to_string(&self.claude_settings_path).unwrap_or_default()
        } else {
            "{}".to_string()
        };
        let live_modified_ago = file_modified_ago(&self.claude_settings_path);
        let live_trimmed = live_content.trim();
        let live_val: Option<Value> = if !live_exists || live_trimmed.is_empty() {
            Some(Value::Object(Map::new()))
        } else {
            serde_json::from_str::<Value>(&live_content)
                .ok()
                .filter(|v| v.is_object())
        };
        let live_corrupted = live_exists && !live_trimmed.is_empty() && live_val.is_none();

        let safe_live_for_preview = if live_corrupted || live_trimmed.is_empty() {
            "{}"
        } else {
            &live_content
        };

        let mut profiles: Vec<ProfileView> = self
            .list_profile_paths()
            .into_iter()
            .filter_map(|path| {
                let file_name = path.file_name()?.to_str()?.to_string();
                let display_name = file_name
                    .strip_suffix(".json")
                    .unwrap_or(&file_name)
                    .to_string();
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
                    compute_effective_config(safe_live_for_preview, &content, &sync_mode)
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
                .then_with(|| {
                    a.display_name
                        .to_lowercase()
                        .cmp(&b.display_name.to_lowercase())
                })
        });

        // Compute Live Sync / Drift / Corrupted status
        let sync_state = if !live_exists {
            "missing".to_string()
        } else if live_corrupted {
            "corrupted".to_string()
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
                active_host: if live_corrupted {
                    "JSON 语法错误".to_string()
                } else {
                    live_summary.base_host
                },
                active_model: live_summary.model_summary,
                preserved_sections,
                byte_size: live_content.len(),
                live_content,
                modified_ago: live_modified_ago,
            },
            profiles,
            backups_count,
            target_file,
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
        if !val.is_object() {
            return Err("配置文件根节点必须是 JSON 对象".to_string());
        }
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

        atomic_write_string(&target_path, &pretty)?;

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
            let mut st = self.activate_profile(&file_name, Some(&mode))?;
            st.target_file = Some(file_name);
            Ok(st)
        } else {
            Ok(self.build_workspace_state_with_target(Some(file_name)))
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
        let target_path = self.resolve_profile_path(file_name)?;
        if !target_path.exists() {
            return Err(format!("配置文件 '{}' 不存在", file_name));
        }

        let val: Value =
            serde_json::from_str(content).map_err(|e| format!("JSON 语法无效: {}", e))?;
        if !val.is_object() {
            return Err("配置文件根节点必须是 JSON 对象".to_string());
        }
        let pretty =
            serde_json::to_string_pretty(&val).map_err(|e| format!("格式化失败: {}", e))?;

        let next_mode = sync_mode
            .clone()
            .or_else(|| {
                self.meta_store
                    .profiles
                    .get(file_name)
                    .map(|m| m.sync_mode.clone())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| self.app_settings.default_sync_mode.clone());

        // If this profile is currently active, validate that syncing to ~/.claude/settings.json
        // succeeds BEFORE mutating live state (or persist profile file + sync atomically).
        atomic_write_string(&target_path, &pretty)?;

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
        entry.sync_mode = next_mode.clone();
        if let Some(pin) = pinned {
            entry.pinned = pin;
        }
        self.save_meta_store()?;

        if self.app_settings.default_config_file == file_name {
            let label = format!(
                "保存活动配置「{}」前自动备份",
                file_name.strip_suffix(".json").unwrap_or(file_name)
            );
            self.write_effective_to_claude(&pretty, &next_mode, &label)
                .map_err(|e| format!("配置已保存至本地库，但同步至 Claude 失败: {}", e))?;
        }

        Ok(self.build_workspace_state_with_target(Some(file_name.to_string())))
    }

    pub fn activate_profile(
        &mut self,
        file_name: &str,
        sync_mode_override: Option<&str>,
    ) -> Result<WorkspaceState, String> {
        let path = self.resolve_profile_path(file_name)?;
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

        if let Some(override_mode) = sync_mode_override {
            if !override_mode.is_empty() {
                let entry = self
                    .meta_store
                    .profiles
                    .entry(file_name.to_string())
                    .or_insert_with(|| ProfileMeta {
                        provider: infer_provider_from_json(&profile_content),
                        sync_mode: override_mode.to_string(),
                        ..Default::default()
                    });
                entry.sync_mode = override_mode.to_string();
                let _ = self.save_meta_store();
            }
        }

        self.app_settings.default_config_file = file_name.to_string();
        self.save_app_settings()?;

        Ok(self.build_workspace_state_with_target(Some(file_name.to_string())))
    }

    fn write_effective_to_claude(
        &self,
        profile_content: &str,
        sync_mode: &str,
        backup_label: &str,
    ) -> Result<(), String> {
        let live_exists = self.claude_settings_path.exists();
        let current_live = if live_exists {
            fs::read_to_string(&self.claude_settings_path)
                .map_err(|e| format!("读取 ~/.claude/settings.json 失败: {}", e))?
        } else {
            String::new()
        };

        // Always guarantee the untouched original ~/.claude/settings.json is permanently archived first
        if live_exists && !current_live.trim().is_empty() {
            let _ = self.write_original_baseline_str(&current_live);
        }

        let live_for_merge = if current_live.trim().is_empty() {
            "{}"
        } else {
            &current_live
        };

        let effective = compute_effective_config(live_for_merge, profile_content, sync_mode)?;

        // Backup existing ~/.claude/settings.json if auto_backup is enabled and content actually changes
        if self.app_settings.auto_backup && live_exists && !current_live.trim().is_empty() {
            let old_val = serde_json::from_str::<Value>(&current_live).ok();
            let new_val = serde_json::from_str::<Value>(&effective).ok();
            let changed = match (&old_val, &new_val) {
                (Some(ov), Some(nv)) => ov != nv,
                _ => current_live.trim() != effective.trim(),
            };
            if changed {
                self.create_backup_snapshot_dedup(&current_live, backup_label)?;
            }
        }

        atomic_write_string(&self.claude_settings_path, &effective)
    }

    pub fn toggle_pin(&mut self, file_name: &str) -> Result<WorkspaceState, String> {
        let _ = self.resolve_profile_path(file_name)?;
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
        Ok(self.build_workspace_state_with_target(Some(file_name.to_string())))
    }

    pub fn duplicate_profile(&mut self, file_name: &str) -> Result<WorkspaceState, String> {
        let src = self.resolve_profile_path(file_name)?;
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
        let old_path = self.resolve_profile_path(old_file_name)?;
        if !old_path.exists() {
            return Err("原配置文件不存在".to_string());
        }

        let stem = sanitize_profile_stem(new_name)?;
        let new_file_name = format!("{}.json", stem);
        if new_file_name == old_file_name {
            return Ok(self.build_workspace_state_with_target(Some(new_file_name)));
        }

        let new_path = self.settings_subdir().join(&new_file_name);
        let is_case_only_rename = old_file_name.eq_ignore_ascii_case(&new_file_name)
            || old_file_name.to_lowercase() == new_file_name.to_lowercase();

        if !is_case_only_rename && new_path.exists() {
            return Err("目标配置名称已存在".to_string());
        }

        if is_case_only_rename {
            // Two-step rename ensures case-insensitive filesystems (macOS APFS / Windows NTFS) update casing
            let seq = ATOMIC_SEQ.fetch_add(1, Ordering::Relaxed);
            let tmp_path = self
                .settings_subdir()
                .join(format!(".rename-tmp-{}-{}", std::process::id(), seq));
            fs::rename(&old_path, &tmp_path).map_err(|e| format!("重命名失败: {}", e))?;
            if let Err(e) = fs::rename(&tmp_path, &new_path) {
                let _ = fs::rename(&tmp_path, &old_path);
                return Err(format!("重命名失败: {}", e));
            }
        } else {
            fs::rename(&old_path, &new_path).map_err(|e| format!("重命名失败: {}", e))?;
        }

        if let Some(meta) = self.meta_store.profiles.remove(old_file_name) {
            self.meta_store.profiles.insert(new_file_name.clone(), meta);
            self.save_meta_store()?;
        }
        if self.app_settings.default_config_file == old_file_name {
            self.app_settings.default_config_file = new_file_name.clone();
            self.save_app_settings()?;
        }

        Ok(self.build_workspace_state_with_target(Some(new_file_name)))
    }

    pub fn delete_profile(&mut self, file_name: &str) -> Result<WorkspaceState, String> {
        let path = self.resolve_profile_path(file_name)?;
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

    pub fn import_live_config(
        &mut self,
        mode: &str,
        custom_name: Option<&str>,
    ) -> Result<WorkspaceState, String> {
        if !self.claude_settings_path.exists() {
            return Err("未检测到 ~/.claude/settings.json 文件".to_string());
        }
        let raw = fs::read_to_string(&self.claude_settings_path)
            .map_err(|e| format!("读取 Claude 配置失败: {}", e))?;
        let parsed: Value = serde_json::from_str(&raw)
            .map_err(|e| format!("当前 ~/.claude/settings.json 语法无效，无法导入: {}", e))?;
        if !parsed.is_object() {
            return Err("当前 ~/.claude/settings.json 根节点不是 JSON 对象".to_string());
        }

        let extracted = extract_provider_profile_from_live(&raw);

        if mode == "update_active" && !self.app_settings.default_config_file.is_empty() {
            let active_fn = self.app_settings.default_config_file.clone();
            let active_mode = self
                .meta_store
                .profiles
                .get(&active_fn)
                .map(|m| m.sync_mode.as_str())
                .unwrap_or("merge");
            let next_content = if active_mode == "replace" {
                serde_json::to_string_pretty(&parsed).unwrap_or(raw)
            } else {
                extracted
            };
            return self.save_profile(&active_fn, &next_content, None, None, None, None);
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
        if file_name.is_empty() || validate_safe_file_name(file_name).is_err() {
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
        if let Some(dir_str) = config_directory {
            let trimmed = dir_str.trim();
            if !trimmed.is_empty() {
                let next_dir = expand_tilde_path(trimmed);
                if next_dir != self.app_settings.config_directory {
                    fs::create_dir_all(next_dir.join(SETTINGS_SUBDIR)).map_err(|e| {
                        format!("无法在所选路径创建配置目录 '{}': {}", next_dir.display(), e)
                    })?;
                    fs::create_dir_all(next_dir.join(BACKUPS_SUBDIR)).map_err(|e| {
                        format!("无法在所选路径创建备份目录 '{}': {}", next_dir.display(), e)
                    })?;
                    self.app_settings.config_directory = next_dir;
                    self.load_meta_from_current_dir();
                    self.ensure_directories_and_seed();
                }
            }
        }
        if let Some(t) = theme {
            self.app_settings.theme = t;
        }
        if let Some(m) = default_sync_mode {
            self.app_settings.default_sync_mode = m;
        }
        if let Some(b) = auto_backup {
            self.app_settings.auto_backup = b;
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

    // --- Automatic Time-Machine Snapshots & Original Baseline Protection ---
    fn create_backup_snapshot_dedup(&self, content: &str, label: &str) -> Result<Option<String>, String> {
        // Avoid writing a redundant duplicate snapshot if the newest snapshot already stores identical JSON/bytes
        let existing = self.list_backups_chronological();
        if let Some(latest) = existing.first() {
            let same = match (
                serde_json::from_str::<Value>(&latest.content),
                serde_json::from_str::<Value>(content),
            ) {
                (Ok(a), Ok(b)) => a == b,
                _ => latest.content.trim() == content.trim(),
            };
            if same {
                return Ok(None);
            }
        }
        self.create_backup_snapshot(content, label).map(Some)
    }

    pub fn create_backup_snapshot(&self, content: &str, label: &str) -> Result<String, String> {
        let dir = self.backups_subdir();
        fs::create_dir_all(&dir).map_err(|e| format!("创建备份目录失败: {}", e))?;
        let (now_secs, now_millis) = current_epoch_times();
        let seq = ATOMIC_SEQ.fetch_add(1, Ordering::Relaxed);
        let id = format!("snap-{:013}-{:04}", now_millis, seq % 10000);
        let file_name = format!("{}.json", id);
        let envelope = BackupEnvelope {
            label: label.to_string(),
            timestamp_secs: now_secs,
            timestamp_ms: now_millis,
            is_original: false,
            content: content.to_string(),
        };
        let body = serde_json::to_string_pretty(&envelope)
            .map_err(|e| format!("序列化快照失败: {}", e))?;
        atomic_write_string(&dir.join(file_name), &body)?;
        self.prune_old_backups();
        Ok(id)
    }

    fn prune_old_backups(&self) {
        let rolling: Vec<BackupEntry> = self
            .list_backups_chronological()
            .into_iter()
            .filter(|b| !b.is_original && b.id != ORIGINAL_BACKUP_ID)
            .collect();
        if rolling.len() <= MAX_ROLLING_BACKUPS {
            return;
        }
        for old in &rolling[MAX_ROLLING_BACKUPS..] {
            let _ = fs::remove_file(self.backups_subdir().join(&old.file_name));
        }
    }

    fn list_backups_chronological(&self) -> Vec<BackupEntry> {
        let dir = self.backups_subdir();
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut list: Vec<BackupEntry> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                let Some(fname) = p.file_name().and_then(|s| s.to_str()) else {
                    return false;
                };
                p.is_file()
                    && !fname.starts_with('.')
                    && p.extension().map_or(false, |x| x == "json")
            })
            .filter_map(|p| {
                let file_name = p.file_name()?.to_str()?.to_string();
                let id = file_name
                    .strip_suffix(".json")
                    .unwrap_or(&file_name)
                    .to_string();
                let raw = fs::read_to_string(&p).ok()?;
                if let Ok(env) = serde_json::from_str::<BackupEnvelope>(&raw) {
                    let is_original = env.is_original || id == ORIGINAL_BACKUP_ID;
                    let ts_ms = if env.timestamp_ms > 0 {
                        env.timestamp_ms
                    } else {
                        (env.timestamp_secs as u128) * 1000
                    };
                    Some(BackupEntry {
                        id,
                        file_name,
                        created_at: relative_time_from_secs(env.timestamp_secs),
                        timestamp_secs: env.timestamp_secs,
                        timestamp_ms: ts_ms,
                        label: env.label,
                        byte_size: env.content.len(),
                        content: env.content,
                        is_original,
                    })
                } else {
                    None
                }
            })
            .collect();

        list.sort_by(|a, b| {
            b.timestamp_ms
                .cmp(&a.timestamp_ms)
                .then_with(|| b.file_name.cmp(&a.file_name))
        });
        list
    }

    pub fn list_backups(&self) -> Vec<BackupEntry> {
        let mut list = self.list_backups_chronological();
        // Pin permanent original baseline backup at the top of the list for instant access
        list.sort_by(|a, b| {
            b.is_original
                .cmp(&a.is_original)
                .then_with(|| b.timestamp_ms.cmp(&a.timestamp_ms))
                .then_with(|| b.file_name.cmp(&a.file_name))
        });
        list
    }

    pub fn restore_backup(&mut self, id: &str) -> Result<WorkspaceState, String> {
        if id.contains('/') || id.contains('\\') || id.contains("..") {
            return Err("无效的快照 ID".to_string());
        }
        let snaps = self.list_backups();
        let target = snaps
            .iter()
            .find(|b| b.id == id)
            .cloned()
            .ok_or_else(|| "未找到指定历史快照".to_string())?;

        if self.claude_settings_path.exists() {
            if let Ok(cur) = fs::read_to_string(&self.claude_settings_path) {
                if !cur.trim().is_empty() {
                    let _ = self.write_original_baseline_str(&cur);
                    let differs = match (
                        serde_json::from_str::<Value>(&cur),
                        serde_json::from_str::<Value>(&target.content),
                    ) {
                        (Ok(a), Ok(b)) => a != b,
                        _ => cur.trim() != target.content.trim(),
                    };
                    if differs {
                        let _ = self.create_backup_snapshot_dedup(&cur, "执行回滚前自动保护快照");
                    }
                }
            }
        }

        atomic_write_string(&self.claude_settings_path, &target.content)?;

        // Reconcile default_config_file with restored ~/.claude/settings.json:
        // 1. If a saved profile's effective output matches the restored file, bind it as active.
        // 2. Otherwise clear default_config_file so saving another profile won't overwrite the restored state.
        let mut matched_file: Option<String> = None;
        if let Ok(restored_val) = serde_json::from_str::<Value>(&target.content) {
            let candidate_paths = self.list_profile_paths();
            // Prefer keeping current default_config_file if it already matches
            for path in &candidate_paths {
                let Some(fname) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                let Ok(prof_raw) = fs::read_to_string(path) else {
                    continue;
                };
                let mode = self
                    .meta_store
                    .profiles
                    .get(fname)
                    .map(|m| m.sync_mode.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("merge");
                if let Ok(eff) = compute_effective_config(&target.content, &prof_raw, mode) {
                    if let Ok(eff_val) = serde_json::from_str::<Value>(&eff) {
                        if eff_val == restored_val {
                            if fname == self.app_settings.default_config_file {
                                matched_file = Some(fname.to_string());
                                break;
                            } else if matched_file.is_none() {
                                matched_file = Some(fname.to_string());
                            }
                        }
                    }
                }
            }
        }

        let next_default = matched_file.clone().unwrap_or_default();
        if self.app_settings.default_config_file != next_default {
            self.app_settings.default_config_file = next_default;
            let _ = self.save_app_settings();
        }

        Ok(self.build_workspace_state_with_target(matched_file))
    }

    pub fn delete_backup(&self, id: &str) -> Vec<BackupEntry> {
        if id == ORIGINAL_BACKUP_ID
            || id.contains('/')
            || id.contains('\\')
            || id.contains("..")
        {
            return self.list_backups();
        }
        let file_path = self.backups_subdir().join(format!("{}.json", id));
        if let Ok(raw) = fs::read_to_string(&file_path) {
            if let Ok(env) = serde_json::from_str::<BackupEnvelope>(&raw) {
                if env.is_original {
                    return self.list_backups();
                }
            }
        }
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
    if !profile_val.is_object() {
        return Err("配置文件根节点必须是 JSON 对象".to_string());
    }

    if sync_mode == "replace" {
        return serde_json::to_string_pretty(&profile_val)
            .map_err(|e| format!("格式化失败: {}", e));
    }

    let trimmed_live = live_json_str.trim();
    let mut base_map: Map<String, Value> = if trimmed_live.is_empty() {
        Map::new()
    } else {
        match serde_json::from_str::<Value>(trimmed_live) {
            Ok(Value::Object(m)) => m,
            Ok(_) => {
                return Err(
                    "当前 ~/.claude/settings.json 根节点不是 JSON 对象，已中止智能合并以防丢失数据"
                        .to_string(),
                )
            }
            Err(e) => {
                return Err(format!(
                    "当前 ~/.claude/settings.json 存在语法错误 ({})，为保护您的 Hooks 与权限已中止智能合并。请先通过快照回滚或使用「完整覆盖」模式。",
                    e
                ))
            }
        }
    };

    let profile_map = match profile_val {
        Value::Object(m) => m,
        _ => unreachable!(),
    };

    // 1. Clean managed provider env keys while preserving user's custom non-provider env keys
    let mut env_map = match base_map.shift_remove("env") {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };

    for key in MANAGED_ENV_KEYS {
        env_map.shift_remove(*key);
    }

    if let Some(Value::Object(prof_env)) = profile_map.get("env") {
        for (k, v) in prof_env {
            // Skip injecting empty-string API keys/tokens in Smart Merge mode so OAuth / shell env works cleanly
            if (k == "ANTHROPIC_API_KEY" || k == "ANTHROPIC_AUTH_TOKEN")
                && v.as_str().map_or(false, |s| s.trim().is_empty())
            {
                continue;
            }
            env_map.insert(k.clone(), v.clone());
        }
    }

    if !env_map.is_empty() {
        base_map.insert("env".to_string(), Value::Object(env_map));
    }

    // 2. Reset profile-managed top-level routing/thinking keys so clearing them or switching providers never leaks stale values
    base_map.shift_remove("model");
    base_map.shift_remove("alwaysThinkingEnabled");

    for (k, v) in &profile_map {
        if k == "env" {
            continue;
        }
        if k == "model" && v.as_str().map_or(false, |s| s.trim().is_empty()) {
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

fn atomic_write_string(path: &Path, content: &str) -> Result<(), String> {
    let real_target = if path.exists() {
        fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };

    let parent = real_target
        .parent()
        .ok_or_else(|| format!("无效的目标文件路径: {}", real_target.display()))?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("创建目录 '{}' 失败: {}", parent.display(), e))?;

    let seq = ATOMIC_SEQ.fetch_add(1, Ordering::Relaxed);
    let file_name = real_target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("config.json");
    let tmp_path = parent.join(format!(
        ".{}.ccs-tmp-{}-{}",
        file_name,
        std::process::id(),
        seq
    ));

    fs::write(&tmp_path, content)
        .map_err(|e| format!("写入临时文件 '{}' 失败: {}", tmp_path.display(), e))?;

    if fs::rename(&tmp_path, &real_target).is_err() {
        let write_res = fs::write(&real_target, content);
        let _ = fs::remove_file(&tmp_path);
        write_res.map_err(|e| format!("写入文件 '{}' 失败: {}", real_target.display(), e))?;
    }
    Ok(())
}

fn validate_safe_file_name(file_name: &str) -> Result<&str, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || !trimmed.ends_with(".json")
    {
        return Err(format!("非法配置文件名: {}", file_name));
    }
    Ok(trimmed)
}

fn current_epoch_times() -> (u64, u128) {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (dur.as_secs(), dur.as_millis())
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
    let env = root.and_then(|m| m.get("env")).and_then(|v| v.as_object());

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
    let has_auth_token_key = env.map_or(false, |e| e.contains_key("ANTHROPIC_AUTH_TOKEN"));
    let has_api_key_key = env.map_or(false, |e| e.contains_key("ANTHROPIC_API_KEY"));

    let (auth_kind, secret) = if use_bedrock {
        ("bedrock".to_string(), String::new())
    } else if use_vertex {
        ("vertex".to_string(), String::new())
    } else if !auth_token.is_empty() {
        ("auth_token".to_string(), auth_token)
    } else if !api_key.is_empty() {
        ("api_key".to_string(), api_key)
    } else if has_auth_token_key {
        ("auth_token".to_string(), String::new())
    } else if has_api_key_key {
        ("api_key".to_string(), String::new())
    } else {
        ("oauth_none".to_string(), String::new())
    };

    let masked_key = if !secret.is_empty() {
        mask_secret(&secret)
    } else if use_bedrock || use_vertex {
        "IAM / Cloud".to_string()
    } else if has_auth_token_key || has_api_key_key {
        "待填写密钥".to_string()
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
    [
        "permissions",
        "hooks",
        "enabledPlugins",
        "statusLine",
        "mcpServers",
    ]
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
    let without_ext = trimmed
        .strip_suffix(".json")
        .or_else(|| trimmed.strip_suffix(".JSON"))
        .unwrap_or(trimmed)
        .trim();
    let cleaned: String = without_ext
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    let final_stem = cleaned.trim().trim_matches('.');
    if final_stem.is_empty() {
        return Err("配置名称不能为空".to_string());
    }
    if final_stem.eq_ignore_ascii_case("settings") {
        return Err("配置名称不能为保留名称 settings".to_string());
    }
    Ok(final_stem.to_string())
}

fn expand_tilde_path(raw: &str) -> PathBuf {
    if raw == "~" {
        if let Some(u) = UserDirs::new() {
            return u.home_dir().to_path_buf();
        }
    }
    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
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
            println!(
                "当前配置: {}",
                if st.settings.default_config_file.is_empty() {
                    "(未指定)"
                } else {
                    &st.settings.default_config_file
                }
            );
            println!("同步状态: {}", st.live_status.sync_state);
            println!("活动网关: {}", st.live_status.active_host);
            println!("活动模型: {}", st.live_status.active_model);
            if !st.live_status.preserved_sections.is_empty() {
                println!(
                    "保护字段: {}",
                    st.live_status.preserved_sections.join(", ")
                );
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
            // Prefer exact match before substring match
            let matched = st
                .profiles
                .iter()
                .find(|p| {
                    p.file_name.to_lowercase() == q_lower
                        || p.display_name.to_lowercase() == q_lower
                })
                .or_else(|| {
                    st.profiles
                        .iter()
                        .find(|p| p.display_name.to_lowercase().contains(&q_lower))
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
    fn smart_merge_preserves_hooks_and_clears_stale_model_and_thinking() {
        let live = r#"{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
    "ANTHROPIC_API_KEY": "sk-ant-old",
    "MY_CUSTOM_FLAG": "keep-me"
  },
  "model": "claude-opus-4-7",
  "alwaysThinkingEnabled": true,
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

        // Stale API key, top-level model, and alwaysThinkingEnabled cleared so DeepSeek env model applies cleanly
        assert_eq!(merged["env"].get("ANTHROPIC_API_KEY"), None);
        assert_eq!(merged.get("model"), None);
        assert_eq!(merged.get("alwaysThinkingEnabled"), None);
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
    fn smart_merge_omits_empty_secret_placeholders_and_aborts_on_corrupt_live_json() {
        let live = r#"{"hooks": {"Stop": []}}"#;
        let starter_profile = r#"{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
    "ANTHROPIC_API_KEY": ""
  },
  "model": "claude-sonnet-4-6"
}"#;
        let merged_str = compute_effective_config(live, starter_profile, "merge").unwrap();
        let merged: Value = serde_json::from_str(&merged_str).unwrap();
        // Empty API key is omitted so Claude Max OAuth / shell env is never broken
        assert_eq!(merged["env"].get("ANTHROPIC_API_KEY"), None);
        assert_eq!(
            merged["env"]["ANTHROPIC_BASE_URL"],
            "https://api.anthropic.com"
        );

        // Corrupt live JSON MUST abort Smart Merge instead of silently wiping user hooks
        let corrupt_live = r#"{"hooks": {"Stop": [}, "broken": }"#;
        assert!(compute_effective_config(corrupt_live, starter_profile, "merge").is_err());
        // Full replace still succeeds when explicitly requested
        assert!(compute_effective_config(corrupt_live, starter_profile, "replace").is_ok());
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
    fn workspace_lifecycle_sync_drift_original_baseline_and_rollback() {
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
        // Untouched original baseline is captured immediately on startup
        assert_eq!(st0.backups_count, 1);
        assert!(mgr.list_backups()[0].is_original);
        // Deleting the permanent original baseline is blocked
        assert_eq!(mgr.delete_backup(ORIGINAL_BACKUP_ID).len(), 1);

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
        // Dedup prevented creating a duplicate copy of the initial state already in original baseline
        assert_eq!(st1.backups_count, 1);

        // Rotate > 25 snapshots and verify permanent original baseline is NEVER pruned
        for idx in 0..28 {
            mgr.create_backup_snapshot(
                &format!(r#"{{"env":{{"ITER":"{}"}}}}"#, idx),
                &format!("iter-{}", idx),
            )
            .unwrap();
        }
        let after_prune = mgr.list_backups();
        assert_eq!(after_prune.len(), MAX_ROLLING_BACKUPS + 1);
        assert!(after_prune.iter().any(|b| b.is_original && b.id == ORIGINAL_BACKUP_ID));

        // Simulate external modification to ~/.claude/settings.json -> Drift detected!
        fs::write(
            &claude_file,
            r#"{"env":{"ANTHROPIC_BASE_URL":"https://external.example.com"}}"#,
        )
        .unwrap();
        let st_drift = mgr.build_workspace_state();
        assert_eq!(st_drift.live_status.sync_state, "drifted");

        // Rollback to ORIGINAL_BACKUP_ID restores initial config AND re-binds matching profile!
        let st_restored = mgr.restore_backup(ORIGINAL_BACKUP_ID).unwrap();
        let restored = fs::read_to_string(&claude_file).unwrap();
        assert!(restored.contains("sk-initial"));
        assert!(restored.contains("hooks"));
        assert_eq!(st_restored.live_status.sync_state, "synced");
        assert_eq!(
            st_restored.settings.default_config_file,
            "Claude 当前配置.json"
        );

        // Case-only rename works cleanly on case-insensitive filesystems (APFS/NTFS)
        let st_renamed = mgr
            .rename_profile("OpenRouter Gateway.json", "openrouter gateway")
            .unwrap();
        assert_eq!(
            st_renamed.target_file.as_deref(),
            Some("openrouter gateway.json")
        );

        // Path traversal is rejected
        assert!(mgr.delete_profile("../app_settings.json").is_err());
    }
}
