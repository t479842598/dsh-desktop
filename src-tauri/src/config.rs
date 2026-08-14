use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 仓库重建配置：off / command / dsh-headless
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RebuildConfig {
    pub mode: String,
    pub command: String,
    pub prompt: String,
    pub retry_on_failure: bool,
}

impl Default for RebuildConfig {
    fn default() -> Self {
        Self {
            mode: "off".into(),
            command: "".into(),
            prompt: "".into(),
            retry_on_failure: true,
        }
    }
}

/// 单个仓库监视配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RepoConfig {
    pub name: String,
    pub local_path: String,
    pub remote: String,
    pub branch: String,
    pub auto_pull: String, // off | ff-only | merge | reset
    pub rebuild: RebuildConfig,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            name: "".into(),
            local_path: "".into(),
            remote: "origin".into(),
            branch: "master".into(),
            auto_pull: "ff-only".into(),
            rebuild: RebuildConfig::default(),
        }
    }
}

/// dsh 服务启动配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DshConfig {
    pub command: Vec<String>,
    pub cwd: String,
    pub port: u16,
    pub ready_timeout_sec: u64,
    pub headless_command: Vec<String>,
}

impl Default for DshConfig {
    fn default() -> Self {
        Self {
            // 默认用本地源码运行 dsh（npm 包 @deepseek-ai/dsh 依赖不完整，无法独立启动）
            command: vec!["pnpm".into(), "dsh".into(), "web".into()],
            cwd: "/Volumes/1T 原装/项目研发/deepseek-harness".into(),
            port: 3080,
            ready_timeout_sec: 60,
            headless_command: vec!["pnpm".into(), "dsh".into()],
        }
    }
}

/// 远程访问配置（spec 04）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteConfig {
    pub enabled: bool,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub allowed_hosts: Vec<String>,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 3081,
            username: "".into(),
            password: "".into(),
            allowed_hosts: vec![],
        }
    }
}

/// 通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotifyConfig {
    pub enabled: bool,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// 顶层配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub dsh: DshConfig,
    pub poll_interval_sec: u64,
    pub notify: NotifyConfig,
    pub repos: Vec<RepoConfig>,
    pub remote: RemoteConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dsh: DshConfig::default(),
            poll_interval_sec: 300,
            notify: NotifyConfig::default(),
            repos: vec![RepoConfig {
                name: "deepseek-harness".into(),
                local_path: "/Volumes/1T 原装/项目研发/deepseek-harness".into(),
                remote: "origin".into(),
                branch: "master".into(),
                auto_pull: "ff-only".into(),
                rebuild: RebuildConfig {
                    mode: "off".into(),
                    command: "pnpm install && pnpm run build".into(),
                    ..RebuildConfig::default()
                },
            }],
            remote: RemoteConfig::default(),
        }
    }
}

/// 配置目录：默认 ~/.dsh-desktop；可用 DSH_DESKTOP_CONFIG_DIR 环境变量覆盖（测试隔离用）
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DSH_DESKTOP_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".dsh-desktop")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn state_path() -> PathBuf {
    config_dir().join("state.json")
}

/// 读取配置：文件缺失写默认值；解析失败保留上次有效值并返回错误信息
pub fn load_config() -> (AppConfig, Option<String>) {
    let path = config_path();
    let default = AppConfig::default();
    if !path.exists() {
        if let Err(e) = fs::create_dir_all(config_dir()) {
            return (default, Some(format!("创建配置目录失败: {e}")));
        }
        let text = serde_json::to_string_pretty(&default).unwrap_or_default();
        let _ = fs::write(&path, text);
        return (default, None);
    }
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<AppConfig>(&text) {
            Ok(cfg) => (cfg, None),
            Err(e) => (default, Some(format!("配置文件解析失败({}): {e}", path.display()))),
        },
        Err(e) => (default, Some(format!("读取配置失败({}): {e}", path.display()))),
    }
}

/// 保存配置（原子写）
#[allow(dead_code)]
pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("config.json.tmp");
    let text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, config_path()).map_err(|e| e.to_string())
}
