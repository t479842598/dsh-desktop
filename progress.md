## 2026-08-31 - Task: 依赖更新检查与升级（dsh-desktop / dsh harness）

### What was done
- 检查并确认 dsh-desktop 客户端本身无新版本（本地 0.6.2 与 origin/master 同步，已装 App 亦为 0.6.2）。
- 前端依赖升级：vite 6.4.3 → 8.2.2、typescript 5.9.3 → 7.0.2，均通过 `pnpm build` 与 `tsc --noEmit` 验证。
- Rust 依赖升级：window-vibrancy 0.6.0 → 0.8.0、tauri-plugin-opener 2.5.4 → 2.5.5、objc2 0.6.4 / objc2-foundation 0.3.2（锁定），通过 `cargo check` 与 `cargo test`（10 用例全过）。
- dsh harness 升级尝试：全局 @deepseek-ai/dsh 0.1.1-rc.2 → 0.1.2-alpha.2。冒烟测试发现 alpha.2 与已装第三方插件（@linxin666/dsh-client-ui-*、dsh-pet 等）不兼容（dsh-settings 导出项移除），插件树加载失败，已回滚至 0.1.1-rc.2 并保留 /tmp 回滚源。运行中服务（3080）全程未受影响。

### Testing
- `pnpm build`：vite v8.2.2 构建成功，8 modules transformed。
- `npx tsc --noEmit`：通过（typescript 7.0.2）。
- `cargo check` / `cargo test`：编译通过，10/10 测试通过。
- dsh 冒烟测试：`dsh web --port 3099 --no-open` 启动，插件树加载失败（alpha.2 不兼容），据此回滚；回滚后 `dsh --version` = 0.1.1-rc.2，3080 服务 HTTP 200 正常。
- CLI 参数兼容性：0.1.2-alpha.2 的 `--port / --no-open / --trusted-host` 仍在帮助中，参数面本身无变化。

### Notes
改动文件：
- package.json：typescript ^7.0.2、vite ^8.2.2
- pnpm-lock.yaml：随上述升级更新
- src-tauri/Cargo.toml：window-vibrancy "0.8"
- src-tauri/Cargo.lock：crate 版本更新

回滚方式：
- 前端：`git checkout -- package.json pnpm-lock.yaml && pnpm install`
- Rust：`git checkout -- src-tauri/Cargo.toml src-tauri/Cargo.lock`
- dsh（如需再试 alpha.2）：先升级配套插件（如 @linxin666/dsh-web-all 0.3.6 → 0.3.10）再 `npm i -g @deepseek-ai/dsh@alpha`；回滚源已存 /tmp/deepseek-ai-dsh-0.1.1-rc.2.tgz，回滚命令 `npm i -g /tmp/deepseek-ai-dsh-0.1.1-rc.2.tgz`。

## 2026-08-31 - Task: dsh harness 升级 alpha.2 + 客户端提交打包替换

