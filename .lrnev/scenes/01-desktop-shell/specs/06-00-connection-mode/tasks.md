---
spec: '06-00-connection-mode'
scene: '01-desktop-shell'
created: '2026-08-15'
---

# 06-00 Connection Mode - 任务清单

> 任务由 lrnev `task_create` 工具创建，不要手编。
> 状态机：pending → in_progress → completed / failed；blocked 可回 in_progress；failed 可回 pending 重试。

## 阶段 1

<!-- FILL: 使用 task_create 追加任务；任务会以 `### T-XXX 标题 <!-- lrnev-task: ... -->` 形式追加到这里 -->

## 验收标准（整体）

- <!-- FILL: 按本 Spec 调整整体验收清单 -->
- [ ] 所有任务完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### T-001 config.rs: connection 段（mode/remote url+账号密码） <!-- lrnev-task: status=completed, created=2026-08-14T17:34:03.939Z, updated=2026-08-14T17:36:54.401Z, validates=F-01 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T17:34:34.942Z"},{"from":"in_progress","to":"completed","at":"2026-08-14T17:36:54.401Z"}] -->

### T-002 lib.rs: save_connection command + 打开 UI 按模式路由（含远程凭证） <!-- lrnev-task: status=completed, created=2026-08-14T17:34:03.939Z, updated=2026-08-14T17:37:23.819Z, depends_on=T-001, validates=F-03|F-04 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T17:37:23.763Z"},{"from":"in_progress","to":"completed","at":"2026-08-14T17:37:23.819Z"}] -->

**依赖**：T-001

### T-003 前端: 设置 tab（模式单选/远程表单/保存）+ 状态页显示模式 <!-- lrnev-task: status=completed, created=2026-08-14T17:34:03.939Z, updated=2026-08-14T17:38:29.657Z, depends_on=T-002, validates=F-02|F-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T17:36:54.510Z"},{"from":"in_progress","to":"completed","at":"2026-08-14T17:38:29.657Z"}] -->

**依赖**：T-002
