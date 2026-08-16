mod config;
mod dsh;
mod git;
mod notify;
mod rebuild;
mod shell;
mod state;
mod watcher;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Listener, Manager, State, WebviewWindow};
use tauri::Position;
use tauri::LogicalPosition;

use dsh::DshHandle;
use watcher::{WatcherShared, WatcherStatus};

/// 应用共享状态
pub struct AppState {
    pub dsh: DshHandle,
    pub watcher: Arc<WatcherShared>,
    /// 最近弹出右键菜单的窗口 label（菜单事件回调据此定向 emit）
    pub ctx_menu_window: std::sync::Mutex<Option<String>>,
    /// 最近弹出右键菜单的目标信息（链接/图片 URL 供动作使用）
    pub ctx_menu_target: std::sync::Mutex<Option<ContextMenuTarget>>,
}

#[tauri::command]
fn get_status(_app: AppHandle, state: State<'_, AppState>) -> serde_json::Value {
    let cfg = config::load_config().0;
    let dsh = state.dsh.0.lock().unwrap();
    serde_json::json!({
        "dsh_ready": dsh.ready,
        "dsh_error": dsh.last_error,
        "dsh_port": cfg.dsh.port,
        "poll_interval_sec": cfg.poll_interval_sec,
        "notify_enabled": cfg.notify.enabled,
        "connection_mode": cfg.connection.mode,
        "remote_url": cfg.connection.remote.url,
        "remote_username": cfg.connection.remote.username,
        "repos": cfg.repos.iter().map(|r| serde_json::json!({
            "name": r.name,
            "local_path": r.local_path,
            "remote": r.remote,
            "branch": r.branch,
            "auto_pull": r.auto_pull,
        })).collect::<Vec<_>>(),
    })
}

#[tauri::command]
fn restart_dsh(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // 远程模式没有本地 dsh 子进程，拒绝重启并提示
    if config::load_config().0.connection.mode == "remote" {
        return Err("当前为远程模式，本地 dsh 未启动".into());
    }
    // 异步执行：重启包含 kill + spawn + 就绪探测（最长 30s），不能阻塞命令线程
    let app2 = app.clone();
    let dsh_handle = state.dsh.0.clone();
    tauri::async_runtime::spawn(async move {
        let cfg = config::load_config().0;
        let result = tauri::async_runtime::spawn_blocking(move || {
            let mut dsh = dsh_handle.lock().unwrap();
            dsh.restart(&cfg.dsh)
        })
        .await;
        match result {
            Ok(Ok(())) => {
                let _ = app2.emit("dsh-ready", ());
            }
            Ok(Err(e)) => {
                let _ = app2.emit("dsh-crashed", e.clone());
            }
            Err(e) => {
                let _ = app2.emit("dsh-crashed", e.to_string());
            }
        }
    });
    Ok("dsh 正在重启…".into())
}

#[tauri::command]
fn poll_now(app: AppHandle) -> Result<Vec<watcher::PollResult>, String> {
    // 异步执行（避免阻塞命令）
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let results = watcher::poll_once(Some(&app2)).await;
        let _ = app2.emit("watcher-poll", &results);
    });
    Ok(Vec::new())
}

#[tauri::command]
fn watcher_status(state: State<'_, AppState>) -> Result<WatcherStatus, String> {
    let st = state
        .watcher
        .status
        .try_lock()
        .map(|g| (*g).clone())
        .unwrap_or_else(|_| WatcherStatus {
            running: true,
            last_poll_at: None,
            last_results: vec![],
        });
    Ok(st)
}

