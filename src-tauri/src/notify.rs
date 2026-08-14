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
        let _ = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "New-BurntToastNotification -Text '{safe_title}', '{safe_body}'"
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
