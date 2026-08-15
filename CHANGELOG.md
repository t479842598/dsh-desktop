# Changelog

本项目的所有显著变更都记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [0.3.0] - 2026-08-15

### 变更

- **发布产物调整**：macOS 改为打包 `.app` 并压缩为 zip 发布（CI runner 上
  create-dmg 不稳定）；Windows 保持 NSIS 安装包（exe）+ MSI 双格式，
  详见 README「下载与安装」。

## [0.2.0] - 2026-08-15

### 新增

- **单窗口桌面壳**：主窗口直接加载 dsh Web UI（本地 `127.0.0.1:3080` 或远程 URL），
  壳层 UI（灵动岛工具条、设置面板、右键菜单、启动画面）通过注入脚本叠加在页面上，
  无需独立的控制台窗口。
- **连接模式切换**：设置面板内可切换「本地模式 / 远程模式」，远程模式支持配置
  远程 URL + Basic Auth 账号密码；保存后自动热切换目标并重启/停止本地 dsh。
- **灵动岛工具条**：悬浮于窗口顶部 70% 位置，平时隐藏、鼠标靠近自动浮现；
  内含品牌、模式标签、「设置」入口与窗口控制（缩小 / 放大 / 关闭）；
  长按灵动岛可拖动窗口。
- **原生右键菜单**：输入框（剪切/复制/粘贴/全选）、选区、空白面（返回/前进/刷新）、
  链接（复制链接/浏览器打开）、图片（复制/另存为）五类场景弹出系统原生菜单。
- **桌面系统通知**：`dsh-notification` 插件在 Tauri WebView 环境下自动改走系统通知
  （macOS `osascript` / Windows PowerShell），任务完成时弹出系统通知。
- **无边框窗口效果**：Windows 11 Mica / macOS vibrancy 毛玻璃，标题栏按钮跟随系统主题。
- **窗口状态记忆**：窗口大小/位置跨启动恢复。

### 变更

- 依赖说明：本壳**不自带 dsh 运行时**，启动时调用**本机已安装的 DeepSeek harness 源码**
  （见 README「依赖」一节），因此安装包体积小且与本地 harness 版本保持同步。
- 结构：`src-tauri/src/shell.rs` 新增注入脚本（灵动岛/设置/右键/事件桥接）；
  `src-tauri/src/dsh.rs` 启动子进程改为跨平台（Unix `/bin/sh`、Windows `cmd /C`）。

### 修复

- 顶部热区不再拦截整行点击（改为 mousemove 坐标检测 + 岛自身 `pointer-events` 控制）。
- 切换连接模式后主窗口自动导航到正确目标，模式标签同步刷新。
- 远程 origin 下自定义 command 被 Tauri ACL 拒绝的问题：窗口控制改走
  `core:window` 内置命令，退出/保存/右键/通知改走事件桥接。
- macOS 无边框窗口红黄绿控制按钮的显示与定位。

### 已知限制

- 远程模式的 Basic Auth 经 URL userinfo 传递：macOS WKWebView 支持；
  Windows WebView2 可能弹出系统认证框（平台行为差异，待适配）。
- Windows 版本已通过源码层跨平台改造，完整构建与实机验证走 CI 双平台流水线。

## [0.1.0] - 2026-08-14

### 新增

- 初始版本：Tauri 2 桌面壳，自动拉起本地 dsh Web 服务，托盘常驻 + 自动更新管线。
