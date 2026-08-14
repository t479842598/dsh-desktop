use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::config;
use crate::git::Git;
use crate::notify;
use crate::rebuild;
use crate::state::WatcherState;

/// 一轮轮询的单个仓库结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct PollResult {
    pub repo: String,
    pub updated: bool,
    pub message: String,
}

/// Watcher 后台状态
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct WatcherStatus {
    pub running: bool,
    pub last_poll_at: Option<String>,
    pub last_results: Vec<PollResult>,
}

/// 共享 Watcher 状态
#[derive(Default)]
pub struct WatcherShared {
    pub status: Mutex<WatcherStatus>,
}

/// 启动轮询循环（后台 task）
pub fn start_watcher(app: AppHandle, shared: Arc<WatcherShared>) {
    let app2 = app.clone();
    let shared2 = shared.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let results = poll_once(Some(&app2)).await;
            {
                let mut st = shared2.status.lock().await;
                st.last_poll_at = Some(
                    chrono::Local::now()
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string(),
                );
                st.last_results = results.clone();
            }
            let _ = app2.emit("watcher-poll", &results);
            // 下次轮询
            let (cfg, _) = config::load_config();
            let interval = Duration::from_secs(cfg.poll_interval_sec.max(30));
            sleep(interval).await;
        }
    });
}

/// 执行一轮检查（手动触发或定时）
pub async fn poll_once(_app: Option<&AppHandle>) -> Vec<PollResult> {
    let (cfg, warn) = config::load_config();
    if let Some(w) = warn {
        log::warn!("[watcher] {w}");
    }
    let notify_enabled = cfg.notify.enabled;
    let mut state = WatcherState::load();
    let mut results = Vec::new();

    for repo in &cfg.repos {
        let key = repo.local_path.clone();
        let mut result = PollResult {
            repo: repo.name.clone(),
            updated: false,
            message: String::new(),
        };

        // 1) fetch
        if let Err(e) = Git::fetch(&repo.local_path, &repo.remote, &repo.branch) {
            result.message = format!("git fetch 失败: {e}");
            log::warn!("[{key}] {}", result.message);
            results.push(result);
            continue;
        }
        // 2) 对比 SHA
        let (local, remote) = match (
            Git::local_head(&repo.local_path),
            Git::fetch_head(&repo.local_path),
        ) {
            (Ok(l), Ok(r)) => (l, r),
            (Err(e), _) | (_, Err(e)) => {
                result.message = format!("读取 SHA 失败: {e}");
                results.push(result);
                continue;
            }
        };

        if local == remote {
            result.message = "无更新".into();
            results.push(result);
            continue;
        }

        // 3) 有新提交
        let prev = state.repo_ro(&key).last_seen_sha.clone();
        if prev == remote {
            // 已经处理过这个 SHA（上次拉取后），跳过
            result.message = "无新提交（已处理）".into();
            results.push(result);
            continue;
        }

        let summary = Git::log_summary(&repo.local_path, &local, &remote, 20);
        let count = summary.lines().count();
        result.updated = true;
        result.message = format!("发现 {count} 个新提交: {summary}");
        log::info!("[{key}] {}", result.message);
        if notify_enabled {
            notify::notify(
                &format!("{} 有新更新", repo.name),
                &format!("发现 {count} 个新提交，正在拉取…"),
            );
        }

        // 4) 拉取
        let pull_mode = repo.auto_pull.as_str();
        let pull_result = if pull_mode == "off" {
            Ok(())
        } else {
            // ff-only/merge 前检查工作区；reset 直接硬重置
            if pull_mode != "reset" {
                match Git::is_clean(&repo.local_path) {
                    Ok(true) => {}
                    Ok(false) => {
                        let msg = "工作区有未提交改动，跳过自动拉取（保留现场）".to_string();
                        log::warn!("[{key}] {msg}");
                        if notify_enabled {
                            notify::notify(&format!("{} 拉取失败", repo.name), &msg);
                        }
                        state.repo(&key).last_seen_sha = remote.clone();
                        state.save();
                        result.message = msg;
                        results.push(result);
                        continue;
                    }
                    Err(e) => {
                        result.message = format!("检查工作区失败: {e}");
                        results.push(result);
                        continue;
                    }
                }
            }
            Git::pull(&repo.local_path, pull_mode)
        };

        match pull_result {
            Ok(()) => {
                let now = chrono::Local::now().to_rfc3339();
                let st = state.repo(&key);
                st.last_seen_sha = remote.clone();
                st.last_pulled_at = Some(now.clone());
                st.last_result = Some("ok".into());
                state.save();
                result.message = format!(
                    "已拉取至 {}",
                    remote.chars().take(8).collect::<String>()
                );
                if notify_enabled {
                    notify::notify(
                        &format!("{} 已更新", repo.name),
                        &format!("已拉取最新代码（{}）", &remote[..8.min(remote.len())]),
                    );
                }
                // 5) 重建
                let rb = rebuild::run_rebuild(repo, &summary, &key);
                let st = state.repo(&key);
                st.last_result = Some(if rb.success { "ok".into() } else { "error".into() });
                state.save();
                if notify_enabled {
                    notify::notify(
                        &format!("{} 重建{}", repo.name, if rb.success { "完成" } else { "失败" }),
                        &rb.summary,
                    );
                }
                result.message = format!("{} | 重建: {}", result.message, rb.summary);
            }
            Err(e) => {
                let msg = format!("自动拉取失败: {e}");
                log::error!("[{key}] {msg}");
                if notify_enabled {
                    notify::notify(&format!("{} 拉取失败", repo.name), &msg);
                }
                let st = state.repo(&key);
                st.last_seen_sha = remote.clone();
                st.last_result = Some("error".into());
                state.save();
                result.message = msg;
            }
        }
        results.push(result);
    }

    state.save();
    results
}

