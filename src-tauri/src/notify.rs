use std::process::Command;

/// 系统通知出口：macOS 用 osascript；Windows 用 powershell toast（后续补充）
pub fn notify(title: &str, body: &str) {
    let safe_title = sanitize(title);
    let safe_body = sanitize(body);
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("osascript")
            .args([
                "-e",
                &format!(
                    "display notification \"{safe_body}\" with title \"{safe_title}\""
                ),
            ])
            .output();
    }
    #[cfg(target_os = "windows")]
    {
        // 简单可靠的 Windows 通知：msg.exe + 注册表（不依赖 BurntToast 模块）
        let _ = Command::new("powershell")
            .args([
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &format!(
                    "New-Object -ComObject Wscript.Shell; $s=New-Object -ComObject Wscript.Shell; $s.Popup(\"{safe_body}\", 5, \"{safe_title}\", 64) | Out-Null"
                ),
            ])
            .output();
    }
    log::info!("[notify] {safe_title}: {safe_body}");
}

/// 转义 osascript 字符串中的危险字符
fn sanitize(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
