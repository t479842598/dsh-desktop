---
spec: '04-00-remote-access'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 04-00 Remote Access - 设计

## L0 摘要

Rust 侧轻量 HTTP 认证网关：`0.0.0.0:port` 监听 → Basic Auth（sha256+timingSafeEqual）→ Host allowlist → 转发 `127.0.0.1:3080`。dsh 保持 loopback-only，网关是唯一对外入口。

## L1 概览

### 架构思路

- 参考 pi-web-QT：`lib/web-auth.ts`（credentialsMatch 双比较防泄漏 + timingSafeEqual）与 `lib/request-security.ts`（PI_WEB_ALLOWED_HOSTS）
- 网关不做 TLS（文档指引用 Caddy/Cloudflare Tunnel 前置，与 pi-web deployment.md 一致）
- 转发用最小 HTTP 代理：读请求行+头 → 校验 → 与后端建立 TCP 连接 → 双向 pipe（HTTP 与 WebSocket 都适用，因为只代理字节流，不做协议解析）

### 主要模块

| 模块 | 职责 |
|---|---|
| `src-tauri/src/remote.rs` | 网关：监听、认证、Host 校验、双向转发 |
| `src-tauri/src/config.rs` | `remote` 段读写（与 spec 01 共用） |
| 设置页（前端） | 用户名/密码/可信主机/端口表单 + 保存 |

### 关键决策

| 决策 | 选项 | 倾向 | 是否产 ADR |
|---|---|---|---|
| 认证位置 | dsh 层 / 壳层网关 | 壳层网关（dsh 禁 0.0.0.0，且不 fork 内核） | 是 |
| 转发实现 | 完整 HTTP 代理库 / 字节流双向 pipe | 字节流 pipe（HTTP+WS 通吃，代码少） | 否 |
| 凭据存储 | 明文 config.json / keychain | config.json（参考 pi-web 同款做法；文档警告勿入库） | 否 |

## L2 详情

### 模块详细设计

#### D-01 网关请求流（remote.rs）

1. `TcpListener::bind(0.0.0.0:port)`（仅当 `remote.enabled && password 非空`）
2. 每连接：读缓冲直到 `\r\n\r\n`，解析首行与 Host 头、Authorization 头
3. Host 校验：`allowed_hosts` 包含请求 Host（精确匹配，允许 host 或 host:port）→ 否则 403 响应
4. Basic Auth 校验：`sha256(提供的 user:pass)` 与 `sha256(配置 user:pass)` timingSafeEqual；用户名与密码分开比较防枚举 → 否则 401
5. 通过后：`TcpStream::connect(127.0.0.1:3080)`，把已读缓冲 + 后续字节双向转发（tokio `io::copy_bidirectional`）
6. 后端断开即断开客户端

- WebSocket 无需特殊处理（字节流代理天然支持 upgrade）
- 失败响应为最小 HTTP/1.1 文本响应（401 带 `WWW-Authenticate: Basic realm="dsh"`）

#### D-02 配置（config.rs remote 段）

```jsonc
"remote": {
  "enabled": false,
  "port": 3081,
  "username": "",
  "password": "",
  "allowed_hosts": []        // 空 = 自动：仅本机回环 + 检测到的局域网 IP
}
```

- `enabled=true` 但 `password` 为空 → 视为未配置，网关不启动并在设置页提示
- `allowed_hosts` 自动补充：`127.0.0.1[:port]`、`localhost[:port]`、检测到的 LAN IPv4（复用 dsh `resolveLanTrust` 思路，Rust 侧用 `get_if_addrs` 或解析 `ifconfig`）

#### D-03 设置页（前端）

- 表单：启用开关、用户名、密码（password input）、可信主机（逗号分隔）、端口
- 保存 → invoke `save_remote_config` → 写 config.json → 触发后端重启（spec 01 的 `restart_dsh`）
- 显示当前状态（网关运行中/未启用）

#### D-04 生命周期

- 壳启动时：先启动网关（若配置启用）再启动 dsh
- `restart_dsh`：网关不受影响（网关不依赖 dsh 存活）；dsh 就绪后网关自动恢复转发
- 端口占用：bind 失败 → notify + 设置页显示错误

### 数据模型

- `RemoteConfig { enabled: bool, port: u16, username: String, password: String, allowed_hosts: Vec<String> }`
- 网关状态：`Mutex<Option<RemoteGateway>>`（含监听端口与 JoinHandle）

### 接口契约

- Tauri command：`save_remote_config(RemoteConfig) -> Result<()>`、`get_remote_config() -> RemoteConfig`
- 事件：`remote-status`（运行中/未启用/端口错误）
