---
spec: '04-00-remote-access'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 04-00 Remote Access - 任务清单

> 任务由 lrnev `task_create` 工具创建，不要手编。
> 状态机：pending → in_progress → completed / failed；blocked 可回 in_progress；failed 可回 pending 重试。

## 阶段 1

<!-- FILL: 使用 task_create 追加任务；任务会以 `### T-XXX 标题 <!-- lrnev-task: ... -->` 形式追加到这里 -->

## 验收标准（整体）

- <!-- FILL: 按本 Spec 调整整体验收清单 -->
- [ ] 所有任务完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### T-001 认证网关 remote.rs（Basic Auth + Host 校验 + 字节流转发） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.810Z, updated=2026-08-13T16:43:25.673Z, validates=F-01|F-02|D-01 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T16:43:25.592Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T16:43:25.673Z"}] -->

参考 pi-web web-auth.ts：sha256+timingSafeEqual 双比较，allowed_hosts 校验，TCP 双向 pipe 到 127.0.0.1:3080

**验收**：
- 错误凭据 401
- 错误 Host 403
- 正确凭据可访问 dsh UI

### T-002 remote 配置段 + 生命周期绑定 <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.810Z, updated=2026-08-13T16:43:25.841Z, depends_on=T-001, validates=F-03|F-04 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T16:43:25.757Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T16:43:25.841Z"}] -->

config.rs remote 段，网关随壳启停，dsh 重启后网关恢复，端口占用通知

**验收**：
- 默认未启用
- dsh 重启后网关恢复

**依赖**：T-001

### T-003 设置页（用户名/密码/可信主机/端口/保存重启） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:48.810Z, updated=2026-08-13T17:14:48.144Z, depends_on=T-002, validates=F-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:48.038Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:48.144Z"}] -->

前端设置表单，保存后写配置并重启后端

**验收**：
- 保存后重启且新凭据生效

**依赖**：T-002
