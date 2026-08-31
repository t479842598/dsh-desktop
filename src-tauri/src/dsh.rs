use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::{config_dir, DshConfig};

/// dsh 子进程 + 就绪状态
pub struct DshProcess {
    pub child: Option<Child>,
    pub ready: bool,
    pub last_error: Option<String>,
}

impl DshProcess {
    pub fn new() -> Self {
        Self {
            child: None,
            ready: false,
            last_error: None,
        }
    }

    /// spawn dsh web 子进程；stdout/stderr 重定向到日志文件
    pub fn start(&mut self, cfg: &DshConfig) -> Result<(), String> {
        if self.child.is_some() {
            let _ = self.kill();
        }
        // 健康复用：端口已由外部常驻服务（如 launchd KeepAlive）提供可用的 dsh Web
        // 服务时直接复用、不杀不拉——否则与 KeepAlive 服务互抢端口（杀→秒级重启
        // →自己拉起的 dsh 启动慢，bind 时 EADDRINUSE 崩溃）会导致 splash 卡死
        if healthy_dsh(cfg.port) {
            log::info!("[dsh] 端口 {} 已有健康 dsh 服务，直接复用", cfg.port);
            self.ready = false;
            self.last_error = None;
            return Ok(());
        }
        // 端口占用自愈：仅当端口被非健康进程占用（连不上/非 dsh）时，杀掉占用该
        // 端口的 node/tsx 进程后重试，避免 EADDRINUSE 导致服务起不来
        if port_open(cfg.port) {
            log::warn!("[dsh] 端口 {} 被非健康进程占用，尝试清理残留进程", cfg.port);
            kill_port_owner(cfg.port);
            std::thread::sleep(std::time::Duration::from_millis(800));
            if port_open(cfg.port) {
                return Err(format!(
                    "端口 {} 仍被占用且无法自动清理，请手动关闭占用进程后重试",
                    cfg.port
                ));
            }
        }
        let log_dir = config_dir().join("logs");
        std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
        let log_file = log_dir.join("dsh.log");
        let f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| e.to_string())?;

        if cfg.command.is_empty() {
            return Err("dsh.command 为空".into());
        }

