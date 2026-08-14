use std::process::{Command, Output};

/// git 命令封装：参数数组传参，无 shell 注入
pub struct Git;

fn run(path: &str, args: &[&str], timeout_secs: u64) -> Result<Output, String> {
    let child = Command::new("git")
        .args(["-C", path])
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn git 失败: {e}"))?;

    // 用 wait_timeout 做超时（std 版不支持，简化：同步等待，超时由调用方控制）
    let output = child
        .wait_with_output()
        .map_err(|e| format!("git 执行失败: {e}"))?;
    let _ = timeout_secs;
    Ok(output)
}

impl Git {
    /// git fetch <remote> [branch]
    pub fn fetch(path: &str, remote: &str, branch: &str) -> Result<(), String> {
        let mut args = vec!["fetch", remote];
        if !branch.is_empty() {
            args.push(branch);
        }
        let out = run(path, &args, 120)?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    /// git rev-parse <ref> → SHA
    pub fn rev_parse(path: &str, reference: &str) -> Result<String, String> {
        let out = run(path, &["rev-parse", reference], 10)?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    /// 本地 HEAD
    pub fn local_head(path: &str) -> Result<String, String> {
        Self::rev_parse(path, "HEAD")
    }

    /// FETCH_HEAD（最近一次 fetch 的远端头）
    pub fn fetch_head(path: &str) -> Result<String, String> {
        Self::rev_parse(path, "FETCH_HEAD")
    }

    /// 按策略拉取
    pub fn pull(path: &str, mode: &str) -> Result<(), String> {
        match mode {
            "off" => Ok(()),
            "merge" => {
                let out = run(path, &["merge", "FETCH_HEAD"], 300)?;
                if out.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
                }
            }
            "reset" => {
                let out = run(path, &["reset", "--hard", "FETCH_HEAD"], 300)?;
                if out.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
                }
            }
            // 默认 ff-only
            _ => {
                let out = run(path, &["merge", "--ff-only", "FETCH_HEAD"], 300)?;
                if out.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
                }
            }
        }
    }

    /// git log --oneline <from>..<to> 摘要（最多 limit 条）
    pub fn log_summary(path: &str, from: &str, to: &str, limit: usize) -> String {
        let out = run(
            path,
            &["log", "--oneline", &format!("{from}..{to}")],
            15,
        );
        match out {
            Ok(o) if o.status.success() => {
                let all = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let lines: Vec<&str> = all.lines().collect();
                if lines.len() > limit {
                    format!("{}（共 {} 条）", lines[..limit].join("; "), lines.len())
                } else {
                    all
                }
            }
            _ => format!("{from}..{to}"),
        }
    }

    /// 仓库是否干净（无未提交改动）
    pub fn is_clean(path: &str) -> Result<bool, String> {
        let out = run(path, &["status", "--porcelain"], 10)?;
        if out.status.success() {
            Ok(out.stdout.is_empty())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// 创建临时 git 仓库
    fn tmp_repo(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dsh-desktop-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.to_str().unwrap();
        run(p, &["init", "-b", "master"], 10).unwrap();
        run(p, &["config", "user.email", "test@test.com"], 10).unwrap();
        run(p, &["config", "user.name", "test"], 10).unwrap();
        dir
    }

    fn commit(repo: &PathBuf, file: &str, msg: &str) {
        let p = repo.to_str().unwrap();
        fs::write(repo.join(file), msg).unwrap();
        run(p, &["add", "."], 10).unwrap();
        run(p, &["commit", "-m", msg], 10).unwrap();
    }

    /// 从 src clone 一个全新仓库（非空目录），返回目标路径
    fn clone_repo(src: &PathBuf, name: &str) -> PathBuf {
        let dst = std::env::temp_dir().join(format!("dsh-desktop-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dst);
        run(dst.parent().unwrap().to_str().unwrap(), &["clone", src.to_str().unwrap(), dst.to_str().unwrap()], 30).unwrap();
        dst
    }

    #[test]
    fn fetch_and_rev_parse() {
        // 本地 clone 一个仓库，验证 fetch/rev-parse/log_summary
        let src = tmp_repo("src");
        commit(&src, "a.txt", "first");
        commit(&src, "b.txt", "second");

        let dst = clone_repo(&src, "dst");
        let dp = dst.to_str().unwrap();

        assert!(Git::fetch(dp, "origin", "master").is_ok());
        let local = Git::local_head(dp).unwrap();
        let remote = Git::fetch_head(dp).unwrap();
        assert_eq!(local, remote);
        assert_eq!(local.len(), 40);

        // 新提交后 fetch 可检测
        commit(&src, "c.txt", "third");
        Git::fetch(dp, "origin", "master").unwrap();
        let remote2 = Git::fetch_head(dp).unwrap();
        assert_ne!(local, remote2);

        // log summary
        let summary = Git::log_summary(dp, &local, &remote2, 20);
        assert!(summary.contains("third"));
    }

    #[test]
    fn pull_ff_only() {
        let src = tmp_repo("pull-src");
        commit(&src, "a.txt", "one");
        let dst = clone_repo(&src, "pull-dst");
        let dp = dst.to_str().unwrap();
        Git::fetch(dp, "origin", "master").unwrap();
        assert!(Git::pull(dp, "ff-only").is_ok());

        commit(&src, "b.txt", "two");
        Git::fetch(dp, "origin", "master").unwrap();
        let before = Git::local_head(dp).unwrap();
        assert!(Git::pull(dp, "ff-only").is_ok());
        let after = Git::local_head(dp).unwrap();
        assert_ne!(before, after);
    }

    #[test]
    fn dirty_worktree_detected() {
        let src = tmp_repo("dirty-src");
        commit(&src, "a.txt", "one");
        let dst = clone_repo(&src, "dirty-dst");
        let dp = dst.to_str().unwrap();
        // 弄脏工作区
        fs::write(dst.join("dirty.txt"), "local change").unwrap();
        assert!(!Git::is_clean(dp).unwrap());
    }

    #[test]
    fn reset_mode() {
        let src = tmp_repo("reset-src");
        commit(&src, "a.txt", "one");
        let dst = clone_repo(&src, "reset-dst");
        let dp = dst.to_str().unwrap();
        // 弄脏（修改已跟踪文件）
        fs::write(dst.join("a.txt"), "local change").unwrap();
        // 远端也修改同一文件 → ff-only 必然失败
        fs::write(src.join("a.txt"), "remote change").unwrap();
        run(src.to_str().unwrap(), &["add", "."], 10).unwrap();
        run(src.to_str().unwrap(), &["commit", "-m", "two"], 10).unwrap();
        Git::fetch(dp, "origin", "master").unwrap();
        // ff-only 应该失败（脏工作区）
        assert!(Git::pull(dp, "ff-only").is_err());
        // reset 应该成功
        assert!(Git::pull(dp, "reset").is_ok());
    }
}
