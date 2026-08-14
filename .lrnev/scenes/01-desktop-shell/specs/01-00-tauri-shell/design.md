---
spec: '01-00-tauri-shell'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 01-00 Tauri Shell - 设计

## L0 摘要

Tauri 2（Rust 壳 + 系统 WebView）作为 dsh Web 服务的桌面容器：进程管理在 Rust 侧，UI 复用 dsh 现有 React Web 应用，零前端改造。

## L1 概览

### 架构思路

- **外壳分离**：不 fork、不改 dsh 内核；壳只负责拉起 dsh 进程、加载其 UI、提供托盘/通知/配置
- **主窗口远程加载**：Tauri 主窗口直接加载 `http://127.0.0.1:3080`（dev 与生产一致），本地前端仅用于未就绪占位页与后续状态面板
- **进程生命周期**：dsh 子进程跟随壳进程；退出时先 kill 再退，防止孤儿
- **配置即文件**：`~/.dsh-desktop/config.json`，每次轮询/操作前重读，支持热改（重启生效的最小闭环）

### 主要模块

| 模块 | 职责 |
|---|---|
| `src-tauri/src/lib.rs` | Tauri 应用装配、托盘、菜单、命令注册 |
| `src-tauri/src/dsh.rs` | dsh 子进程 spawn / 就绪探测 / kill / 重启 |
| `src-tauri/src/config.rs` | 配置读写（默认值、示例、目录保证） |
| `src-tauri/src/notify.rs` | 系统通知出口（osascript 实现） |
| `src-tauri/src/watcher.rs` | 自动更新管线（spec 02 挂载点） |
| `src/`（前端） | 占位页/状态页（Vite + 原生 TS，轻量） |

### 关键决策

| 决策 | 选项 | 倾向 | 是否产 ADR |
|---|---|---|---|
| 壳框架 | Electron / Tauri 2 | Tauri 2（体积小、系统 WebView、Rust 侧进程管理直接） | 是 |
| dsh 集成方式 | fork 内核 / 外部命令拉起 | 外部命令拉起（npm 包或用户指定命令），不 fork | 是 |
| 主窗口内容 | 本地前端 / 远程加载 dsh UI | 远程加载 `127.0.0.1:3080` | 否 |
| 通知实现 | tauri-plugin-notification / osascript | osascript（零权限配置、最稳） | 否 |

## L2 详情

### 模块详细设计

#### D-01 进程管理（dsh.rs）

- spawn：`Command::new(配置.command)`（默认 `npx @deepseek-ai/dsh web`），继承环境变量（DEEPSEEK_API_KEY 等），stdout/stderr 重定向到日志文件 `~/.dsh-desktop/logs/dsh.log`
- 就绪探测：每 500ms `TcpStream::connect("127.0.0.1:3080")`，超时（默认 30s）后报错可重试
- 回收：`app.exit(0)` 时 `child.kill()` + `wait()`；壳崩溃兜底用进程组/`killpg`（macOS 支持）
- 重启：占位页重试按钮 → kill 旧进程 → 重新 spawn → 再次探测

#### D-02 配置（config.rs）

```jsonc
{
  "dsh": {
    "command": ["npx", "@deepseek-ai/dsh", "web"],   // 可自定义，如 ["node", "/path/to/dsh/lib/bin.js", "web"]
    "port": 3080,
    "ready_timeout_sec": 30
  },
  "poll_interval_sec": 300,
  "notify": { "enabled": true },
  "repos": [
    {
      "name": "deepseek-harness",
      "local_path": "/Volumes/1T 原装/项目研发/deepseek-harness",
      "remote": "origin",          // fetch 的 remote 名
      "branch": "master",
      "auto_pull": "ff-only",      // off | ff-only | merge | reset
      "push_to_fork": "",          // 可选：同步到 fork 的 remote 名/URL
      "rebuild": { "mode": "off" } // off | command | dsh-headless（spec 03）
    }
  ]
}
```

- 首次启动：若文件不存在则写默认值（含示例 repo，`rebuild.mode=off`，不自动执行危险操作）
- 读取：每次轮询前 `read_config()` 重新解析，语法错误时保留上次有效配置并通知

#### D-03 托盘（lib.rs）

- 菜单：打开 dsh（show 主窗口）/ 立即检查更新（触发 watcher 轮询）/ 打开配置（系统编辑器打开 config.json）/ 退出
- 关闭窗口不退出应用（`tauri.conf.json` 不设 exit_on_close，拦截 close 事件 hide）

#### D-04 通知（notify.rs）

- `notify(title, body) -> Result<()>`：macOS 用 `osascript -e 'display notification "<body>" with title "<title>"'`
- 配置 `notify.enabled=false` 时静默；失败降级为日志

### 数据模型

- `AppState`（Tauri managed）：`Mutex<DshProcess>`（子进程句柄 + 就绪状态）+ `Mutex<WatcherState>`（spec 02 状态）
- 配置结构体 `AppConfig` 与 JSON 一一对应，`serde` 反序列化，未知字段忽略

### 接口契约

- Tauri commands：`get_status`（dsh 是否就绪、上次轮询结果）、`restart_dsh`、`open_config`、`poll_now`
- 事件：`dsh-ready`、`dsh-crashed`（前端占位页监听）