        // 通过 shell 做 chdir 再 exec：绕开 Rust Command::current_dir 在 GUI 上下文
        // 偶发的 chdir 失效问题（观察到子进程 cwd 落在 / 导致 node getcwd 卡死）
        let mut command;
        #[cfg(unix)]
        {
            // Unix：/bin/sh，单引号转义，独立进程组（setsid 语义，避免继承 GUI 会话）
            let mut shell_cmd = String::from("cd ");
            shell_cmd.push_str(&shell_quote(&cfg.cwd));
            shell_cmd.push_str(" && exec");
            for part in &cfg.command {
                shell_cmd.push(' ');
                shell_cmd.push_str(&shell_quote(part));
            }
            command = Command::new("/bin/sh");
            command
                .args(["-c", &shell_cmd])
                .process_group(0);
        }
        #[cfg(windows)]
        {
            // Windows：直接以 dsh.cmd 作为程序启动（Rust 会自动按 PATHEXT 找到
            // dsh.cmd 并用 cmd /c 运行），工作目录用 current_dir 设置。
            // 不要手动拼 `cd /d "..." && cmd /C ...` 字符串：Rust 的 args 会给
            // 含空格的参数整体加引号，与 cmd 的引号解析冲突（嵌套引号 → UNC
            // 路径 / 语法错误，3080 起不来）。
            // CREATE_NO_WINDOW：隐藏 cmd 控制台窗口（否则启动 dsh 时闪终端）。
            if cfg.command.is_empty() {
                return Err("dsh.command 为空".into());
            }
            let mut cmd = Command::new(&cfg.command[0]);
            cmd.args(&cfg.command[1..]);
            cmd.current_dir(&cfg.cwd);
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            command = cmd;
        }
        command
            .stdout(Stdio::from(f.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(f));
        // GUI 应用从 Finder 启动时 PATH 精简，补全常见 Node 工具链路径（macOS）
        #[cfg(target_os = "macos")]
        {
            let mut path = std::env::var("PATH").unwrap_or_default();
            // dsh 脚本 shebang 为 #!/usr/bin/env node，必须保证 node 在 PATH 中；
            // dsh 命令所在目录通常即 node 所在目录（如 ~/.local/node/bin），优先补入
            if let Some(dir) = std::path::Path::new(&cfg.command[0]).parent() {
                let dir = dir.to_string_lossy().to_string();
                if !path.split(':').any(|p| p == dir) {
                    path.push(':');
                    path.push_str(&dir);
                }
            }
            for extra in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/local/opt/node/bin"] {
                if !path.split(':').any(|p| p == extra) {
                    path.push(':');
                    path.push_str(extra);
                }
            }
            command.env("PATH", path);
        }
        // 清除 GUI 会话继承变量，避免 node 启动时与 WindowServer/LaunchServices 交互阻塞（macOS）
        #[cfg(target_os = "macos")]
        for var in [
            "__CFBundleIdentifier",
            "__CF_USER_TEXT_ENCODING",
            "XPC_FLAGS",
            "XPC_SERVICE_NAME",
        ] {
            command.env_remove(var);
        }
        // 显式设置 HOME 与 LANG，保证 node/tsx 行为确定（Unix；Windows 用 USERPROFILE）
        #[cfg(unix)]
        {
            if let Ok(home) = std::env::var("HOME") {
                command.env("HOME", home);
            }
            command.env("LANG", "en_US.UTF-8");
        }
        let child = command
            .spawn()
            .map_err(|e| format!("启动 dsh 失败: {e}"))?;
        self.child = Some(child);
        self.ready = false;
        self.last_error = None;
        Ok(())
    }

    /// 轮询端口直至就绪
    pub fn wait_ready(&mut self, cfg: &DshConfig) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(cfg.ready_timeout_sec.max(5));
        loop {
            if let Some(child) = self.child.as_mut() {
                if child.try_wait().ok().flatten().is_some() {
                    // 子进程已退出：可能是与常驻服务（launchd KeepAlive）抢端口时
                    // 落败，自己拉起的实例因 EADDRINUSE 崩溃。此时给端口一段宽限期，
                    // 若已被健康 dsh 服务接管（如 launchd 重启的实例）则直接复用
                    // 视为就绪；宽限期后仍无健康服务才报错，避免 splash 永久卡死。
                    self.child = None;
                    let grace = Instant::now() + Duration::from_secs(15);
                    while Instant::now() < grace && Instant::now() < deadline {
                        if healthy_dsh(cfg.port) {
                            self.ready = true;
                            return Ok(());
                        }
                        std::thread::sleep(Duration::from_millis(500));
                    }
                    self.ready = false;
                    return Err("dsh 进程已退出".into());
                }
            }
            if port_open(cfg.port) {
                self.ready = true;
                return Ok(());
            }
            if Instant::now() > deadline {
                self.ready = false;
                return Err(format!(
                    "等待 dsh 就绪超时（{}s），请查看日志",
                    cfg.ready_timeout_sec
                ));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// kill 子进程并回收
    pub fn kill(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            self.ready = false;
        }
        Ok(())
    }

    /// 重启（kill + start + wait_ready）
    pub fn restart(&mut self, cfg: &DshConfig) -> Result<(), String> {
        self.kill()?;
        self.start(cfg)?;
        self.wait_ready(cfg)
    }
}

/// 从 dsh 启动日志中解析最近一次打印的「带 launch token 的 Web UI URL」。
///
/// dsh web（0.1.2-alpha 起）启动时会在 stdout 打印一行
/// `dsh web: http://127.0.0.1:<port>/?token=...`，该 token 是进程级随机生成
/// 的认证凭据，客户端必须携带才能通过 dsh 的 browser-auth 栅栏（否则 401）。
/// 旧版（无认证）打印的裸 URL 同样会被解析并返回，兼容两种版本。
///
/// 覆盖两种启动来源：
/// - 本壳自己拉起的 dsh：stdout 重定向到 ~/.dsh-desktop/logs/dsh.log；
/// - launchd 常驻服务（健康复用时复用）：stdout 写到 ~/.dsh/harness.out.log。
///
/// 取「最近一次」匹配行：token 随进程重启失效，只有最后一次启动的输出
/// 才与当前监听进程对应。找不到匹配行返回 None（调用方回退裸 URL）。
pub fn launch_token_url(cfg: &DshConfig) -> Option<String> {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let candidates = [
        config_dir().join("logs").join("dsh.log"),
        home.join(".dsh").join("harness.out.log"),
    ];
    let needle = format!("dsh web: http://127.0.0.1:{}", cfg.port);
    let mut last: Option<String> = None;
    for path in candidates {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines().rev() {
            if line.starts_with(&needle) {
                last = Some(line.trim().to_string());
                break;
            }
        }
    }
    last.and_then(|line| {
        let after = line.trim_start_matches("dsh web: ").trim();
        let url = after.split_whitespace().next().unwrap_or("");
        if url.is_empty() {
            None
        } else {
            Some(url.to_string())
        }
    })
}

