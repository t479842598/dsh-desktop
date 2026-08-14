---
spec: '01-00-tauri-shell'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 01-00 Tauri Shell - 任务清单

> 任务由 lrnev `task_create` 工具创建，不要手编。
> 状态机：pending → in_progress → completed / failed；blocked 可回 in_progress；failed 可回 pending 重试。

## 阶段 1

<!-- FILL: 使用 task_create 追加任务；任务会以 `### T-XXX 标题 <!-- lrnev-task: ... -->` 形式追加到这里 -->

## 验收标准（整体）

- <!-- FILL: 按本 Spec 调整整体验收清单 -->
- [ ] 所有任务完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### T-001 Tauri 2 工程骨架（Cargo.toml/tauri.conf.json/前端 Vite） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:16.885Z, updated=2026-08-13T16:41:13.629Z, validates=F-01 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T16:34:11.542Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T16:41:13.629Z"}] -->

建立 dsh-desktop 工程：Rust 壳 + Vite 前端，pnpm tauri dev 可启动空窗口

**验收**：
- pnpm tauri dev 启动出现窗口
- cargo check 无错误

### T-002 配置模块 config.rs（默认值/读写/示例文件） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:16.885Z, updated=2026-08-13T16:43:15.584Z, validates=F-05 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T16:41:13.726Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T16:43:15.584Z"}] -->

~/.dsh-desktop/config.json 读写，含 dsh/notify/repos/remote 段，首次启动写默认，提供 config.example.json

**验收**：
- 首次启动生成默认配置
- 修改后重读生效

### T-003 dsh 进程管理 dsh.rs（spawn/就绪探测/kill/重启） <!-- lrnev-task: status=completed, created=2026-08-13T16:33:16.885Z, updated=2026-08-13T17:14:46.147Z, depends_on=T-002, validates=F-02 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:46.031Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:46.147Z"}] -->

启动 dsh web 子进程，轮询端口就绪，退出 kill，支持 restart

**验收**：
- 服务就绪探测成功
- 退出无孤儿进程

**依赖**：T-002

### T-004 主窗口加载 dsh Web UI + 占位页 <!-- lrnev-task: status=completed, created=2026-08-13T16:33:16.885Z, updated=2026-08-13T17:14:46.379Z, depends_on=T-003, validates=F-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:46.265Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:46.379Z"}] -->

就绪后窗口加载 http://127.0.0.1:3080，未就绪显示占位页与重试

**验收**：
- 窗口可加载 dsh UI
- 崩溃后可重试

**依赖**：T-003

### T-005 托盘 + 通知出口 notify.rs <!-- lrnev-task: status=completed, created=2026-08-13T16:33:16.885Z, updated=2026-08-13T17:14:46.623Z, depends_on=T-003, validates=F-04 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:46.495Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:46.623Z"}] -->

托盘菜单（打开/检查更新/配置/退出），osascript 通知

**验收**：
- 托盘菜单可用
- 通知进通知中心

**依赖**：T-003

### T-006 跨平台打包配置 + CI workflow <!-- lrnev-task: status=completed, created=2026-08-13T16:33:16.885Z, updated=2026-08-13T17:14:46.838Z, depends_on=T-001, validates=F-06 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-13T17:14:46.729Z"},{"from":"in_progress","to":"completed","at":"2026-08-13T17:14:46.838Z"}] -->

tauri.conf.json 配 macOS/Windows 目标，GitHub Actions 矩阵 4 目标构建

**验收**：
- 当前平台 tauri build 出包
- CI workflow 覆盖矩阵

**依赖**：T-001
