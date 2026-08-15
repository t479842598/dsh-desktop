---
spec: 07-00-desktop-polish
scene: 01-desktop-shell
status: completed
priority: P1
created: '2026-08-15'
updated: '2026-08-15'
---

# 07-00 Desktop Polish - 需求

## L0 摘要

移植 fork（luolangaga/deepseek-harness apps/desktop）的桌面体验功能到 Tauri 2 壳：原生右键菜单、Mica/毛玻璃无边框窗口、启动画面、窗口状态记忆。

## L1 概览

### 目标

- 原生右键菜单：可编辑区/选区/空白面/链接/图片五类场景弹出系统原生菜单（对齐 fork showContextMenu 行为）
- 无边框窗口 + Mica（Win11）/ vibrancy（macOS），标题栏按钮跟随系统主题
- 启动画面：wordmark + 光带扫过动画，host ready 后淡出
- 窗口大小/位置记忆，全屏退出恢复圆角

### 用户故事

- 作为用户，我希望右键能弹原生菜单（剪切/复制/粘贴），不用键盘快捷键
- 作为用户，我希望窗口有现代毛玻璃质感且记住上次位置

### 范围

**包含**：
- `tauri::menu::Menu` + `popup_menu` 原生右键菜单（前端 contextmenu 事件桥接）
- `window-vibrancy` crate：Win11 `apply_mica`、macOS `apply_vibrancy`
- `decorations:false` + `titleBarStyle:"Overlay"` 无边框配置
- 前端 splash 覆盖层（CSS/SVG 动画），`dsh-ready` 事件后淡出
- `tauri-plugin-window-state` 窗口状态记忆

**不包含**：
- 自定义标题栏按钮（Overlay 用系统控件，不做自定义 caption buttons）
- Linux 特有窗口效果（仅 Win/macOS 配置）

## L2 详情

### 详细需求

#### F-01 原生右键菜单
- 描述：前端捕获 `contextmenu` 事件，收集目标信息（isEditable/selectionText/linkURL/imageURL），invoke 到 Rust；Rust 按目标类型构建 `tauri::menu::Menu` 并 `popup_menu`
- 验收：WHEN 在输入框右键 THEN 弹剪切/复制/粘贴/全选；WHEN 选中文字右键 THEN 弹复制/全选；WHEN 空白处右键 THEN 返回/前进/刷新；WHEN 链接右键 THEN 复制链接/浏览器打开；WHEN 图片右键 THEN 复制图片/另存为

#### F-02 Mica 无边框窗口
- 描述：`tauri.conf.json` 设 `decorations:false`、`titleBarStyle:"Overlay"`；setup 时 Win 调 `apply_mica`、macOS 调 `apply_vibrancy`
- 验收：WHEN 启动 THEN 窗口无系统边框且 Win11 有 Mica 材质、macOS 有 vibrancy；标题栏控件可见

#### F-03 启动画面
- 描述：主窗口加载 splash 覆盖层（wordmark + 光带动画）；收到 `dsh-ready` 事件后淡出显示 UI
- 验收：WHEN 启动 THEN splash 出现；WHEN dsh ready THEN splash 淡出

#### F-04 窗口状态记忆
- 描述：集成 `tauri-plugin-window-state`，记住宽高/位置/最大化；全屏退出恢复圆角
- 验收：WHEN 调整窗口大小/位置并重启 THEN 恢复上次状态

## 验收标准

- [x] 五类右键菜单场景均弹原生菜单且动作正确
- [x] 无边框 + Mica/vibrancy 生效
- [x] splash 出现并在就绪后消失
- [x] 窗口状态记忆生效
