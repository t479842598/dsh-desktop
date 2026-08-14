---
spec: '03-00-model-rebuild'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 03-00 Model Rebuild - 设计

## L0 摘要

复用 dsh 现成 CLI `--profile headless` 做模型重建：watcher 拉取后 spawn headless 任务，模型自主完成构建与修复，结果走通知。

## L1 概览

### 架构思路

- **零内核改动**：headless 是 dsh 产品能力（`dsh --profile headless "task"`），壳只负责 spawn 与结果收集
- **env 透传**：子进程继承壳环境，`DEEPSEEK_API_KEY` 从用户环境/`.env` 读取（dsh 自身支持根 `.env`）
- **降级链**：dsh-headless 无 key → command 模式（若配置）→ 提示用户

### 主要模块

- `src-tauri/src/rebuild.rs` 内 `run_headless(repo, prompt) -> RebuildResult`

### 关键决策

| 决策 | 选项 | 倾向 | 是否产 ADR |
|---|---|---|---|
| 模型执行方式 | dsh 插件 / headless CLI | headless CLI（现成能力，零改动） | 否 |
| key 来源 | 壳配置 / 环境变量 | 环境变量 + 仓库 `.env`（dsh 原生支持） | 否 |

## L2 详情

### 模块详细设计

#### D-01 headless 执行（rebuild.rs）

1. 检查 env 有 `DEEPSEEK_API_KEY`（或仓库根 `.env` 存在），否则降级
2. `Command::new(dsh_cmd[0]).args(...).arg("--profile").arg("headless").arg(prompt)`，cwd=repo path，env 继承
3. 捕获 stdout/stderr（尾部 3000 字符），超时（默认 900s）则 kill 并标记失败
4. 退出码 0 → 成功；非 0 → 失败（含输出摘要）
5. `retry_on_failure`（默认 true）时失败自动重跑一次

#### D-02 Prompt 模板（rebuild.rs）

```rust
fn render_prompt(repo, branch, summary, custom) -> String {
  custom.unwrap_or(format!(
    "仓库 {repo} 分支 {branch} 有新提交：{summary}\n\
     请执行：1) 拉取最新代码；2) 运行构建；3) 失败则分析并修复；4) 报告结果。"
  ))
}
```

- 摘要来自 spec 02 `log_summary`（`git log --oneline old..new` 前 20 条）

### 数据模型

- `RebuildResult { success: bool, summary: String, output_tail: String, mode: "command"|"dsh-headless"|"skipped" }`

### 接口契约

- 被 spec 02 rebuild 分发器调用；结果经 notify 出口通知，并写入 state.json 的 `last_result`
