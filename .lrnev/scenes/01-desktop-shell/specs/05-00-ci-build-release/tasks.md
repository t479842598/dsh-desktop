---
spec: '05-00-ci-build-release'
scene: '01-desktop-shell'
created: '2026-08-14'
---

# 05-00 Ci Build Release - 任务清单

> 任务由 lrnev `task_create` 工具创建，不要手编。
> 状态机：pending → in_progress → completed / failed；blocked 可回 in_progress；failed 可回 pending 重试。

## 阶段 1

<!-- FILL: 使用 task_create 追加任务；任务会以 `### T-XXX 标题 <!-- lrnev-task: ... -->` 形式追加到这里 -->

## 验收标准（整体）

- <!-- FILL: 按本 Spec 调整整体验收清单 -->
- [ ] 所有任务完成
- [ ] 单元测试通过
- [ ] 集成测试通过

### T-001 编写 GitHub Actions 双平台构建 workflow <!-- lrnev-task: status=in_progress, created=2026-08-14T01:14:28.451Z, updated=2026-08-14T01:14:30.724Z, validates=F-01|F-02|F-03 -->
<!-- lrnev-task-history: [{"from":"pending","to":"in_progress","at":"2026-08-14T01:14:30.724Z"}] -->

重写 .github/workflows/build.yml：矩阵仅保留 macos-14(aarch64-apple-darwin) + windows-latest(x86_64-pc-windows-msvc)；修正 bundle 路径为 target/<triple>/release/bundle；加 pnpm store + cargo 缓存；tag v* 时发布 job 挂 Release assets

**验收**：
- workflow 矩阵仅 2 条目
- 产物路径带 target triple
- tag 触发自动发 Release

### T-002 初始化 git 仓库并推送到 GitHub <!-- lrnev-task: status=pending, created=2026-08-14T01:14:28.534Z, validates=F-01 -->

git init、确认 .gitignore 覆盖 node_modules/dist/target/gen/.dsh-desktop、首次提交（含 .lrnev）、gh repo create + push

**验收**：
- 仓库在 GitHub 可见
- 工作区干净

### T-003 从 GitHub Actions 触发构建并验证产物 <!-- lrnev-task: status=pending, created=2026-08-14T01:14:28.620Z, validates=F-02 -->

gh workflow run 触发，等待双平台 job 完成；gh run download 取安装包并核对类型（.dmg/.app、.msi/.exe）

**验收**：
- mac arm64 与 win x64 产物均可下载
- 本地能打开/校验产物