/// 单引号转义 shell 参数（用于 /bin/sh -c 拼接）
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\\\''"))
}

/// Windows 双引号转义参数（用于 cmd /C 拼接）：内部双引号翻倍，避免提前闭合
#[cfg(windows)]
fn win_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\"', "\"\""))
}

/// 杀掉占用指定端口的进程（仅限 node/tsx 类，即 dsh 运行进程；避免误杀用户其它程序）
fn kill_port_owner(port: u16) {
    #[cfg(unix)]
    {
        let out = std::process::Command::new("lsof")
            .args(["-ti", &format!(":{port}")])
            .output();
        let Ok(out) = out else { return };
        if !out.status.success() {
            return;
        }
        let pids = String::from_utf8_lossy(&out.stdout);
        for pid in pids.lines() {
            let Ok(p) = pid.trim().parse::<i32>() else { continue };
            // 只杀 node/tsx 进程（dsh 是 node 服务）；pnpm/npx 包装进程也会被 lsof 列出，一并处理
            let comm = std::process::Command::new("ps")
                .args(["-p", &p.to_string(), "-o", "comm="])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
            let comm_lower = comm.to_lowercase();
            if comm_lower.contains("node") || comm_lower.contains("tsx") || comm_lower.contains("pnpm") {
                log::warn!("[dsh] 清理端口占用进程 pid={} ({})", p, comm.trim());
                let _ = std::process::Command::new("kill").arg(&p.to_string()).status();
            }
        }
    }
    #[cfg(windows)]
    {
        // Windows：netstat 找占用 PID，taskkill 强杀（按进程名过滤 node/tsx/pnpm）
        let out = std::process::Command::new("netstat")
            .args(["-ano"])
            .output();
        let Ok(out) = out else { return };
        let text = String::from_utf8_lossy(&out.stdout);
        let mut pids = std::collections::HashSet::new();
        let needle = format!(":{port} ");
        for line in text.lines() {
            if line.contains(&needle) && (line.contains("LISTENING") || line.contains("LISTEN")) {
                if let Some(pid) = line.split_whitespace().last() {
                    if let Ok(p) = pid.parse::<i32>() {
                        pids.insert(p);
                    }
                }
            }
        }
        for p in pids {
            // 只杀 node/tsx 进程
            let comm = std::process::Command::new("tasklist")
                .args(["/FI", &format!("PID eq {p}"), "/FO", "CSV", "/NH"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
            let comm_lower = comm.to_lowercase();
            if comm_lower.contains("node") || comm_lower.contains("tsx") || comm_lower.contains("pnpm") {
                log::warn!("[dsh] 清理端口占用进程 pid={}", p);
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &p.to_string(), "/F"])
                    .status();
            }
        }
    }
}

/// 检查端口是否可连接
pub fn port_open(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(300),
    )
    .is_ok()
}

/// 探测端口是否已在提供健康的 dsh Web 服务：TCP 可连 + HTTP 200 + 页面含 dsh 特征。
/// 用于「健康复用」判定：只有确认是 dsh 页面才复用，避免把非 dsh 的占用进程
/// 误当服务（此类情况仍走自愈杀进程路径）。dsh 服务可能瞬时繁忙（如刚被
/// launchd 重启、多实例竞争时），故失败后重试 3 次，避免误判导致与常驻服务互杀。
fn healthy_dsh(port: u16) -> bool {
    for attempt in 0..3 {
        if healthy_dsh_once(port) {
            return true;
        }
        if attempt < 2 {
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    false
}

fn healthy_dsh_once(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(300),
    ) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(3000)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(1500)));
    use std::io::{Read, Write};
    // Accept-Encoding: identity 避免 gzip 压缩导致 body 里检测不到 dsh 特征
    if stream
        .write_all(
            format!(
                "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept-Encoding: identity\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .is_err()
    {
        return false;
    }
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buf);
                if text.contains(" 200 ") && text.to_lowercase().contains("dsh") {
                    return true;
                }
                if buf.len() > 8192 {
                    break;
                }
            }
        }
    }
    let text = String::from_utf8_lossy(&buf);
    text.starts_with("HTTP/1.") && text.contains(" 200 ") && text.to_lowercase().contains("dsh")
}

/// 全局 dsh 进程句柄（Arc 便于跨线程克隆）
pub struct DshHandle(pub Arc<Mutex<DshProcess>>);

