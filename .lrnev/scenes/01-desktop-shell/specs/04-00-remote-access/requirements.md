---
spec: '04-00-remote-access'
scene: '01-desktop-shell'
status: draft
priority: P0
created: '2026-08-14'
---

# 04-00 Remote Access - 需求

## L0 摘要

壳层提供远程访问认证网关：HTTP Basic Auth（用户名+密码）+ 可信域名/主机校验（参考 pi-web-QT 的 `web-auth.ts` / `request-security.ts`），把远程流量安全转发到本机 dsh Web 服务；凭据在设置页配置，保存后自动重启后端生效。

## L1 概览

### 目标

- 用户可在设置页配置用户名、密码、可信主机，启用后可从局域网/外网访问本机 dsh Web UI
- 默认**不启用**（无凭据 = 纯本机访问），配置保存后后端自动重启生效
- 认证实现防时序攻击（timingSafeEqual），Host 校验防 DNS rebinding
- dsh 本体仍只监听 127.0.0.1（dsh 原生禁止 0.0.0.0），壳层网关负责对外暴露

### 用户故事

- 作为用户，我希望在设置里配置远程访问的用户名密码，以便手机/其他电脑访问本机 dsh
- 作为用户，我希望默认没有任何远程访问能力，以免意外暴露

### 范围

**包含**：
- 设置 UI：用户名、密码、可信主机列表、启用开关
- 壳层认证网关（Rust）：监听 0.0.0.0:配置端口，Basic Auth + Host 校验，转发到 127.0.0.1:3080
- 配置持久化 `~/.dsh-desktop/config.json` 的 `remote` 段
- 保存后自动重启 dsh 后端（网关随壳重启）

**不包含**：
- TLS/HTTPS 终止（远程部署建议前置 Caddy/Cloudflare Tunnel，参考 pi-web deployment.md）
- 多用户管理、token 轮换
- Windows/macOS 系统防火墙的自动放行（文档说明手动放行）

## L2 详情

### 详细需求

#### F-01 认证网关（Basic Auth）
- 描述：壳在配置端口监听 `0.0.0.0`，对每个请求校验 `Authorization: Basic` 头；用户名/密码与配置比对（sha256 + timingSafeEqual，不泄漏是哪个字段错）；校验通过才转发到 `127.0.0.1:3080`
- 验收：错误凭据返回 401（不泄露信息）；正确凭据可访问 dsh UI；未配置密码时网关不启动（或拒绝所有外部请求）

#### F-02 可信主机校验（Host allowlist）
- 描述：配置 `remote.allowed_hosts`（如 `dsh.example.com:3081`、`192.168.1.5:3081`），请求 Host 头不在列表内则 403；默认自动包含本机回环与局域网 IP
- 验收：Host 不在列表返回 403；在列表可访问；防 DNS rebinding

#### F-03 设置页与保存重启
- 描述：设置页可配置启用开关、用户名、密码、可信主机、端口；保存写入 config.json，后端自动重启使新配置生效
- 验收：保存后 dsh 服务自动重启且新凭据生效；默认状态为未启用（无凭据）

#### F-04 网关与 dsh 生命周期绑定
- 描述：网关随壳启动/停止；dsh 子进程崩溃重启时网关自动恢复转发；网关自身监听失败时通知用户
- 验收：kill dsh 后网关仍存活且重启 dsh 后恢复；端口占用时提示用户

## 验收标准（整体）

- [ ] 设置页配置用户名密码后保存，重启生效，远程可访问 dsh UI
- [ ] 错误凭据 401、错误 Host 403
- [ ] 默认不启用远程访问
- [ ] 参考 pi-web-QT 认证实现（web-auth.ts），防时序攻击
