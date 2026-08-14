# dsh-desktop

DeepSeek Harness 的桌面壳（Tauri 2 + Rust）。

- 自动拉起 dsh Web 服务（`http://127.0.0.1:3080`），主窗口加载其 UI
- 托盘常驻 + 系统通知（macOS osascript）
- 自动更新管线：定时 git fetch 检测远程新提交 → 自动拉取（策略可配）→ 重建（命令或 dsh headless 模型）→ 通知
- 双击启动即拉起 dsh Web 服务并打开界面（无远程访问，局域网组网用 Tailscale）

## 环境要求

- Node.js ≥ 22（dsh 运行依赖）
- Rust ≥ 1.77 + Xcode CLT（Tauri 构建）
- pnpm

## 开发

```sh
pnpm install
pnpm tauri dev
```

首次运行生成 `~/.dsh-desktop/config.json`（默认配置），日志在 `~/.dsh-desktop/logs/`。

## 配置

`~/.dsh-desktop/config.json`（示例见 `config.example.json`）：

| 字段 | 说明 |
|---|---|
| `dsh.command` | dsh web 启动命令（数组形式，无 shell 注入） |
| `dsh.port` | dsh 监听端口（默认 3080） |
| `poll_interval_sec` | 更新检查间隔（秒，默认 300） |
| `repos[]` | 仓库监视列表 |
| `repos[].auto_pull` | `off` / `ff-only`（默认，安全）/ `merge` / `reset`（破坏性） |
| `repos[].rebuild.mode` | `off` / `command` / `dsh-headless` |

### 自动更新凭据

复用本地 git 凭据（gh CLI 登录态 / macOS keychain），无需额外配置。示例仓库指向
`~/dsh-harness`（deepseek-harness 主卷副本，remote=origin）。修改 `local_path`/`remote`/`branch`
可监视任意本地 clone；fork 库可把 `remote` 配为指向自己 fork 的 remote 名。

> **重要部署约束**：dsh 源码必须位于**主卷**（如 `~/dsh-harness`），不能放在外置卷
> （`/Volumes/...`）。macOS 上 node 在 GUI 应用上下文里对**外置卷目录**调用 `getcwd`
> 会永久挂起（表现为 dsh 启动后不监听端口、卡在 process.cwd()），这是系统级行为，
> 无代码可绕。同步上游更新时用：
> ```sh
> rsync -a "/Volumes/1T 原装/项目研发/deepseek-harness/" ~/dsh-harness/
> ```
> （或直接把监视仓库的 local_path 配成外置卷，但 dsh 服务本体必须跑在主卷副本上。）

### 模型重建（dsh-headless）

`rebuild.mode = "dsh-headless"` 时，拉取后调用 `dsh --profile headless` 让模型完成
"拉取-构建-修复"。需要 `DEEPSEEK_API_KEY` 环境变量；无 key 时降级为 `command` 模式（若配置）。

## 构建与发布

```sh
pnpm tauri build        # 当前平台安装包
```

跨平台打包由 `.github/workflows/build.yml` 驱动，只出两个目标：

| 平台 | Runner | 产物 |
|---|---|---|
| macOS arm64 | `macos-14`（Apple Silicon） | `.dmg` / `.app` |
| Windows x64 | `windows-latest` | `.msi` + NSIS `.exe` |

- **打 tag**（`v*`）→ 自动构建并创建 GitHub Release，两个安装包挂为 assets
- **手动触发** → Actions 页面 `Run workflow`，产物在 run 的 Artifacts 里下载
- 未签名未公证：macOS 首次打开需右键→打开；Windows 有 SmartScreen 提示

```sh
gh run download <run-id> --dir dist-download   # 拉取全部产物
```

## 项目结构

```
src-tauri/src/
  lib.rs       应用装配、托盘、commands
  config.rs    配置读写
  dsh.rs       dsh 子进程管理
  git.rs       git 命令封装
  watcher.rs   自动更新轮询
  rebuild.rs   重建分发（command / dsh-headless）
  state.rs     去重状态持久化
  notify.rs    系统通知
src/           前端（状态页）
```
