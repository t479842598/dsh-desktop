---
spec: 06-00-connection-mode
scene: 01-desktop-shell
status: completed
priority: P0
created: '2026-08-15'
updated: '2026-08-15'
---

# 06-00 Connection Mode - 需求

## L0 摘要

壳层支持两种连接模式：**本地模式**（默认，dsh 子进程起 127.0.0.1:3080，WebView 加载本地 UI）与**远程模式**（用户配置远程 URL + Basic Auth 账号密码，打开远程 dsh Web UI）。设置页可切换模式并保存。

## L1 概览

### 目标

- 默认本地模式，行为与现状完全一致（回归保护）
- 设置页可切换为远程模式：填远程 URL + 账号密码，保存后"打开 dsh Web UI"加载远程地址
- 远程凭证随请求携带（Basic Auth），不落明文到前端以外的地方

### 用户故事

- 作为用户，我希望在设置里切换本地/远程模式，以便在公司外网通过远程 URL 访问自己的 dsh
- 作为用户，我希望保存后立即生效，不用手动编辑配置文件

### 范围

**包含**：
- config.rs 新增 `connection.mode`（`local` | `remote`）与 `connection.remote`（url/username/password）
- 前端设置页：模式单选 + 远程 URL/账号/密码输入 + 保存
- 打开 dsh Web UI 时按当前模式选择加载目标
- 状态页显示当前模式

**不包含**：
- 无端口模式（sidecar IPC，留待远期）
- 一键安装本地服务
- 远程模式下 dsh 子进程的生命周期管理（远程模式不启本地 dsh）

## L2 详情

### 详细需求

#### F-01 配置结构
- 描述：`AppConfig` 增加 `connection` 段：`mode: "local"|"remote"`、`remote.url`、`remote.username`、`remote.password`；默认 `local`
- 验收：默认配置无 connection 段时按 local 处理；保存后持久化到 config.json

#### F-02 设置页切换
- 描述：前端新增"设置"tab：连接模式单选（本地/远程），远程模式显示 URL/账号/密码输入；保存写配置
- 验收：WHEN 用户切到远程并填 URL+账号密码保存 THEN 配置文件更新且状态页显示"远程模式"
- 验收：WHEN 用户切回本地保存 THEN 恢复本地 3080 行为

#### F-03 打开 UI 按模式路由
- 描述："打开 dsh Web UI"按钮：本地→`http://127.0.0.1:{port}`；远程→配置的远程 URL（凭证经 URL userinfo 或请求头携带）
- 验收：WHEN 本地模式打开 THEN 加载 127.0.0.1:3080；WHEN 远程模式打开 THEN 加载远程 URL

#### F-04 远程凭证携带
- 描述：远程打开时携带 Basic Auth 凭证（URL userinfo 方案，WebView 自动解析）；无账号密码时提示
- 验收：WHEN 远程 URL 配置了账号密码 THEN 打开时凭证生效；WHEN 未配置 THEN 提示需配置

## 验收标准

- [x] 默认启动为本地模式，行为与现状一致
- [x] 设置页可切远程 + 填 URL/账号/密码并保存
- [x] 远程模式打开 UI 加载远程地址且凭证生效
- [x] 状态页显示当前模式
