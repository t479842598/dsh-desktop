---
spec: '01-00-tauri-shell'
scene: '01-desktop-shell'
status: draft
priority: P0
created: '2026-08-14'
---

# 01-00 Tauri Shell - 需求

## L0 摘要

Tauri 2 桌面壳：启动 dsh Web 服务（sidecar），主窗口加载其 Web UI，提供托盘、通知、退出清理，作为 dsh 的 macOS 桌面前端。

## L1 概览

### 目标

- 双击启动 → 自动拉起 dsh Web 服务（`http://127.0.0.1:3080`）→ 主窗口加载 UI
- 退出应用时自动回收 dsh 子进程，不留孤儿进程
- 托盘常驻：打开 UI / 检查更新 / 退出
- 配置以 `~/.dsh-desktop/config.json` 持久化，提供示例文件

### 用户故事

- 作为 dsh 用户，我希望桌面端一键启动 Web UI，以便像原生应用一样使用 dsh
- 作为 dsh 用户，我希望关掉窗口后应用仍在托盘运行，以便后台持续接收更新提醒

### 范围

**包含**：
- Tauri 2 项目骨架（Rust 壳 + 前端构建）
- dsh Web 服务进程管理（spawn、端口就绪探测、退出回收）
- 主窗口加载 `http://127.0.0.1:3080`
- 托盘图标与菜单、系统通知出口
- 配置文件读写（config.json + 默认值 + 示例）

**不包含**：
- 修改 dsh 内核代码（外壳分离，dsh 以 npm 包/独立命令方式被拉起）
- 自动更新管线（见 02-00-auto-update-pipeline）
- 模型重建（见 03-00-model-rebuild）
- 远程访问认证（见 04-00-remote-access）

## L2 详情

### 详细需求

#### F-01 Tauri 2 项目骨架
- 描述：在 `dsh-desktop/` 建立 Tauri 2 + 前端（Vite）工程，`cargo build` 与 `pnpm tauri dev` 可跑通
- 验收：`pnpm tauri dev` 启动后出现应用窗口；`cargo check` 无错误

#### F-02 dsh Web 服务管理
- 描述：Rust 侧管理 dsh Web 子进程：启动命令可配置（默认 `npx @deepseek-ai/dsh web`，支持自定义命令/路径），启动后轮询 `127.0.0.1:3080` 直至就绪，应用退出时 kill 子进程
- 验收：服务未启动时窗口显示等待/重试；应用退出后 `pgrep -f "dsh web"` 无残留

#### F-03 主窗口加载 dsh Web UI
- 描述：就绪后主窗口加载 `http://127.0.0.1:3080`；未就绪时显示本地占位页（加载中 + 重试按钮）
- 验收：dsh Web UI 正常交互；服务崩溃后占位页可触发重启

#### F-04 托盘与系统通知
- 描述：托盘图标常驻，菜单含「打开 dsh」「立即检查更新」「打开配置」「退出」；系统通知通过本壳统一的 notify 出口发出（macOS 用 osascript）
- 验收：托盘菜单各项可用；通知能出现在 macOS 通知中心

#### F-05 配置文件
- 描述：`~/.dsh-desktop/config.json` 支持：`dsh`（启动命令、端口、超时）、`poll_interval_sec`、`repos[]`（自动更新管线使用）、`notify.enabled`、`remote`（远程访问，见 spec 04）；缺失时写默认值；提供 `config.example.json` 与「打开配置」入口
- 验收：首次启动生成默认配置；修改配置后重启生效

#### F-06 跨平台支持（macOS + Windows，ARM + x64）
- 描述：壳工程支持 macOS（arm64/x86_64）与 Windows（x86_64/arm64）四目标构建；`tauri.conf.json` 配置各平台 bundle 目标；提供 GitHub Actions 矩阵构建 workflow（macos-latest / windows-latest，含 arm64 target）
- 验收：`pnpm tauri build` 在当前平台产出安装包；CI workflow 覆盖 4 目标矩阵；代码中无平台特定阻塞（路径用 `dirs`，命令用跨平台 spawn）

## 验收标准（整体）

- [ ] `pnpm tauri dev` 一键跑通，主窗口加载 dsh Web UI
- [ ] 退出无孤儿进程
- [ ] 托盘 + 通知可用
- [ ] 配置可持久化、可编辑
