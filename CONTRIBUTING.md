# 贡献指南（CONTRIBUTING）

欢迎为 **DSH desktop**（`dsh-desktop`）贡献代码。本项目使用
[lrnev](https://github.com/cordiverse/lrnev) 治理（`/.lrnev/`），请先阅读
项目内 `AGENTS.md` 与 `~/.pi/agent/AGENTS.md` 的全局规范。

## 工作流

### 1. 开发前

```sh
pnpm install
pnpm tauri dev
```

- 前端（`src/`）改动即时热更新；Rust（`src-tauri/`）改动由 tauri dev 自动重编译。
- 首次运行生成 `~/.dsh-desktop/config.json`，日志在 `~/.dsh-desktop/logs/`。

### 2. 开发中（lrnev 治理）

- 改动代码前，先 `project_status` 接手现状；新特性按「能写 WHEN…THEN 验收且可独立交付」
  才开 spec，小改动直接做。
- 完成一个任务后更新对应 `task` 状态；技术决策记 `adr`，踩坑记 `error`。

### 3. 提交流程

```sh
git add -A
git commit -m "<type>(<scope>): <subject>"
```

提交信息类型：`feat` / `fix` / `docs` / `ci` / `refactor` / `chore` / `test`。

### 4. 发布流程

发布由 **tag 触发 CI**（`.github/workflows/build.yml`）：

1. **更新 `CHANGELOG.md`**（Keep a Changelog 格式），补 `## [x.y.z] - 日期` 段。
2. **更新版本号**：`src-tauri/tauri.conf.json` 的 `version`。
3. **提交并推送**到 `main`/`master`。
4. **打 tag**（tag 注释里写明依赖说明）：

   ```sh
   git tag -a v0.1.0 -m "dsh-desktop v0.1.0

   依赖：使用本机已安装的 DeepSeek harness 源码（不自带 dsh 运行时），
   启动时从配置的 dsh.cwd 拉起本地 harness。"
   git push origin v0.1.0
   ```

5. CI 自动构建 **macOS arm64（.dmg）+ Windows x64（.msi / .exe）** 并挂到 GitHub Release。
6. 到 GitHub Releases 页确认 assets、补 Release 说明（特性/截图），标记正式版。

### 手动构建（本地）

```sh
pnpm tauri build              # 当前平台
pnpm tauri build --target x86_64-pc-windows-msvc   # 交叉编译（需 Windows 工具链/llvm-rc）
```

## 关键架构约定

- **不自带 dsh 运行时**：壳通过 `dsh.command` + `dsh.cwd` 拉起**本机已安装的
  DeepSeek harness 源码**（`pnpm dsh web` 模式）。改 harness 代码后需同步/重装
  主卷副本（见 README「依赖」）。
- **单窗口**：主窗口加载 dsh 页面，壳 UI 由 `src-tauri/src/shell.rs` 的注入脚本叠加。
- **ACL 约束**：注入脚本运行在远程 origin（3080/远程 URL），自定义 command 会被
  Tauri ACL 拒绝——窗口控制走 `core:window` 内置命令，退出/保存/右键/通知走事件桥接
  （Rust `app.listen` + 前端 `plugin:event|emit`）。新增交互时遵循此模式，不要新加
  远程页面直接调用的自定义 command。
