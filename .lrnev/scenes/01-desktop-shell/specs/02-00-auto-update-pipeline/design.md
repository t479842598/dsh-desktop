---
spec: '02-00-auto-update-pipeline'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 02-00 Auto Update Pipeline - 设计

## L0 摘要

壳内 tokio 定时任务：轮询 git fetch → 状态对比（持久化 last_seen SHA）→ 拉取 → 重建 → 通知。凭据天然复用本地 git credential helper / gh keyring。

## L1 概览

### 架构思路

- **git 为数据源**：`git fetch` 即增量检查，无 GitHub API 配额问题；凭据由 git 自身解析（credential helper → macOS keychain / gh）
- **状态持久化**：`~/.dsh-desktop/state.json` 记录每仓库已处理的最新 SHA，重启不重复通知
- **重建解耦**：watcher 只负责"检测+拉取+通知"，重建通过 trait 分发到 command 执行器或 dsh-headless 执行器（spec 03）
- **失败安全**：任何一步失败都通知用户、保留现场、下一轮重试

### 主要模块

| 模块 | 职责 |
|---|---|
| `src-tauri/src/watcher.rs` | 定时调度、fetch、SHA 对比、去重 |
| `src-tauri/src/git.rs` | git 命令封装（fetch/rev-parse/pull/reset/log） |
| `src-tauri/src/rebuild.rs` | 重建分发（off/command/dsh-headless） |
| `src-tauri/src/state.rs` | 去重状态持久化 |
| `src-tauri/src/notify.rs` | 通知出口（与 spec 01 共用） |

### 关键决策

| 决策 | 选项 | 倾向 | 是否产 ADR |
|---|---|---|---|
| 数据源 | GitHub API / git fetch | git fetch（零配额、凭据复用、支持私有库） | 是 |
| 拉取默认策略 | ff-only / merge / reset | ff-only（安全默认；reset 需显式开启） | 否 |
| 重建方式 | off / command / dsh-headless | 三档可配，默认 off（保守） | 否 |

## L2 详情

### 模块详细设计

#### D-01 轮询循环（watcher.rs）

```
tokio::spawn(loop {
  for repo in config.repos {
    fetch(repo)                    // git -C path fetch remote
    let remote_head = rev_parse(FETCH_HEAD)
    let local_head = rev_parse(HEAD)
    if remote_head != local_head && remote_head != state[repo].last_seen {
      notify("发现 N 个新提交")
      match auto_pull { ... }      // off: 仅记录; ff-only/merge/reset: 执行
      if pulled { rebuild(repo); notify(result) }
      state[repo].last_seen = remote_head; save()
    }
  }
  sleep(poll_interval_sec)
})
```

- 每仓库独立 try/catch，单仓库失败不影响其他
- 轮询间隔从配置读取，支持 `poll_now` command 手动触发

#### D-02 git 封装（git.rs）

- `fetch(path, remote)`：`git -C <path> fetch <remote> --prune`，捕获 stderr，超时（默认 120s）
- `rev_parse(path, "HEAD")` / `rev_parse(path, "FETCH_HEAD")`
- `pull(path, mode)`：ff-only → `git merge --ff-only FETCH_HEAD`；merge → `git merge FETCH_HEAD`；reset → `git reset --hard FETCH_HEAD`
- `log_summary(path, from, to)`：`git log --oneline from..to | head -20`（通知摘要用）
- 全部用 `std::process::Command`，无 shell 注入（参数数组传参）；cwd 设为 repo path

#### D-03 状态（state.rs）

```json
{ "repos": { "<local_path>": { "last_seen_sha": "...", "last_pulled_at": "ISO8601", "last_result": "ok|error" } } }
```

- 启动时加载，变更即写（原子写：写临时文件再 rename）

#### D-04 重建分发（rebuild.rs）

- `mode=off`：直接返回成功
- `mode=command`：`sh -c <command>`（cwd=repo path），捕获 stdout/stderr 尾部 2000 字符用于通知
- `mode=dsh-headless`：调用 spec 03 执行器（spawn `dsh --profile headless "<prompt>"`，需要 DEEPSEEK_API_KEY；无 key 时降级为 command/off 并提示）

### 数据模型

- `RepoConfig { name, local_path, remote: "origin", branch: "master", auto_pull: "ff-only", rebuild: RebuildConfig }`
- `RebuildConfig { mode: "off"|"command"|"dsh-headless", command?: String, prompt?: String }`
- `WatcherState { repos: HashMap<String, RepoState> }`

### 接口契约

- Tauri command：`poll_now() -> Result<Vec<PollResult>>`（手动触发一轮）、`get_watcher_status() -> WatcherStatus`
- 事件：`update-found`、`update-pulled`、`rebuild-result`（前端历史展示，可选）
