---
spec: '03-00-model-rebuild'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 03-00 Model Rebuild - 任务清单

> 任务由 lrnev `task_create` 工具创建，不要手编。
> 状态机：pending → in_progress → completed / failed；blocked 可回 in_progress；failed 可回 pending 重试。

## 阶段 1

<!-- FILL: 使用 task_create 追加任务；任务会以 `### T-XXX 标题 <!-- lrnev-task: ... -->` 形式追加到这里 -->

## 验收标准（整体）

- <!-- FILL: 按本 Spec 调整整体验收清单 -->
- [ ] 所有任务完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### T-001 dsh headless 执行器（spawn/降级/重试） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:17.006Z, updated=2026-08-13T16:43:25.339Z, validates=F-01|F-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T16:43:25.241Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T16:43:25.339Z"}] -->

spawn dsh --profile headless，env 透传，无 key 降级 command，失败重试一次

**验收**：
- headless 调用成功
- 无 key 降级不崩溃

### T-002 Prompt 模板渲染 + 结果通知 <!-- lrnev-task: status=completed, created=2026-08-13T16:33:17.006Z, updated=2026-08-13T16:43:25.510Z, depends_on=T-001, validates=F-02|F-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T16:43:25.425Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T16:43:25.510Z"}] -->

模板含仓库/分支/更新摘要，结果走 notify

**验收**：
- 模板渲染正确
- 结果通知成功/失败

**依赖**：T-001