### What was done
- dsh harness 0.1.1-rc.2 → 0.1.2-alpha.2 升级完成：
  - profile 插件升级：@linxin666/dsh-web-all 0.3.6→0.3.10（含全部 @linxin666/* 子插件）、dsh-context 0.34.0→0.38.5、dshmarket 1.34.0→1.38.1、@michengai/dsh-agency-agents 0.1.21→0.1.23。
  - 移除 dsh-neu-theme（0.1.1 停更、与 alpha.2 不兼容，是唯一无法适配的插件）。
  - 重启 3080 服务（旧 PID 861 退出后由壳自动拉起 PID 9791 新版），服务就绪、认证正常（HTTP 401）。
- 客户端：前端依赖（vite 8.2.2 / typescript 7.0.2）+ Rust 依赖（window-vibrancy 0.8.0 / tauri-plugin-opener 2.5.5）变更已提交（commit f276616）。
- 打包：`pnpm tauri build` 产出 dmg + app bundle（0.6.2），校验哈希一致后替换 /Applications/dsh-desktop.app，旧版移入回收站。

### Testing
- dsh 冒烟：`dsh web --port 3088` 插件树加载通过（无 settingsNamespace 错误）；3080 实机服务 pipeline.ready + server.started，HTTP 401 认证正常。
- 前端：pnpm build（vite 8）✓、tsc --noEmit ✓。
- Rust：cargo test 10/10 ✓。
- App 校验：/Applications 与 bundle 二进制 shasum 一致。

### Notes
- 改动文件（仓库内）：package.json、pnpm-lock.yaml、src-tauri/Cargo.toml、src-tauri/Cargo.lock（commit f276616）。
- 改动文件（仓库外，~/.dsh/profiles/web/）：package.json、pnpm-lock.yaml（dsh-web-all 等升级、neu-theme 移除），备份 package.json.bak.20260831-215921。
- 副作用：dsh-neu-theme 主题插件被移除（无兼容版，若作者更新可重新安装）。
- 回滚方式：
  - 客户端：`git checkout f276616~1 -- package.json pnpm-lock.yaml src-tauri/Cargo.toml src-tauri/Cargo.lock && pnpm install`；App 备份在 /tmp/dsh-desktop.app.bak.20260831、旧版在 ~/.Trash。
  - dsh：`npm i -g /tmp/deepseek-ai-dsh-0.1.1-rc.2.tgz`，profile 恢复 `cp package.json.bak.20260831-215921 package.json && pnpm install`。

## 2026-08-31 - Task: 修复客户端白屏/加载中（dsh 0.1.2-alpha launch token 认证适配）

### What was done
- 现象：升级 dsh 到 0.1.2-alpha.2 后客户端打开白屏/一直加载中。
- 根因：dsh 0.1.2-alpha 起为 web UI 启用了 launch-token 认证（进程级随机 token，打印在 stdout 的 `dsh web: http://127.0.0.1:3080/?token=...`），桌面壳导航到裸 `http://127.0.0.1:3080` 未带 token → HTTP 401 → 页面加载失败。
- 修复（commit 6941502）：
  - dsh.rs 新增 `launch_token_url()`：从 `~/.dsh-desktop/logs/dsh.log`（壳自拉）或 `~/.dsh/harness.out.log`（launchd 常驻）解析最近一次 `dsh web:` 行中的 URL（带 token 或裸 URL 均兼容）。
  - lib.rs 新增 `local_web_url()` 并替换 4 处本地导航硬编码，优先用带 token URL，回退裸 URL（兼容旧版）。
- 重新打包并替换 /Applications/dsh-desktop.app（哈希一致，旧版移回收站）。

### Testing
- cargo test 12/12 通过（新增 launch_token_url 两个测试：带 token 解析、无匹配返回 None）。
- 实测：当前 harness.out.log 最后一条 token URL 访问 3080 返回 HTTP 303（认证通过），修复逻辑与真实环境匹配。

### Notes
- 改动文件：src-tauri/src/dsh.rs（launch_token_url + 测试）、src-tauri/src/lib.rs（local_web_url + 4 处导航替换）。
- 回滚：`git checkout 6941502~1 -- src-tauri/src/dsh.rs src-tauri/src/lib.rs` 后重新打包；App 旧版在回收站。

## 2026-08-31 - Task: 白屏根因定位（dsh alpha.2 认证与自定义密码门栈冲突）

### What was done
- 追加排查：白屏非桌面壳单一问题。已停用 shell 界面的 launchd 服务删除恢复（com.qingtang.dsh-harness 用 bak 重建，端口改回 3080）并重新加载运行。
- 实验性禁用过 remote-web-ui / skin-center（均非根因，已恢复）。
- 确认：dsh 0.1.2-alpha.2 新增**官方 launch-token 认证层**（硬编码、无配置开关），与用户自定义 ds-auths-plugin 密码门叠加冲突。

### 诊断结论
- 无 token 访问 `/` → 官方层 401（"dsh web authentication required"）。
- 带 token 访问 → 303 种官方 cookie，官方层通过。
- 带官方 cookie 访问 `/` → **HTTP 500 `AUTH_INTERNAL_ERROR`**（ds-auths-plugin 的 fallback 抛未识别异常）。
- 升级前 rc.2 无官方 token 层，ds-auths-plugin loopback 放行（principal null）→ 正常显示。
- 即：**alpha.2 认证机制与用户 ds-auths-plugin + gateway 自动登录栈不兼容**，非桌面壳可单独修复（涉及官方包与用户插件交互）。

### 桌面壳已完成的修复（均已验证，对 rc.2/alpha.2 双兼容）
- `launch_token_url()`：从 dsh 启动日志解析 token URL 导航（commit 6941502）。
- `healthy_dsh` 认 401：避免误杀 launchd 常驻服务造成 EADDRINUSE 循环（未提交）。

### Testing
- cargo test 13/13 通过。
- 复现链路：curl 带 token → 303；带官方 cookie 访问 `/` → 500（ds-auths-plugin）。

### Notes
- 待用户决策：回滚 dsh 到 rc.2（用户稳定基线，立即可用）vs 继续 alpha.2（需适配/替换 ds-auths-plugin 自定义认证栈）。
- 改动文件：src-tauri/src/dsh.rs、src-tauri/src/lib.rs（客户端，已提交 6941502 + 未提交 healthy 修复）；~/.dsh/profiles/web/cordis.patch.yml（实验，已恢复）。
- 回滚源：dsh /tmp/deepseek-ai-dsh-0.1.1-rc.2.tgz；LaunchAgents 已重建并加载。

## 2026-09-01 - Task: 回滚 dsh 到 rc.2（方案 A 落地）

### What was done
- 按用户决策（方案 A），将 dsh 从 0.1.2-alpha.2 回滚到 0.1.1-rc.2：
  - 全局包：npm 回滚到 /tmp/deepseek-ai-dsh-0.1.1-rc.2.tgz。
  - 插件：恢复 rc.2 基线（dsh-web-all ^0.3.6、dsh-context ^0.34.0、dshmarket ^1.34.0、agency-agents ^0.1.21），并恢复 dsh-neu-theme ^0.1.1。
- 重启 launchd 服务（com.qingtang.dsh-harness），rc.2 服务正常监听 3080。
- 客户端保留两处兼容修复并提交：token URL 解析（6941502）、healthy_dsh 认 401（aae4fa0，对 rc.2 无害）。
- 重新打包并替换 /Applications/dsh-desktop.app（哈希一致）。

### Testing
- `dsh --version` = 0.1.1-rc.2。
- 不带 token 访问 http://127.0.0.1:3080/ → HTTP 200 + 完整 HTML（21891 字节），不再 401/500。
- cargo test 13/13 通过。
- 服务进程稳定（90274，无反复重启）。
- App 替换哈希校验一致。

### Notes
- 结论：rc.2 下 dsh 首页、认证栈（ds-auths-plugin）、gateway 均正常，客户端白屏问题已解决。
- 改动文件：src-tauri/src/dsh.rs（two commits 已提交）；~/.dsh/profiles/web/package.json、pnpm-lock.yaml（恢复 rc.2 基线，alpha.2 备份在 package.json.alpha2.bak.20260831-235102）。
- 回滚点：alpha.2 全局包可 `npm i -g @deepseek-ai/dsh@alpha` 恢复；插件可回滚到 alpha2 备份；客户端 commits 可 revert。
