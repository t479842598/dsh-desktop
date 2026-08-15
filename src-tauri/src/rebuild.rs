use std::process::Command;

use crate::config::{RebuildConfig, RepoConfig};

/// 重建结果
#[derive(Debug, Clone)]
#[allow(dead_code)] // output_tail/mode 预留：失败详情与模式供后续通知/UI 使用
pub struct RebuildResult {
    pub success: bool,
    pub summary: String,
    pub output_tail: String,
    pub mode: String,
}


/// 重建分发：off / command / dsh-headless
pub fn run_rebuild(repo: &RepoConfig, update_summary: &str, key: &str) -> RebuildResult {
    let cfg = &repo.rebuild;
    match cfg.mode.as_str() {
        "command" => run_command(repo, cfg),
        "dsh-headless" => run_headless(repo, cfg, update_summary, key),
        _ => RebuildResult {
            success: true,
            summary: "未配置重建（off）".into(),
            output_tail: String::new(),
            mode: "off".into(),
        },
    }
}

fn run_command(repo: &RepoConfig, cfg: &RebuildConfig) -> RebuildResult {
    if cfg.command.trim().is_empty() {
        return RebuildResult {
            success: true,
            summary: "rebuild.mode=command 但 command 为空，跳过".into(),
            output_tail: String::new(),
            mode: "command".into(),
        };
    }
    // macOS/linux 用 sh -c；windows 用 cmd /C
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", &cfg.command])
        .current_dir(&repo.local_path)
        .output();
    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .args(["-c", &cfg.command])
        .current_dir(&repo.local_path)
        .output();

    match output {
        Ok(o) => {
            let mut tail = String::from_utf8_lossy(&o.stdout).to_string();
            tail.push_str(&String::from_utf8_lossy(&o.stderr));
            let tail = tail_tail(&tail, 2000);
            RebuildResult {
                success: o.status.success(),
                summary: if o.status.success() {
                    format!("构建成功（{}）", repo.name)
                } else {
                    format!("构建失败（{}）", repo.name)
                },
                output_tail: tail,
                mode: "command".into(),
            }
        }
        Err(e) => RebuildResult {
            success: false,
            summary: format!("执行构建命令失败: {e}"),
            output_tail: String::new(),
            mode: "command".into(),
        },
    }
}

fn run_headless(repo: &RepoConfig, cfg: &RebuildConfig, update_summary: &str, key: &str) -> RebuildResult {
    let prompt = if cfg.prompt.trim().is_empty() {
        format!(
            "仓库 {name} 分支 {branch} 有新提交：{summary}\n请执行：1) 拉取最新代码；2) 运行构建；3) 失败则分析并修复；4) 报告结果。",
            name = repo.name,
            branch = repo.branch,
            summary = update_summary,
        )
    } else {
        cfg.prompt
            .replace("{name}", &repo.name)
            .replace("{branch}", &repo.branch)
            .replace("{summary}", update_summary)
    };

    // 需要 DEEPSEEK_API_KEY；无 key 降级为 command
    if std::env::var("DEEPSEEK_API_KEY").map(|v| v.is_empty()).unwrap_or(true) {
        if cfg.command.trim().is_empty() {
            return RebuildResult {
                success: false,
                summary: "dsh-headless 需要 DEEPSEEK_API_KEY，未设置且无降级 command，跳过重建".into(),
                output_tail: String::new(),
                mode: "skipped".into(),
            };
        }
        log::warn!("[{key}] 无 DEEPSEEK_API_KEY，降级为 command 重建");
        return run_command(repo, cfg);
    }

    // 调用 dsh --profile headless；Windows 下 Rust 不按 PATHEXT 解析 npx.cmd，需经 cmd /C
    let output = run_headless_npx(&repo.local_path, &prompt);

    match output {
        Ok(o) => {
            let mut tail = String::from_utf8_lossy(&o.stdout).to_string();
            tail.push_str(&String::from_utf8_lossy(&o.stderr));
            let tail = tail_tail(&tail, 3000);
            RebuildResult {
                success: o.status.success(),
                summary: if o.status.success() {
                    format!("模型重建成功（{}）", repo.name)
                } else {
                    format!("模型重建失败（{}）", repo.name)
                },
                output_tail: tail,
                mode: "dsh-headless".into(),
            }
        }
        Err(e) => RebuildResult {
            success: false,
            summary: format!("调用 dsh headless 失败: {e}"),
            output_tail: String::new(),
            mode: "dsh-headless".into(),
        },
    }
}

/// 取字符串尾部最多 max 字符
pub fn tail_tail(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().collect();
        let start = chars.len() - max;
        format!("…{}", chars[start..].iter().collect::<String>())
    }
}

/// 调用 `npx @deepseek-ai/dsh --profile headless`：Windows 下 Rust 不按 PATHEXT
/// 解析 .cmd，需经 `cmd /C`；Unix 直接跑 npx。返回 Command::output 的 Result。
fn run_headless_npx(workdir: &str, prompt: &str) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        let cmd = format!(
            "npx @deepseek-ai/dsh --profile headless \"{}\"",
            prompt.replace('\"', "\"\"")
        );
        std::process::Command::new("cmd")
            .args(["/C", &cmd])
            .current_dir(workdir)
            .env("DSH_WORKSPACE", workdir)
            .output()
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("npx")
            .args(["@deepseek-ai/dsh", "--profile", "headless", prompt])
            .current_dir(workdir)
            .env("DSH_WORKSPACE", workdir)
            .output()
    }
}
