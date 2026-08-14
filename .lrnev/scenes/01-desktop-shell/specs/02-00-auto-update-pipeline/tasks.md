---
spec: '02-00-auto-update-pipeline'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 02-00 Auto Update Pipeline - 任务清单

> 任务由 lrnev `task_create` 工具创建，不要手编。
> 状态机：pending → in_progress → completed / failed；blocked 可回 in_progress；failed 可回 pending 重试。

## 阶段 1

<!-- FILL: 使用 task_create 追加任务；任务会以 `### T-XXX 标题 <!-- lrnev-task: ... -->` 形式追加到这里 -->

## 验收标准（整体）

- <!-- FILL: 按本 Spec 调整整体验收清单 -->
- [ ] 所有任务完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### T-001 git 封装 git.rs（fetch/rev-parse/pull/log） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.733Z, updated=2026-08-13T16:43:24.944Z, validates=F-01|D-02 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T16:43:24.836Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T16:43:24.944Z"}] -->

git 命令封装，参数数组无 shell 注入，超时处理

**验收**：
- fetch/对比/拉取命令可用

### T-002 watcher 轮询循环 + SHA 去重（state.rs） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.733Z, updated=2026-08-13T16:43:25.142Z, depends_on=T-001, validates=F-01|F-05|D-01|D-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T16:43:25.043Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T16:43:25.142Z"}] -->

定时 fetch 对比 SHA，state.json 持久化去重，单仓库失败隔离

**验收**：
- 新提交可检测
- 同 SHA 不重复通知

**依赖**：T-001

### T-003 自动拉取策略（off/ff-only/merge/reset） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.733Z, updated=2026-08-13T17:14:47.180Z, depends_on=T-002, validates=F-02 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:47.050Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:47.180Z"}] -->

按 auto_pull 执行拉取，脏工作区保护，失败通知

**验收**：
- ff-only 自动快进
- 脏工作区不破坏现场

**依赖**：T-002

### T-004 重建分发 rebuild.rs（off/command） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.733Z, updated=2026-08-13T17:14:47.397Z, depends_on=T-003, validates=F-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:47.290Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:47.397Z"}] -->

rebuild.mode 分发，command 执行并捕获输出

**验收**：
- command 模式执行并上报结果

**依赖**：T-003

### T-005 通知集成 + 默认配置指向用户 fork 库 <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.733Z, updated=2026-08-13T17:14:47.612Z, depends_on=T-003, validates=F-04|F-05 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:47.506Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:47.612Z"}] -->

三类通知接入 notify，config.example.json 默认含 t479842598/deepseek-harness 示例

**验收**：
- 默认配置含 fork 库示例
- 三类通知可用

**依赖**：T-003

### T-006 手动触发 poll_now + watcher 状态 command <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.733Z, updated=2026-08-13T17:14:47.827Z, depends_on=T-002, validates=F-01 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:47.722Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:47.827Z"}] -->

托盘立即检查更新 + 状态查询 command

**验收**：
- 手动触发一轮检查

**依赖**：T-002
