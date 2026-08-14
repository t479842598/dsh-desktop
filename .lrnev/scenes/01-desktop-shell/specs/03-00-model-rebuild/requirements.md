---
spec: '03-00-model-rebuild'
scene: '01-desktop-shell'
status: draft
priority: P1
created: '2026-08-14'
---

# 03-00 Model Rebuild - 需求

## L0 摘要

自动更新管线拉取新代码后，可选调用 dsh headless agent（DeepSeek 模型）执行"拉取-构建-修复"任务，实现模型驱动的自动重建；无 key 或失败时优雅降级。

## L1 概览

### 目标

- `rebuild.mode=dsh-headless` 时，拉取成功后调用 `dsh --profile headless` 让模型完成构建任务
- 任务 prompt 可配置（模板含仓库名、分支、更新摘要），模型可自行 git pull、跑构建、修错误
- 无 `DEEPSEEK_API_KEY` 时降级为 command 模式（若配了 command）或提示用户
- 构建失败结果通知用户，可配置重试

### 用户故事

- 作为用户，我希望代码更新后模型自动构建并修复失败，以便一键完成"更新→可用"

### 范围

**包含**：
- dsh headless 调用封装（spawn 子进程、env 传递、输出捕获）
- 任务 prompt 模板
- 结果通知与失败处理

**不包含**：
- 在 dsh 内核中新增插件（外壳分离；headless 是 dsh 现成 CLI 能力）
- 模型选型配置（用 dsh profile 默认模型）

## L2 详情

### 详细需求

#### F-01 dsh headless 调用
- 描述：`mode=dsh-headless` 时执行 `dsh --profile headless "<prompt>"`（命令路径可配置，默认 npx），继承环境变量（含 DEEPSEEK_API_KEY），cwd=仓库路径，超时可配（默认 15 分钟）
- 验收：有 key 时模型完成构建任务并输出结果；无 key 时明确提示并降级

#### F-02 Prompt 模板
- 描述：默认模板：`仓库 <name> 分支 <branch> 有新提交（<更新摘要>），请拉取最新代码、执行构建、如失败请修复，最后报告结果`；可配置自定义 prompt
- 验收：模板渲染正确，更新摘要是真实 git log

#### F-03 结果处理
- 描述：模型任务退出码/输出 → 成功/失败通知；失败可配置自动重试一次（`retry_on_failure`，默认 true）
- 验收：成功通知含模型结论摘要；失败通知含错误片段；重试逻辑生效

## 验收标准（整体）

- [ ] 更新后模型自动执行构建任务并通知结果
- [ ] 无 key 降级不崩溃
- [ ] prompt 可配置