/// 立即手动触发一轮（托盘「立即检查更新」）
pub async fn poll_now_cmd(app: AppHandle) -> Result<Vec<PollResult>, String> {
    Ok(poll_once(Some(&app)).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, DshConfig, NotifyConfig, RebuildConfig, RemoteConfig, RepoConfig};
    use std::fs;
    use std::path::PathBuf;

    fn tmp_repo(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dsh-watcher-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.to_str().unwrap();
        std::process::Command::new("git").args(["-C", p, "init", "-b", "master"]).output().unwrap();
        std::process::Command::new("git").args(["-C", p, "config", "user.email", "t@t.com"]).output().unwrap();
        std::process::Command::new("git").args(["-C", p, "config", "user.name", "t"]).output().unwrap();
        dir
    }

    fn commit(repo: &PathBuf, file: &str, msg: &str) {
        fs::write(repo.join(file), msg).unwrap();
        let p = repo.to_str().unwrap();
        std::process::Command::new("git").args(["-C", p, "add", "."]).output().unwrap();
        std::process::Command::new("git").args(["-C", p, "commit", "-m", msg]).output().unwrap();
    }

    fn clone_repo(src: &PathBuf, name: &str) -> PathBuf {
        let dst = std::env::temp_dir().join(format!("dsh-watcher-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dst);
        std::process::Command::new("git")
            .args(["clone", src.to_str().unwrap(), dst.to_str().unwrap()])
            .output()
            .unwrap();
        dst
    }

    /// 构造单仓库配置
    fn cfg_with_repo(local_path: &str, auto_pull: &str) -> AppConfig {
        AppConfig {
            dsh: DshConfig::default(),
            poll_interval_sec: 300,
            notify: NotifyConfig { enabled: false },
            repos: vec![RepoConfig {
                name: "test-repo".into(),
                local_path: local_path.into(),
                remote: "origin".into(),
                branch: "master".into(),
                auto_pull: auto_pull.into(),
                rebuild: RebuildConfig::default(),
            }],
            remote: RemoteConfig::default(),
        }
    }

    /// 把配置写进隔离目录（避免污染用户 ~/.dsh-desktop）
    fn write_cfg(cfg: &AppConfig) {
        let dir = crate::config::config_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.json"), serde_json::to_string_pretty(cfg).unwrap()).unwrap();
    }

    #[tokio::test]
    async fn detects_new_commit_pulls_and_dedupes() {
        // 隔离配置目录（不污染用户 ~/.dsh-desktop）
        let cfg_dir = std::env::temp_dir().join(format!(
            "dsh-watcher-cfg-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&cfg_dir);
        fs::create_dir_all(&cfg_dir).unwrap();
        std::env::set_var("DSH_DESKTOP_CONFIG_DIR", &cfg_dir);

        // 上游仓库 + 本地 clone
        let src = tmp_repo("src");
        commit(&src, "a.txt", "first");
        let dst = clone_repo(&src, "dst");

        write_cfg(&cfg_with_repo(dst.to_str().unwrap(), "ff-only"));

        // 第一轮：无更新
        let r1 = poll_once(None).await;
        assert_eq!(r1.len(), 1);
        assert!(!r1[0].updated);
        assert!(r1[0].message.contains("无更新"));

        // 上游新提交
        commit(&src, "b.txt", "second");
        // 第二轮：应检测到更新并拉取
        let r2 = poll_once(None).await;
        assert_eq!(r2.len(), 1);
        assert!(r2[0].updated, "应检测到更新: {}", r2[0].message);
        assert!(r2[0].message.contains("已拉取"), "应已拉取: {}", r2[0].message);

        // 本地 HEAD 已前进
        let head = Git::local_head(dst.to_str().unwrap()).unwrap();
        assert_ne!(head.len(), 0);

        // 第三轮：同一 SHA 不应重复通知（去重）
        let r3 = poll_once(None).await;
        assert_eq!(r3.len(), 1);
        assert!(!r3[0].updated, "不应重复通知: {}", r3[0].message);
        // 清理隔离配置
        std::env::remove_var("DSH_DESKTOP_CONFIG_DIR");
        let _ = fs::remove_dir_all(&cfg_dir);
    }
}
