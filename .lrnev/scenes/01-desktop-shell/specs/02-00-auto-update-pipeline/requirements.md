---
spec: '02-00-auto-update-pipeline'
scene: '01-desktop-shell'
status: draft
priority: P0
created: '2026-08-14'
---

# 02-00 Auto Update Pipeline - 需求

## L0 摘要

后台守护定时检查配置仓库的远程更新（git fetch + SHA 对比），有新提交时自动拉取（策略可配）、触发重建（命令或模型）、桌面通知；凭据复用本地 gh/git 登录态。默认配置用户 fork 库 `t479842598/deepseek-harness`。

## L1 概览

### 目标

- 每 `poll_interval_sec`（默认 300s）检查一次配置仓库的远程分支
- 发现新提交 → 通知 → 按策略自动拉取（ff-only/merge/reset）→ 触发重建 → 完成通知
- 全程使用本地 git 凭据（gh CLI 登录态 / keychain），无需额外输入密码
- 仓库列表、拉取策略、重建方式全部可配置

### 用户故事

- 作为用户，我希望 fork 的库上游有新提交时桌面端自动提醒并拉取，以便本地始终最新
- 作为用户，我希望拉取后自动触发构建（可选手动、命令、模型），以便一键完成更新闭环

### 范围

**包含**：
- watcher 后台任务：git fetch、SHA 对比、状态去重（避免重复通知同一提交）
- 自动拉取：策略 ff-only / merge / reset（可配置，默认 ff-only）
- 重建触发：off / 命令 / dsh-headless（spec 03 实现模型重建）
- 通知：检测到更新 / 拉取完成 / 重建结果
- 默认配置示例指向用户 fork 库 `https://github.com/t479842598/deepseek-harness`（本地路径 /Volumes/1T 原装/项目研发/deepseek-harness）

**不包含**：
- GitHub API 轮询（用 git fetch 即可，天然增量、零配额）
- 跨仓库依赖、构建产物发布
- 多账号管理（只用本地已登录账号）

## L2 详情

### 详细需求

#### F-01 定时检查（git fetch + SHA 对比）
- 描述：每 `poll_interval_sec` 对每个 repo 执行 `git -C <path> fetch <remote>`，对比本地 HEAD 与 `FETCH_HEAD`/`<remote>/<branch>`；有新提交则进入拉取流程
- 验收：上游 push 新提交后下一轮轮询能检测到；无新提交时不通知；已通知过的 SHA 不重复通知

#### F-02 自动拉取（策略可配）
- 描述：`auto_pull` 取值 `off`（仅通知）/ `ff-only`（默认，fast-forward，有本地改动则失败并通知）/ `merge`（merge 远程分支）/ `reset`（`git reset --hard`，破坏性，需配置显式开启）；拉取失败（冲突/脏工作区）时通知用户并保留现场
- 验收：ff-only 下干净工作区自动快进；脏工作区不破坏现场并通知；reset 模式仅在显式配置时可用

#### F-03 重建触发
- 描述：拉取成功后按 `rebuild.mode` 执行：`off`（不重建）/ `command`（执行 `rebuild.command`，如 `pnpm install && pnpm build`）/ `dsh-headless`（spec 03）；重建结果（成功/失败+摘要）通知
- 验收：command 模式执行指定命令并捕获退出码；失败通知包含输出摘要

#### F-04 桌面通知
- 描述：三类通知：发现新提交（数量+仓库）、拉取完成、重建结果；走壳统一 notify 出口；`notify.enabled=false` 时仅记日志
- 验收：通知出现在 macOS 通知中心；禁用开关生效

#### F-05 可配置仓库
- 描述：`repos[]` 支持多个仓库，字段：`name`、`local_path`、`remote`（默认 origin）、`branch`（默认 master）、`auto_pull`、`rebuild`；默认配置写入用户 fork 库示例
- 验收：修改 config.json 后下一轮轮询生效（无需重启）；多仓库并行检查互不阻塞

## 验收标准（整体）

- [ ] 上游新提交 → 自动检测 → 通知 → 自动拉取 → 重建（按配置）
- [ ] 凭据复用本地 gh 登录态，全程无需输入密码
- [ ] 所有行为可配置，默认配置指向用户 fork 库
- [ ] 去重：同一提交不重复通知