impl DshHandle {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(DshProcess::new())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// 起一个返回固定 body 的 HTTP 200 服务，模拟 dsh Web UI（或普通服务）
    fn serve_http(port: u16, body: &'static str) -> std::thread::JoinHandle<()> {
        let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { continue };
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        })
    }

    fn wait_listening(port: u16) {
        for _ in 0..100 {
            if port_open(port) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn healthy_dsh_detects_dsh_web_service() {
        let port = 48080;
        let _srv = serve_http(
            port,
            "<html data-dsh-skin=\"summer-liquid-glass\"><head></head><body>dsh client modules</body></html>",
        );
        wait_listening(port);
        assert!(healthy_dsh(port));
    }

    #[test]
    fn healthy_dsh_rejects_non_dsh_service() {
        let port = 48081;
        let _srv = serve_http(port, "<html><body>some other service</body></html>");
        wait_listening(port);
        assert!(!healthy_dsh(port));
    }

    #[test]
    fn healthy_dsh_false_when_nothing_listens() {
        let port = 48082;
        let l = TcpListener::bind(("127.0.0.1", port)).unwrap();
        drop(l); // 端口已释放，无监听
        assert!(!healthy_dsh(port));
    }

    #[test]
    fn start_reuses_healthy_service_without_spawning() {
        let port = 48083;
        let _srv = serve_http(port, "<html data-dsh-skin=\"x\">dsh ui</html>");
        wait_listening(port);
        let mut cfg = DshConfig::default();
        cfg.port = port;
        let mut dsh = DshProcess::new();
        dsh.start(&cfg).expect("复用健康服务应成功");
        assert!(dsh.child.is_none(), "复用外部服务时不应 spawn 子进程");
        dsh.wait_ready(&cfg).expect("外部服务就绪探测应成功");
        assert!(dsh.ready);
    }

    #[cfg(unix)]
    #[test]
    fn wait_ready_reuses_surviving_service_after_child_exit() {
        let port = 48084;
        // 模拟 launchd KeepAlive：app 自己的子进程先退出（EADDRINUSE 崩溃），
        // 稍后常驻服务重启接管端口
        let srv = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1200));
            let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { continue };
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                let body = "<html data-dsh-skin=\"x\">dsh ui</html>";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        let child = std::process::Command::new("sleep")
            .arg("0.3")
            .spawn()
            .expect("spawn child");
        let mut dsh = DshProcess::new();
        dsh.child = Some(child);
        let mut cfg = DshConfig::default();
        cfg.port = port;
        cfg.ready_timeout_sec = 10;
        let r = dsh.wait_ready(&cfg);
        // 服务线程常驻等待连接，进程退出时自动结束，无需 join
        assert!(r.is_ok(), "子进程退出后应复用接管端口的外部服务: {:?}", r);
        assert!(dsh.ready);
        assert!(dsh.child.is_none());
    }
    #[test]
    fn launch_token_url_parses_latest_url_with_token() {
        // 用临时 HOME + DSH_DESKTOP_CONFIG_DIR 隔离，避免写真实用户目录
        let tmp = std::env::temp_dir().join("dsh-launch-token-test-parity");
        std::env::set_var("HOME", &tmp);
        std::env::set_var("DSH_DESKTOP_CONFIG_DIR", tmp.join("cfg"));

        let cfg = DshConfig::default();
        let home = dirs::home_dir().unwrap();
        let log_dir = config_dir().join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        let dsh_log = log_dir.join("dsh.log");
        let harness_log = home.join(".dsh").join("harness.out.log");
        std::fs::create_dir_all(home.join(".dsh")).unwrap();

        // harness.out.log（launchd 常驻）最后一行带 token，dsh.log（壳自拉）无 token
        std::fs::write(
            &harness_log,
            "dsh web: http://127.0.0.1:3080\n\ndsh web: http://127.0.0.1:3080/?token=abc123\n",
        )
        .unwrap();
        std::fs::write(&dsh_log, "dsh web: http://127.0.0.1:3080\n").unwrap();

        let url = launch_token_url(&cfg).unwrap();
        assert_eq!(url, "http://127.0.0.1:3080/?token=abc123");
    }

    #[test]
    fn launch_token_url_returns_none_when_no_match() {
        let tmp = std::env::temp_dir().join("dsh-launch-token-test-nomatch");
        std::env::set_var("HOME", &tmp);
        std::env::set_var("DSH_DESKTOP_CONFIG_DIR", tmp.join("cfg"));

        let mut cfg = DshConfig::default();
        cfg.port = 3999;
        std::fs::create_dir_all(config_dir().join("logs")).unwrap();
        std::fs::create_dir_all(dirs::home_dir().unwrap().join(".dsh")).unwrap();
        assert_eq!(launch_token_url(&cfg), None);
    }
}