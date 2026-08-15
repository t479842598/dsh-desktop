# dsh-desktop

**DSH desktop** —— DeepSeek Harness 的桌面客户端（Tauri 2 + Rust），完全开源。

> ⚠️ **重要：本项目不自带 DeepSeek harness 运行时。**
> 启动时会调用**你本机已安装的 DeepSeek harness 源码**（见下方「依赖」一节），
> 而不是使用项目自身附带的依赖。请先按「依赖」准备好本地 harness，否则应用无法启动。

## 特性

- **单窗口桌面壳**：主窗口直接加载 dsh Web UI，壳层 UI（灵动岛工具条、设置面板、
  原生右键菜单、启动画面）叠加在页面上，无独立控制台窗口
- **连接模式切换**：本地模式（拉起本机 dsh，`127.0.0.1:3080`）/ 远程模式
  （连接远程实例，支持 Basic Auth 账号密码），保存后热切换
- **灵动岛工具条**：顶部 70% 位置悬浮，平时隐藏、鼠标靠近自动浮现；
  含设置入口与窗口控制（缩小 / 放大 / 关闭），长按可拖动窗口
- **原生右键菜单**：输入框 / 选区 / 空白 / 链接 / 图片五类场景
- **桌面系统通知**：任务完成时弹系统通知（macOS osascript / Windows PowerShell）
- **无边框窗口**：Windows 11 Mica / macOS vibrancy + 窗口状态记忆
- **自动更新管线**：定时检测远程新提交 → 拉取 → 重建 → 通知
- **托盘常驻** + 系统通知

## 依赖

本项目依赖以下**本机环境**（均不自带、不内置）：

| 依赖 | 说明 |
|---|---|
| **DeepSeek harness 源码** | 运行时必需。壳通过 `dsh.command` + `dsh.cwd` 拉起本地 harness（`pnpm dsh web`）。**请使用你自己 clone/构建的 harness 副本**（如 `~/dsh-harness`），而非本仓库自带的任何依赖。 |
| Node.js ≥ 22 | dsh 运行依赖（pnpm 需可用） |
| Rust ≥ 1.77 + Xcode CLT | Tauri 构建需要 |
| pnpm | 前端与构建 |

> 与 harness 源码的关系：改 harness 代码后，把改动同步到你配置的 `dsh.cwd`
> 目录（或直接让 `dsh.cwd` 指向你的工作副本），重启 dsh 即可生效。

## 安装

从 GitHub Releases 下载对应平台的安装包：

- **macOS**：`.dmg`（Apple Silicon）
- **Windows**：`.msi` 或 NSIS `.exe`（x64）

macOS 未公证：首次打开提示“已损坏”时执行
`xattr -dr com.apple.quarantine /Applications/dsh-desktop.app`。
Windows SmartScreen 提示点“仍要运行”。

## 开发

```sh
pnpm install
pnpm tauri dev
```

首次运行生成 `~/.dsh-desktop/config.json`（示例见 `config.example.json`），
日志在 `~/.dsh-desktop/logs/`。

## 配置

`~/.dsh-desktop/config.json`：

| 字段 | 说明 |
|---|---|
| `dsh.command` | dsh web 启动命令（数组形式，无 shell 注入） |
| `dsh.cwd` | **本地 harness 源码目录**（运行时从这里拉起 dsh） |
| `dsh.port` | dsh 监听端口（默认 3080） |
| `connection.mode` | `local` / `remote` |
| `connection.remote` | 远程 URL / 账号 / 密码（远程模式用） |
| `poll_interval_sec` | 更新检查间隔（秒，默认 300） |
| `repos[]` | 仓库监视列表（自动更新管线） |
| `repos[].rebuild.mode` | `off` / `command` / `dsh-headless` |

## 构建与发布

见 [CONTRIBUTING.md](CONTRIBUTING.md)（含 tag 发布流程）与
`.github/workflows/build.yml`（CI 双平台构建）。

| 平台 | 产物 |
|---|---|
| macOS arm64 | `.dmg` / `.app` |
| Windows x64 | `.msi` + NSIS `.exe` |

## 变更记录

见 [CHANGELOG.md](CHANGELOG.md)。

## 项目结构

```
src-tauri/src/
  lib.rs       应用装配、托盘、commands、事件桥接
  shell.rs     注入到 dsh 页面的壳层 UI（灵动岛/设置/右键/通知）
  config.rs    配置读写
  dsh.rs       dsh 子进程管理（跨平台）
  git.rs       git 命令封装
  watcher.rs   自动更新轮询
  rebuild.rs   重建分发（command / dsh-headless）
  state.rs     去重状态持久化
  notify.rs    系统通知
src/           前端（启动页）
```

## License

[MIT](LICENSE)
