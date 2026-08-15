---
spec: '07-00-desktop-polish'
scene: '01-desktop-shell'
created: '2026-08-15'
---

# 07-00 Desktop Polish - 设计

## L0 摘要

移植 fork（luolangaga/deepseek-harness apps/desktop）桌面体验到 Tauri 2 壳：原生右键菜单（Rust Menu + popup_menu，动作 Rust eval 执行）、无边框 Mica/vibrancy 窗口、前端 splash、window-state 状态记忆。

## L1 概览

### 架构思路

- **右键菜单**：Tauri 无 Electron `webContents.on('context-menu')` 事件，改为前端 `contextmenu` 收集目标信息 → `invoke show_context_menu` → Rust 构建原生 `Menu` 并 `popup_menu_at`；菜单点击经 app 级 `on_menu_event`（ctx- 前缀）分发，Rust 用 `win.eval()` 执行 DOM 动作（execCommand/clipboard/history）。外部 dsh Web UI 窗口（3080/远程页面）通过 `initialization_script` 注入同样的收集桥接。
- **无边框窗口**：`tauri.conf.json` 设 `decorations:false` + `titleBarStyle:"Overlay"`；setup 里 Win 调 `apply_mica`、macOS 调 `apply_vibrancy(HudWindow)`；topbar 加 `data-tauri-drag-region` 拖拽。
- **splash**：前端固定定位覆盖层（wordmark + 光带渐变扫过），`dsh-ready` 事件或远程模式状态刷新后淡出移除。
- **窗口状态**：`tauri-plugin-window-state` 插件默认行为（关闭保存、启动恢复）。

### 主要模块

| 模块 | 职责 |
|---|---|
| `lib.rs` | `show_context_menu` command、`handle_ctx_menu_action`（eval 分发）、`CTX_MENU_BRIDGE_SCRIPT` 注入、setup 窗口效果 |
| `tauri.conf.json` | 无边框 + Overlay 配置 |
| `Cargo.toml` | `window-vibrancy`、`tauri-plugin-window-state` |
| 前端 | `setupContextMenu` 收集桥接、splash DOM/动画、topbar 拖拽区 |

### 关键决策

| 决策 | 选项 | 倾向 | 是否产 ADR |
|---|---|---|---|
| 菜单动作执行 | emit 事件回前端 / Rust eval | Rust eval（外部窗口无前端监听，eval 统一） | 否 |
| 菜单触发 | 前端事件收集 / 无 | 前端收集 + 注入脚本双通道（覆盖壳窗口与外部 dsh 窗口） | 否 |
| 窗口效果 | 全平台 / Win+macOS | Win+macOS（Linux 不做特效） | 否 |

## L2 详情

### 模块详细设计

- `show_context_menu`：按目标类型构建菜单（可编辑/选区/空白/链接/图片），目标信息存入 `AppState.ctx_menu_window/target`，`popup_menu_at` 弹出
- `handle_ctx_menu_action`：从 state 读窗口与目标，按 action 生成 eval JS；`ctx-open-link` 用 `tauri_plugin_opener` 走系统浏览器
- `CTX_MENU_BRIDGE_SCRIPT`：IIFE 防重入，监听 contextmenu 收集 isEditable/selection/link/img，`window.__TAURI_INTERNALS__.invoke` 调 Rust
- splash 隐藏时机：`dsh-ready` 事件、`refreshStatus` 检测到 ready 或远程模式

### 测试要点

- 五类右键场景均弹原生菜单（Rust 单测构建逻辑）
- 无边框窗口可拖动、Mica/vibrancy 生效
- splash 出现且 ready 后消失
- 窗口大小位置重启后恢复
