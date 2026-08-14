mod config;
mod dsh;
mod git;
mod notify;
mod rebuild;
mod state;
mod watcher;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};

use dsh::DshHandle;
use watcher::{WatcherShared, WatcherStatus};

/// 应用共享状态
pub struct AppState {
    pub dsh: DshHandle,
    pub watcher: Arc<WatcherShared>,
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

/// 打开 dsh Web UI 独立窗口（不覆盖壳控制台主窗口）
fn open_dsh_window(app: &AppHandle) {
    let url = format!("http://127.0.0.1:{}", config::load_config().0.dsh.port);
    if let Some(win) = app.get_webview_window("dsh") {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let _ = WebviewWindowBuilder::new(app, "dsh", WebviewUrl::External(url.parse().unwrap()))
        .title("dsh Web UI")
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .build()
        .map(|win| {
            let _ = win.show();
            let _ = win.set_focus();
        });
}

/// 打开 dsh UI（command：供前端按钮调用）
#[tauri::command]
fn open_dsh_ui(app: AppHandle) -> Result<String, String> {
    let (cfg, _) = config::load_config();
    if !dsh::port_open(cfg.dsh.port) {
        return Err(format!("dsh 服务未就绪（端口 {} 无响应），请稍后再试", cfg.dsh.port));
    }
    open_dsh_window(&app);
    Ok("已打开 dsh Web UI".into())
}

/// 启动流程：dsh 进程 + watcher + 远程网关
fn boot(app: &AppHandle, state: &AppState) {
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
        .manage(AppState {
            dsh: DshHandle::new(),
            watcher: Arc::new(WatcherShared::default()),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            restart_dsh,
            poll_now,
            watcher_status,
            open_config,
            restart_backend,
            open_dsh_ui,
        ])
        .setup(|app| {
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

            // 启动
            {
                let app = app.handle();
                let state = app.state::<AppState>();
                boot(app, &state);
            }
            Ok(())
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
