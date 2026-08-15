---
spec: '07-00-desktop-polish'
scene: '01-desktop-shell'
created: '2026-08-15'
---

# 07-00 Desktop Polish - 任务清单

> 任务由 lrnev `task_create` 工具创建，不要手编。
> 状态机：pending → in_progress → completed / failed；blocked 可回 in_progress；failed 可回 pending 重试。

## 阶段 1

<!-- FILL: 使用 task_create 追加任务；任务会以 `### T-XXX 标题 <!-- lrnev-task: ... -->` 形式追加到这里 -->

## 验收标准（整体）

- <!-- FILL: 按本 Spec 调整整体验收清单 -->
- [ ] 所有任务完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### T-001 Rust: show_context_menu command（五类菜单构建 + popup_menu） <!-- lrnev-task: status=completed, created=2026-08-14T17:34:03.994Z, updated=2026-08-14T17:48:31.742Z, validates=F-01 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T17:38:29.739Z"},{"from":"in_progress","to":"completed","at":"2026-08-14T17:48:31.742Z"}] -->

### T-002 前端: contextmenu 事件桥接（收集目标信息 invoke Rust） <!-- lrnev-task: status=completed, created=2026-08-14T17:34:03.994Z, updated=2026-08-14T17:48:31.851Z, depends_on=T-001, validates=F-01 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T17:48:31.796Z"},{"from":"in_progress","to":"completed","at":"2026-08-14T17:48:31.851Z"}] -->

**依赖**：T-001

### T-003 无边框 + Mica/vibrancy（tauri.conf + window-vibrancy + setup） <!-- lrnev-task: status=completed, created=2026-08-14T17:34:03.994Z, updated=2026-08-14T17:50:34.828Z, validates=F-02 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T17:48:31.908Z"},{"from":"in_progress","to":"completed","at":"2026-08-14T17:50:34.828Z"}] -->

### T-004 前端 splash 覆盖层（wordmark 光带 + dsh-ready 淡出） <!-- lrnev-task: status=completed, created=2026-08-14T17:34:03.994Z, updated=2026-08-14T17:53:23.320Z, validates=F-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T17:50:34.888Z"},{"from":"in_progress","to":"completed","at":"2026-08-14T17:53:23.320Z"}] -->

### T-005 tauri-plugin-window-state 窗口状态记忆 <!-- lrnev-task: status=completed, created=2026-08-14T17:34:03.994Z, updated=2026-08-14T17:53:23.375Z, validates=F-04 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T17:50:34.944Z"},{"from":"in_progress","to":"completed","at":"2026-08-14T17:53:23.375Z"}] -->
