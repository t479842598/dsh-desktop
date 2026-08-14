---
spec: '05-00-ci-build-release'
scene: '01-desktop-shell'
---

# 05-00 Ci Build Release - 设计

## 决策

### D-01 workflow 结构：单 workflow 双 job（构建矩阵 + 发布）

- 构建 job 用 matrix（macos-14 arm64 原生构建 aarch64-apple-darwin；windows-latest 构建 x86_64-pc-windows-msvc），`fail-fast: false`，各自 upload-artifact
- 发布 job 仅在 `push tags: v*` 时运行，download 两个 artifact 后交给 `softprops/action-gh-release@v2` 挂 assets
- 理由：显式 tauri build 步骤比 tauri-action 更透明可控；artifact→release 两步走，手动触发时也能只出 artifact

### D-02 产物路径必须带 target triple

`pnpm tauri build --target <triple>` 时 bundle 落在 `src-tauri/target/<triple>/release/bundle/`，
而不是 `target/release/bundle/`。原草案路径会 `if-no-files-found: error` 必挂。

### D-03 缓存策略

- pnpm store：`pnpm/action-setup@v4` + `actions/setup-node@v4 (cache: pnpm)`，key 含 lockfile hash
- Rust：`Swatinem/rust-cache@v2`（自动按 target 分目录，天然隔离两个平台）
- 前端 dist 不经缓存（vite 构建 < 1s）

### D-04 版本与触发

- 版本号保持 package.json/tauri.conf.json 的 0.1.0；Release 名/标签由 tag 决定
- 触发：`push tags v*` + `workflow_dispatch`（不监听普通 push，避免每次提交都烧 CI 配额）

## 边界

- 不签名不公证：macOS 产物为未签名 .dmg/.app（本机可运行，Gatekeeper 需右键打开）；
  Windows 为未签名 .msi/.exe（SmartScreen 提示属预期）
- `permissions: contents: write` 只给发布 job；构建 job 保持 read
