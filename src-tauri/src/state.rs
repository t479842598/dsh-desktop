use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};

use crate::config;

/// 每仓库持久化状态：去重 + 最近结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoState {
    pub last_seen_sha: String,
    pub last_pulled_at: Option<String>,
    pub last_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatcherState {
    pub repos: HashMap<String, RepoState>,
}

impl WatcherState {
    pub fn load() -> Self {
        let path = config::state_path();
        fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = config::state_path();
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        let _ = fs::create_dir_all(dir);
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let tmp = dir.join("state.json.tmp");
            let _ = fs::write(&tmp, text);
            let _ = fs::rename(tmp, path);
        }
    }

    pub fn repo(&mut self, key: &str) -> &mut RepoState {
        self.repos.entry(key.to_string()).or_default()
    }

    pub fn repo_ro(&self, key: &str) -> RepoState {
        self.repos.get(key).cloned().unwrap_or_default()
    }
}