#[tauri::command]
fn open_config(_app: AppHandle) -> Result<(), String> {
    let path = config::config_path();
    // 确保存在
    let _ = config::load_config();
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn restart_backend(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // 远程模式没有本地 dsh 子进程，拒绝重启并提示
    if config::load_config().0.connection.mode == "remote" {
        return Err("当前为远程模式，本地 dsh 未启动".into());
    }
    // 保存远程访问等配置后重启 dsh 服务（异步，避免阻塞命令线程）
    let app2 = app.clone();
    let dsh_handle = state.dsh.0.clone();
    tauri::async_runtime::spawn(async move {
        let cfg = config::load_config().0;
        let result = tauri::async_runtime::spawn_blocking(move || {
            let mut dsh = dsh_handle.lock().unwrap();
            dsh.restart(&cfg.dsh)
        })
        .await;
        match result {
            Ok(Ok(())) => {
                let _ = app2.emit("dsh-ready", ());
            }
            Ok(Err(e)) => {
                let _ = app2.emit("dsh-crashed", e);
            }
            Err(e) => {
                let _ = app2.emit("dsh-crashed", e.to_string());
            }
        }
    });
    Ok("后端正在重启…".into())
}

/// 百分号编码 URL userinfo 片段（RFC 3986：仅保留 unreserved 字符）
fn percent_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// 构建远程访问 URL：配置的 URL + Basic Auth userinfo（`https://user:pass@host/...`）
/// 若 URL 已含 userinfo（如 `https://user@host`）则不重复插入。
fn remote_url_with_auth(cfg: &config::AppConfig) -> String {
    let rc = &cfg.connection.remote;
    let mut url = rc.url.trim().to_string();
    if !url.is_empty() && !rc.username.is_empty() {
        // 检查是否已含 userinfo（:// 后到下一个 / 或 @ 之间的 @ 存在即已带凭证）
        let has_userinfo = url.find("://").is_some_and(|i| {
            let rest = &url[i + 3..];
            let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            rest[..authority_end].contains('@')
        });
        if !has_userinfo {
            let creds = format!(
                "{}:{}",
                percent_encode(&rc.username),
                percent_encode(&rc.password)
            );
            if let Some(idx) = url.find("://") {
                url.insert_str(idx + 3, &format!("{creds}@"));
            }
        }
    }
    url
}

/// 把主窗口导航到 dsh Web UI（本地 3080 或远程 URL）；单窗口方案：
/// 不再开独立 dsh 窗口，壳 UI 由注入脚本（shell.rs SHELL_SCRIPT）叠加在 dsh 页面上。
fn open_dsh_window(app: &AppHandle) {
    let (cfg, _) = config::load_config();
    let url = if cfg.connection.mode == "remote" {
        remote_url_with_auth(&cfg)
    } else {
        format!("http://127.0.0.1:{}", cfg.dsh.port)
    };
    if let Some(win) = app.get_webview_window("main") {
        let Ok(parsed) = url.parse::<tauri::Url>() else {
            let _ = app.emit("dsh-crashed", format!("URL 无效: {url}"));
            return;
        };
        let _ = win.navigate(parsed);
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// 打开 dsh UI（command：供前端按钮调用）；按模式校验就绪条件
#[tauri::command]
fn open_dsh_ui(app: AppHandle) -> Result<String, String> {
    let (cfg, _) = config::load_config();
    if cfg.connection.mode == "remote" {
        let url = remote_url_with_auth(&cfg);
        if url.is_empty() {
            return Err("远程模式未配置 URL，请先在设置里填写".into());
        }
        open_dsh_window(&app);
        return Ok("已打开远程 dsh Web UI".into());
    }
    if !dsh::port_open(cfg.dsh.port) {
        return Err(format!("dsh 服务未就绪（端口 {} 无响应），请稍后再试", cfg.dsh.port));
    }
    open_dsh_window(&app);
    Ok("已打开 dsh Web UI".into())
}

/// 连接模式切换后的进程编排：切远程→停本地 dsh；切本地→重启 dsh
#[tauri::command]
fn apply_connection_mode(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let (cfg, _) = config::load_config();
    let mode = cfg.connection.mode.clone();
    let mode_label = if mode == "remote" { "远程" } else { "本地" };
    let is_remote = mode == "remote";
    let app2 = app.clone();
    let dsh_handle = state.dsh.0.clone();
    tauri::async_runtime::spawn(async move {
        let result = tauri::async_runtime::spawn_blocking(move || {
            let mut dsh = dsh_handle.lock().unwrap();
            if is_remote {
                // 远程模式：确保本地 dsh 停止
                dsh.kill().map_err(|e| e.to_string())
            } else {
                // 本地模式：重启使配置生效
                dsh.restart(&cfg.dsh)
            }
        })
        .await;
        match result {
            Ok(Ok(())) => {
                if !is_remote {
                    let _ = app2.emit("dsh-ready", ());
                }
            }
            Ok(Err(e)) => {
                let _ = app2.emit("dsh-crashed", e.clone());
            }
            Err(e) => {
                let _ = app2.emit("dsh-crashed", e.to_string());
            }
        }
    });
    Ok(format!("已应用连接模式（{mode_label}）"))
}

/// 保存连接配置（模式 + 远程 URL/账号密码），写配置
#[tauri::command]
fn save_connection(
    _app: AppHandle,
    mode: String,
    remote_url: String,
    remote_username: String,
    remote_password: String,
) -> Result<String, String> {
    let (mut cfg, _) = config::load_config();
    if mode != "local" && mode != "remote" {
        return Err(format!("无效的连接模式: {mode}"));
    }
    if mode == "remote" && remote_url.trim().is_empty() {
        return Err("远程模式必须填写远程 URL".into());
    }
    cfg.connection.mode = mode.clone();
    cfg.connection.remote.url = remote_url.trim().into();
    cfg.connection.remote.username = remote_username.trim().into();
    cfg.connection.remote.password = remote_password.into();
    config::save_config(&cfg).map_err(|e| e.to_string())?;
    Ok(format!("已保存连接配置（{}）", if mode == "remote" { "远程" } else { "本地" }))
}

/// 右键菜单目标信息（前端 contextmenu 事件收集后桥接）
/// Tauri 2 宏只转顶层参数名，嵌套结构需显式 camelCase
#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContextMenuTarget {
    is_editable: bool,
    selection_text: String,
    link_url: String,
    image_url: String,
    x: f64,
    y: f64,
}

/// 退出应用（壳 UI 关闭按钮调用）
#[tauri::command]
fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

/// 最小化主窗口（壳 UI 缩小按钮调用）
#[tauri::command]
fn window_minimize(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 最大化/还原主窗口（壳 UI 放大按钮调用）
#[tauri::command]
fn window_toggle_maximize(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_maximized().unwrap_or(false) {
            win.unmaximize().map_err(|e| e.to_string())?;
        } else {
            win.maximize().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 开始拖动主窗口（壳 UI 悬浮拖拽手柄长按调用）
#[tauri::command]
fn window_start_drag(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.start_dragging().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 弹出原生右键菜单：按目标类型构建菜单，点击后 emit `ctx-menu-action` 回前端执行
/// （动作如 cut/copy/paste 需在页面上下文执行，Rust 侧只负责弹菜单与转发）。
#[tauri::command]
fn show_context_menu(
    window: WebviewWindow,
    app: AppHandle,
    target: ContextMenuTarget,
) -> Result<(), String> {
    popup_context_menu(&app, window.label(), &target)
}

/// 构建并弹出原生右键菜单（command 与事件桥接共用）
fn popup_context_menu(app: &AppHandle, window_label: &str, target: &ContextMenuTarget) -> Result<(), String> {
    let menu = Menu::new(app).map_err(|e| e.to_string())?;
    let push = |menu: &Menu<tauri::Wry>, id: &str, label: &str, enabled: bool| -> Result<(), String> {
        menu.append(
            &MenuItem::with_id(app, id, label, enabled, None::<&str>)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    };
    let separator = |menu: &Menu<tauri::Wry>| -> Result<(), String> {
        menu.append(
            &tauri::menu::PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    };

    if target.is_editable {
        push(&menu, "ctx-cut", "剪切", true)?;
        push(&menu, "ctx-copy", "复制", true)?;
        push(&menu, "ctx-paste", "粘贴", true)?;
        push(&menu, "ctx-paste-plain", "粘贴为纯文本", true)?;
        if !target.selection_text.is_empty() {
            push(&menu, "ctx-delete", "删除", true)?;
        }
        separator(&menu)?;
        push(&menu, "ctx-select-all", "全选", true)?;
    } else if !target.selection_text.is_empty() {
        push(&menu, "ctx-copy", "复制", true)?;
        separator(&menu)?;
        push(&menu, "ctx-select-all", "全选", true)?;
    } else {
        push(&menu, "ctx-back", "返回", true)?;
        push(&menu, "ctx-forward", "前进", true)?;
        separator(&menu)?;
        push(&menu, "ctx-reload", "刷新", true)?;
    }
    if !target.link_url.is_empty() {
        separator(&menu)?;
        push(&menu, "ctx-copy-link", "复制链接", true)?;
        push(&menu, "ctx-open-link", "用浏览器打开", true)?;
    }
    if !target.image_url.is_empty() {
        separator(&menu)?;
        push(&menu, "ctx-copy-image", "复制图片", true)?;
        push(&menu, "ctx-save-image", "图片另存为…", true)?;
    }
    if menu.items().map_err(|e| e.to_string())?.is_empty() {
        return Ok(());
    }
    // 记录发起窗口与目标信息，菜单点击事件据此定向 emit（MenuEvent 只有 id）
    if let Some(state) = app.try_state::<AppState>() {
        *state.ctx_menu_window.lock().unwrap() = Some(window_label.into());
        *state.ctx_menu_target.lock().unwrap() = Some(target.clone());
    }
    // 点击事件统一走 app 级 menu event（见 run() 里的 on_menu_event）
    if let Some(win) = app.get_webview_window(window_label) {
        let _ = win.popup_menu_at(
            &menu,
            Position::Logical(LogicalPosition::new(target.x, target.y)),
        );
    }
    Ok(())
}

/// 右键菜单动作分发：菜单项点击后在发起窗口直接 eval 执行 JS
/// （execCommand/clipboard/导航等都需要页面上下文，Tauri 无 webContents 级 API）。
fn handle_ctx_menu_action(app: &AppHandle, action: &str) {
    let (label, target) = app
        .try_state::<AppState>()
        .map(|s| {
            (
                s.ctx_menu_window.lock().ok().and_then(|g| g.clone()),
                s.ctx_menu_target.lock().ok().and_then(|t| t.clone()),
            )
        })
        .unwrap_or_default();
    let label = label.unwrap_or_else(|| "main".into());
    let link = target.as_ref().map(|t| t.link_url.clone()).unwrap_or_default();
    let image = target.as_ref().map(|t| t.image_url.clone()).unwrap_or_default();
    // JSON 序列化保证 URL/文本安全嵌入 JS 字符串
    let link_js = serde_json::to_string(&link).unwrap_or_else(|_| "\"\"".into());
    let image_js = serde_json::to_string(&image).unwrap_or_else(|_| "\"\"".into());
    let js = match action {
        "ctx-cut" => "document.execCommand('cut')".into(),
        "ctx-copy" => "document.execCommand('copy')".into(),
        "ctx-paste" => "document.execCommand('paste')".into(),
        "ctx-paste-plain" => "document.execCommand('insertText', false, '')".into(),
        "ctx-delete" => "document.execCommand('delete')".into(),
        "ctx-select-all" => "document.execCommand('selectAll')".into(),
        "ctx-back" => "history.back()".into(),
        "ctx-forward" => "history.forward()".into(),
        "ctx-reload" => "location.reload()".into(),
        "ctx-copy-link" => format!("navigator.clipboard?.writeText({link_js}).catch(()=>{{}})"),
        "ctx-open-link" => {
            // 外部链接交给系统浏览器，避免 WebView 内嵌导航
            let _ = tauri_plugin_opener::open_url(link, None::<&str>);
            String::new()
        }
        "ctx-copy-image" => format!(
            "fetch({image_js}).then(r=>r.blob()).then(b=>navigator.clipboard?.write([new ClipboardItem({{[b.type||'image/png']:b}})])).catch(()=>{{}})"
        ),
        "ctx-save-image" => format!(
            "fetch({image_js}).then(r=>r.blob()).then(b=>{{const a=document.createElement('a');a.href=URL.createObjectURL(b);a.download={image_js}.split('/').pop()||'image';document.body.appendChild(a);a.click();a.remove();URL.revokeObjectURL(a.href)}}).catch(()=>{{}})"
        ),
        _ => return,
    };
    if let Some(win) = app.get_webview_window(&label) {
        if !js.is_empty() {
            let _ = win.eval(js);
        }
    }
}

/// 启动流程：dsh 进程（本地模式）+ watcher
fn boot(app: &AppHandle, state: &AppState) {
    let (cfg, _) = config::load_config();
    // 远程模式：不启动本地 dsh 子进程（连接远程实例），仅运行 watcher 保持自动更新
    if cfg.connection.mode == "remote" {
        log::info!("[boot] 远程模式，跳过本地 dsh 启动");
        watcher::start_watcher(app.clone(), state.watcher.clone());
        // 直接导航主窗口到远程 dsh UI
        open_dsh_window(app);
        return;
    }
    // 1) 启动 dsh（独立 std::thread，完全脱离 Tauri async runtime，
    //    避免 runtime 线程池环境导致子进程启动异常；就绪后发事件）
    {
        let app = app.clone();
        let state_dsh = state.dsh.0.clone();
        std::thread::spawn(move || {
            let cfg = config::load_config().0;
            let mut dsh = state_dsh.lock().unwrap();
            if let Err(e) = dsh.start(&cfg.dsh) {
                dsh.last_error = Some(e.clone());
                let _ = app.emit("dsh-crashed", e);
                return;
            }
            match dsh.wait_ready(&cfg.dsh) {
                Ok(()) => {
                    let _ = app.emit("dsh-ready", ());
                    // 双击启动后自动打开 dsh Web UI 窗口
                    open_dsh_window(&app);
                }
                Err(e) => {
                    dsh.last_error = Some(e.clone());
                    let _ = app.emit("dsh-crashed", e);
                }
            }
        });
    }
    // 2) watcher
    watcher::start_watcher(app.clone(), state.watcher.clone());
}

/// macOS：阻止系统自动终止应用（App Nap / 无窗口时终止），保证后台 dsh 服务持续运行
#[cfg(target_os = "macos")]
fn prevent_automatic_termination() {
    use objc2_foundation::{NSProcessInfo, NSActivityOptions, NSString};
    let process_info = NSProcessInfo::processInfo();
    // AutomaticTerminationDisabled + UserInitiated：声明应用持续工作，
    // 系统不得因无窗口/空闲而自动终止（保证后台 dsh 与网关常驻）
    let options = NSActivityOptions::AutomaticTerminationDisabled
        | NSActivityOptions::UserInitiated;
    let reason = NSString::from_str("dsh-desktop keeps dsh and gateway running");
    // activity 需保持存活直到进程结束；直接 forget 返回值避免提前释放
    std::mem::forget(process_info.beginActivityWithOptions_reason(options, &reason));
    log::info!("[macos] 已禁用自动终止");
}

#[cfg(not(target_os = "macos"))]
fn prevent_automatic_termination() {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    prevent_automatic_termination();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                // 排除 VISIBLE：窗口显示时机由 splash 流程控制（visible:false + dsh-ready 后显示），
                // 插件默认恢复可见性会让窗口在 splash 前裸显
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        - tauri_plugin_window_state::StateFlags::VISIBLE,
                )
                .build(),
        )
        .manage(AppState {
            dsh: DshHandle::new(),
            watcher: Arc::new(WatcherShared::default()),
            ctx_menu_window: std::sync::Mutex::new(None),
            ctx_menu_target: std::sync::Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            restart_dsh,
            poll_now,
            watcher_status,
            open_config,
            restart_backend,
            open_dsh_ui,
            save_connection,
            apply_connection_mode,
            show_context_menu,
            quit_app,
            window_minimize,
            window_toggle_maximize,
            window_start_drag,
        ])
        .setup(|app| {
            // 创建主窗口：加载 index.html（启动 splash + 壳 UI），注入 SHELL_SCRIPT——
            // WKUserScript 对后续每次导航（dsh 页面 3080/远程）自动重注入，
            // 因此 navigate 后工具条/设置/右键菜单在 dsh 页面上依然可用。
            use tauri::{WebviewUrl, WebviewWindowBuilder};
            #[allow(unused_mut)]
            let mut builder = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("dsh desktop")
            .inner_size(1280.0, 800.0)
            .min_inner_size(800.0, 600.0)
            .resizable(true)
            .fullscreen(false)
            .visible(false)
            .decorations(false);
            // macOS 专属：Overlay 标题栏 + 红黄绿控制按钮位置（Windows 无这些 API，无边框靠注入岛控制）
            #[cfg(target_os = "macos")]
            {
                builder = builder
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .traffic_light_position(tauri::LogicalPosition::new(14.0, 14.0));
            }
            let win = builder
            .initialization_script(shell::SHELL_SCRIPT)
            // 兜底：导航（含 dsh 页面）后重新注入壳层 UI——
            // initialization_script 在 navigate 后可能不重跑，on_page_load 保证每次注入
            .on_page_load(|win, payload| {
                use tauri::webview::PageLoadEvent;
                if payload.event() == PageLoadEvent::Finished {
                    // 先注入当前连接配置（设置面板填充用），再注入壳层 UI
                    // （__dsh_apply_cfg 由 SHELL_SCRIPT 定义：document start 注入时
                    // __dsh_cfg 尚不存在，而幂等 guard 会挡第二次 SHELL_SCRIPT，
                    // 因此这里注入 cfg 后显式调用一次应用，保证模式标签/表单同步）
                    let (cfg, _) = config::load_config();
                    let js = format!(
                        "window.__dsh_cfg = {{ mode: {}, remoteUrl: {}, remoteUser: {} }};\nif (window.__dsh_apply_cfg) window.__dsh_apply_cfg(window.__dsh_cfg);",
                        serde_json::to_string(&cfg.connection.mode).unwrap_or_else(|_| "\"local\"".into()),
                        serde_json::to_string(&cfg.connection.remote.url).unwrap_or_else(|_| "\"\"".into()),
                        serde_json::to_string(&cfg.connection.remote.username).unwrap_or_else(|_| "\"\"".into()),
                    );
                    let _ = win.eval(js);
                    let _ = win.eval(shell::SHELL_SCRIPT);
                }
            })
            .build()
            .expect("failed to build main window");

            // 无边框窗口效果：Win11 Mica / macOS vibrancy（desktop-polish F-02）
            #[cfg(target_os = "windows")]
            {
                let _ = window_vibrancy::apply_mica(&win, None);
            }
            #[cfg(target_os = "macos")]
            {
                let _ = window_vibrancy::apply_vibrancy(
                    &win,
                    window_vibrancy::NSVisualEffectMaterial::HudWindow,
                    None,
                    Some(18.0),
                );
            }
            // 启动 splash 期间先显示窗口（当前加载的是 index.html，自带 splash）
            let _ = win.show();
            let _ = win.set_focus();

            // 托盘
            let open = MenuItem::with_id(app, "open", "打开 dsh Web UI", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "设置（远程访问）", true, None::<&str>)?;
            let check = MenuItem::with_id(app, "check", "立即检查更新", true, None::<&str>)?;
            let cfg_item = MenuItem::with_id(app, "config", "编辑配置文件", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &settings, &check, &cfg_item, &quit])?;

            let tray_icon = app.default_window_icon().cloned();
            let tray_builder = TrayIconBuilder::with_id("main-tray");
            let tray_builder = if let Some(icon) = &tray_icon {
                tray_builder.icon(icon.clone())
            } else {
                tray_builder
            };
            let _tray = tray_builder
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        open_dsh_window(app);
                    }
                    "settings" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                            let _ = win.emit("show-settings", ());
                        }
                    }
                    "check" => {
                        let app2 = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = watcher::poll_now_cmd(app2).await;
                        });
                    }
                    "config" => {
                        let _ = open_config(app.clone());
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        open_dsh_window(&app);
                    }
                })
                .build(app)?;

            app.manage(_tray);

            // 壳 UI 事件桥接：注入脚本运行在远程 origin（3080/远程 URL），
            // 自定义 command 会被 ACL 拒绝，因此窗口控制用 core:window 内置命令，
            // 退出/保存走事件（core:event:default 允许 emit）。
            {
                let quit_app = app.handle().clone();
                app.listen("dsh-quit", move |_| {
                    quit_app.exit(0);
                });
                let save_app = app.handle().clone();
                let save_dsh = app.state::<AppState>().dsh.0.clone();
                app.listen("dsh-save-connection", move |event| {
                    let p = event.payload();
                    let v: serde_json::Value = serde_json::from_str(p).unwrap_or_else(|_| serde_json::json!({}));
                    let mode = v.get("mode").and_then(|m| m.as_str()).unwrap_or("local").to_string();
                    let url = v.get("remoteUrl").and_then(|m| m.as_str()).unwrap_or("").to_string();
                    let user = v.get("remoteUsername").and_then(|m| m.as_str()).unwrap_or("").to_string();
                    let pass = v.get("remotePassword").and_then(|m| m.as_str()).unwrap_or("").to_string();
                    if (mode != "local" && mode != "remote") || (mode == "remote" && url.trim().is_empty()) {
                        let _ = save_app.emit("dsh-save-result", "保存失败：参数无效".to_string());
                        return;
                    }
                    let (mut cfg, _) = config::load_config();
                    cfg.connection.mode = mode.clone();
                    cfg.connection.remote.url = url.trim().into();
                    cfg.connection.remote.username = user.trim().into();
                    cfg.connection.remote.password = pass.into();
                    if let Err(e) = config::save_config(&cfg) {
                        let _ = save_app.emit("dsh-save-result", e);
                        return;
                    }
                    let is_remote = mode == "remote";
                    let app3 = save_app.clone();
                    let dsh3 = save_dsh.clone();
                    tauri::async_runtime::spawn(async move {
                        let result = tauri::async_runtime::spawn_blocking(move || {
                            let mut dsh = dsh3.lock().unwrap();
                            if is_remote {
                                dsh.kill().map_err(|e| e.to_string())
                            } else {
                                dsh.restart(&config::load_config().0.dsh)
                            }
                        })
                        .await;
                        match result {
                            Ok(Ok(())) => {
                                if !is_remote {
                                    let _ = app3.emit("dsh-ready", ());
                                }
                                // 进程编排完成后导航主窗口到目标（本地 3080 或远程 URL），
                                // 确保页面与模式标签同步切换
                                open_dsh_window(&app3);
                            }
                            Ok(Err(e)) => { let _ = app3.emit("dsh-crashed", e); }
                            Err(e) => { let _ = app3.emit("dsh-crashed", e.to_string()); }
                        }
                    });
                    // 切到远程：本地 dsh 已停，立即导航远程（不等进程编排）
                    if is_remote {
                        open_dsh_window(&save_app);
                    }
                    let _ = save_app.emit("dsh-save-result", true);
                });
                // 右键菜单：事件桥接（远程 origin 的自定义 command 会被 ACL 拒）
                let ctx_app = app.handle().clone();
                app.listen("dsh-show-context-menu", move |event| {
                    let p = event.payload();
                    let v: serde_json::Value = serde_json::from_str(p).unwrap_or_else(|_| serde_json::json!({}));
                    let target = ContextMenuTarget {
                        is_editable: v.get("isEditable").and_then(|m| m.as_bool()).unwrap_or(false),
                        selection_text: v.get("selectionText").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                        link_url: v.get("linkURL").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                        image_url: v.get("imageURL").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                        x: v.get("x").and_then(|m| m.as_f64()).unwrap_or(0.0),
                        y: v.get("y").and_then(|m| m.as_f64()).unwrap_or(0.0),
                    };
                    let _ = popup_context_menu(&ctx_app, "main", &target);
                });

                // dsh-notification 插件桥：WebView 里浏览器通知不可用，
                // 前端发 dsh-notify 事件 → 这里弹 macOS/Windows 系统通知
                app.listen("dsh-notify", move |event| {
                    let p = event.payload();
                    let v: serde_json::Value = serde_json::from_str(p).unwrap_or_else(|_| serde_json::json!({}));
                    let title = v.get("title").and_then(|m| m.as_str()).unwrap_or("dsh").to_string();
                    let body = v.get("body").and_then(|m| m.as_str()).unwrap_or("").to_string();
                    notify::notify(&title, &body);
                });
            }

            // 显示主窗口（splash 覆盖层随前端渲染出现，dsh ready 后淡出）
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
            }

            // 启动
            {
                let app = app.handle();
                let state = app.state::<AppState>();
                boot(app, &state);
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            // 右键菜单动作（ctx- 前缀）；托盘菜单事件走 tray builder 自己的回调
            let id = event.id().0.clone();
            if id.starts_with("ctx-") {
                handle_ctx_menu_action(app, &id);
            }
        })
        .on_window_event(|window, event| {
            // 关闭主窗口（壳控制台）时隐藏到托盘而不是退出；dsh UI 窗口直接关闭
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // 应用退出时回收 dsh 子进程，避免残留孤儿进程占用端口
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<AppState>() {
                    let mut dsh = state.dsh.0.lock().unwrap();
                    let _ = dsh.kill();
                }
            }
        });
}
