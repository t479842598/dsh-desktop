use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
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
        // 端口占用自愈：若目标端口已被占用（通常是上次退出残留的孤儿 dsh），
        // 杀掉占用该端口的 node/tsx 进程后重试，避免 EADDRINUSE 导致服务起不来
        if port_open(cfg.port) {
            log::warn!("[dsh] 端口 {} 已被占用，尝试清理残留进程", cfg.port);
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

        // 通过 /bin/sh 做 chdir 再 exec：绕开 Rust Command::current_dir 在 GUI 上下文
        // 偶发的 chdir 失效问题（观察到子进程 cwd 落在 / 导致 node getcwd 卡死）
        let mut command = Command::new("/bin/sh");
        let mut shell_cmd = String::from("cd ");
        shell_cmd.push_str(&shell_quote(&cfg.cwd));
        shell_cmd.push_str(" && exec");
        // 只拼一次完整命令（cfg.command 已含全部参数；args 是其子集，勿重复）
        for part in &cfg.command {
            shell_cmd.push(' ');
            shell_cmd.push_str(&shell_quote(part));
        }
        command
            .args(["-c", &shell_cmd])
            .stdout(Stdio::from(f.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(f));
        // 关键：把 shell 也放到独立进程组并脱离 GUI 会话（setsid 语义，仅 Unix），
        // 避免继承 app 的 Mach/GUI 会话上下文导致 node 启动 getcwd 挂起
        #[cfg(unix)]
        command.process_group(0);
        // GUI 应用从 Finder 启动时 PATH 精简，补全常见 Node 工具链路径
        let mut path = std::env::var("PATH").unwrap_or_default();
        for extra in [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/local/opt/node/bin",
            "/Users/qingtang/.fnm/current/bin",
        ] {
            if !path.split(':').any(|p| p == extra) {
                path.push(':');
                path.push_str(extra);
            }
        }
        command.env("PATH", path);
        // 清除 GUI 会话继承变量，避免 node 启动时与 WindowServer/LaunchServices 交互阻塞
        for var in [
            "__CFBundleIdentifier",
            "__CF_USER_TEXT_ENCODING",
            "XPC_FLAGS",
            "XPC_SERVICE_NAME",
        ] {
            command.env_remove(var);
        }
        // 显式设置 HOME 与 LANG，保证 node/tsx 行为确定
        if let Ok(home) = std::env::var("HOME") {
            command.env("HOME", home);
        }
        command.env("LANG", "en_US.UTF-8");
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
                    // 子进程已退出
                    self.ready = false;
                    self.child = None;
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

/// 单引号转义 shell 参数（用于 /bin/sh -c 拼接）
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\\\''"))
}

/// 杀掉占用指定端口的进程（仅限 node/tsx 类，即 dsh 运行进程；避免误杀用户其它程序）
fn kill_port_owner(port: u16) {
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

/// 检查端口是否可连接
pub fn port_open(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(300),
    )
    .is_ok()
}

/// 全局 dsh 进程句柄（Arc 便于跨线程克隆）
pub struct DshHandle(pub Arc<Mutex<DshProcess>>);

impl DshHandle {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(DshProcess::new())))
    }
}
