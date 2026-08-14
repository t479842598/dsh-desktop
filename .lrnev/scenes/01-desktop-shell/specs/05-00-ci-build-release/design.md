---
spec: '05-00-ci-build-release'
scene: '01-desktop-shell'
---

# 05-00 Ci Build Release - 设计

## 决策

### D-01 workflow 结构：双平台矩阵构建 + tag 直接发 Release

- 构建 job 用 matrix（macos-14 arm64 原生构建 aarch64-apple-darwin；windows-latest 构建 x86_64-pc-windows-msvc），`fail-fast: false`
- **tag 发布不走 artifact 中转**：每个平台 job 构建完成后直接 `softprops/action-gh-release@v2` 把自己的安装包挂到 GitHub Release（Release assets 不计入 Actions artifact 配额）
- `workflow_dispatch` 手动触发时才走 `upload-artifact`（此时无 tag、无 Release）
- 理由：免费账号 Actions artifact 存储配额仅 500MB，账号下其他仓库的旧构建产物会把它占满（见 errorbook 65abb3d91c65）；Release assets 无此配额问题

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
