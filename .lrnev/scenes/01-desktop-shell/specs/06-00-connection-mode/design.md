---
spec: '06-00-connection-mode'
scene: '01-desktop-shell'
created: '2026-08-15'
---

# 06-00 Connection Mode - 设计

## L0 摘要

壳层支持本地/远程双连接模式：`config.json` 的 `connection` 段保存模式与远程凭证，前端设置页切换，打开 UI 时按模式路由（本地→127.0.0.1:port，远程→远程 URL 带 Basic Auth userinfo）。

## L1 概览

### 架构思路

- 复用现有 `DshConfig`（本地 dsh 子进程）；新增 `ConnectionConfig` 独立于 `RemoteConfig`（后者是"对外暴露网关"，前者是"连接远程实例"，方向相反）
- 远程模式：boot 跳过本地 dsh 启动（省资源）；打开 UI 用 `WebviewWindowBuilder` 加载远程 URL，凭证经 URL userinfo（`https://user:pass@host`）由 WebView 自动携带
- 窗口 label 区分：本地 `"dsh"` / 远程 `"dsh-remote"`，避免切模式时复用旧窗口

### 主要模块

| 模块 | 职责 |
|---|---|
| `config.rs` | `ConnectionConfig`（mode + remote.url/username/password）读写 |
| `lib.rs` | `save_connection` command、`open_dsh_window` 按模式路由、`remote_url_with_auth`、boot 远程跳过 |
| 前端 `main.ts` | 设置 tab、模式单选、远程表单、保存、状态页模式显示 |

### 关键决策

| 决策 | 选项 | 倾向 | 是否产 ADR |
|---|---|---|---|
| 凭证携带 | URL userinfo / 请求头注入 | URL userinfo（WebView 原生解析，零额外代码） | 否 |
| 远程模式本地 dsh | 照常启动 / 跳过 | 跳过（省资源，远程模式不需要本地实例） | 否 |
| 窗口复用 | 同 label 复用 / 分 label | 分 label（避免切模式后加载旧地址） | 否 |

## L2 详情

### 模块详细设计

- `save_connection(mode, remote_url, remote_username, remote_password)`：校验 mode ∈ {local, remote}，远程模式要求 URL 非空；写配置；不强制重启（本地 dsh 状态由模式决定）
- `remote_url_with_auth(cfg)`：URL 非空且 username 非空时，在 `://` 后插入 `percent_encode(user):percent_encode(pass)@`
- `open_dsh_window`：远程模式窗口 title 带"（远程）"，label 用 `dsh-remote`
- boot：`connection.mode == "remote"` 时仅启动 watcher，跳过 dsh 子进程与就绪探测

### 测试要点

- 默认配置无 connection 段 → 按 local 处理
- 切远程保存后 status 显示远程模式、打开 UI 走远程 URL
- 切回本地恢复 3080 行为
