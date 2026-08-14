---
spec: '05-00-ci-build-release'
scene: '01-desktop-shell'
status: draft
priority: P1
created: '2026-08-14'
---

# 05-00 Ci Build Release - 需求

## L0 摘要

GitHub Actions 一键打包 dsh-desktop 桌面端：macOS arm64 + Windows x64 两个安装包，提交 GitHub 后从 Actions 产出并可下载。

## L1 概览

### 目标

- 仓库推送到 GitHub 后，`.github/workflows/build.yml` 能在 macOS（arm64）与 Windows（x64）两个 runner 上完成前端 + Rust + Tauri 打包
- 打 tag（`v*`）时自动把两个平台的安装包挂到 GitHub Release
- 手动触发（workflow_dispatch）也能随时重打包
- 产物：macOS arm64 → .dmg/.app；Windows x64 → .msi/.exe（NSIS）

### 用户故事

- 作为 dsh-desktop 使用者，我希望每次发版都能从 GitHub Release 直接下载当前平台安装包，以便无需本地 Rust 工具链
- 作为开发者，我希望 CI 只打包 mac arm64 和 win x64 两个目标，以便覆盖主力平台且不浪费构建时间

### 范围

**包含**：
- GitHub Actions workflow：双平台矩阵（aarch64-apple-darwin、x86_64-pc-windows-msvc）
- 依赖安装（pnpm frozen-lockfile + Rust target）、构建缓存（pnpm store / cargo / frontend dist）
- 产物上传（Actions artifact）与 tag 发布（GitHub Release assets）
- 仓库初始化：git init、.gitignore 生效、提交并推送到 GitHub

**不包含**：
- macOS 公证/签名（无 Apple 开发者证书，保持未签名）
- Windows 代码签名（无证书）
- macOS x86_64、Windows arm64、Linux 打包
- Tauri 自动更新（updater）签名与发布

## L2 详情

### 详细需求

#### F-01 双平台构建矩阵
- 只构建 `aarch64-apple-darwin`（macos runner）与 `x86_64-pc-windows-msvc`（windows runner）两个 target
- 验收：workflow 矩阵只有这 2 个条目，且产物路径为 `src-tauri/target/<triple>/release/bundle/`（带 target 目录，否则找不到 bundle）

#### F-02 产物上传与 Release 发布
- 每次构建上传 Actions artifact（名称含平台+架构）
- push `v*` tag 时自动创建 GitHub Release 并把两个平台安装包挂为 assets
- workflow_dispatch 手动触发可跳过发布、只出 artifact
- 验收：`gh run download` 能取到两个平台的安装包；Release 页面能看到 assets

#### F-03 构建缓存与依赖
- pnpm store 缓存、cargo 缓存（按 target 隔离）、前端依赖复用
- 验收：同一 commit 重复触发时安装与编译明显复用缓存（workflow 含 cache 步骤）

### 非功能性需求

- 性能：全量首次构建 ≤ 30 分钟（含 Rust 编译）；缓存命中后 ≤ 10 分钟
- 兼容性：Node 22 + pnpm 9（lockfile v9）；Rust stable 带对应 target

### 边界与依赖

- 依赖 tauri-shell（01-00）产物可编译；icons 已齐全（src-tauri/icons/）
- 不依赖任何签名证书；发布不需要 secrets（仅 GITHUB_TOKEN）

### 验收标准

- [ ] 仓库已推送 GitHub，`.github/workflows/build.yml` 有效且矩阵仅含 mac arm64 + win x64
- [ ] 从 GitHub Actions 触发一次构建成功，`gh run download` 得到 .dmg（mac arm64）与 .msi/.exe（win x64）
- [ ] 打 v* tag 后 Release 自动挂上两个平台的安装包
