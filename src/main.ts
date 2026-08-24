import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface DshStatus {
  dsh_ready: boolean;
  dsh_error: string | null;
  dsh_port: number;
  poll_interval_sec: number;
  notify_enabled: boolean;
  connection_mode: string;
  remote_url: string;
  remote_username: string;
  repos: Array<{
    name: string;
    local_path: string;
    remote: string;
    branch: string;
    auto_pull: string;
  }>;
}

interface PollResult {
  repo: string;
  updated: boolean;
  message: string;
}

interface WatcherStatus {
  running: boolean;
  last_poll_at: string | null;
  last_results: PollResult[];
}

function el<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

function toast(msg: string) {
  const t = el<HTMLDivElement>("toast");
  if (!t) return;
  t.textContent = msg;
  t.classList.add("show");
  setTimeout(() => t.classList.remove("show"), 4000);
}

/** 淡出启动画面（desktop-polish F-03） */
function hideSplash() {
  const s = el<HTMLDivElement>("splash");
  if (!s || s.classList.contains("hidden")) return;
  s.classList.add("hidden");
  setTimeout(() => s.remove(), 500);
}

let currentStatus: DshStatus | null = null;

async function refreshStatus() {
  try {
    const st: DshStatus = await invoke("get_status");
    currentStatus = st;
    // 就绪后淡出启动画面
    if (st.dsh_ready || st.connection_mode === "remote") hideSplash();
    const line = el("dsh-status");
    const modeTag = st.connection_mode === "remote" ? "· 远程模式" : "";
    if (st.dsh_ready) {
      line.className = "status-line ok";
      line.textContent = `运行中 · http://127.0.0.1:${st.dsh_port}${modeTag}`;
    } else if (st.connection_mode === "remote") {
      line.className = "status-line ok";
      line.textContent = `远程模式 · ${st.remote_url || "未配置 URL"}${modeTag}`;
    } else {
      line.className = "status-line err";
      line.textContent = `未就绪：${st.dsh_error ?? "启动中…"}`;
    }

    const repos = el("repos-list");
    repos.textContent = st.repos
      .map(
        (r) =>
          `${r.name}\n  路径: ${r.local_path}\n  remote: ${r.remote} / ${r.branch} / 拉取: ${r.auto_pull}`,
      )
      .join("\n");

    // 同步设置页表单（仅在用户未编辑时）
    syncSettingsForm(st);
    updateConnectPanel(st);
  } catch (e) {
    el("dsh-status").className = "status-line err";
    el("dsh-status").textContent = `查询状态失败: ${e}`;
  }
}

/** 本地 3080 未就绪时展示「连接其他地址」卡片；就绪后隐藏 */
function updateConnectPanel(st: DshStatus) {
  const panel = el<HTMLElement>("panel-connect");
  if (!panel) return;
  if (!st.dsh_ready) {
    panel.classList.remove("hidden");
    // 预填上次保存的远程地址作为备选（仅预填，绝不自动连接）
    const urlInput = el<HTMLInputElement>("connect-url");
    const userInput = el<HTMLInputElement>("connect-username");
    if (urlInput.value === "") urlInput.value = st.remote_url ?? "";
    if (userInput.value === "") userInput.value = st.remote_username ?? "";
  } else {
    panel.classList.add("hidden");
  }
}

async function refreshWatcher() {
  try {
    const st: WatcherStatus = await invoke("watcher_status");
    el("watcher-status").textContent = st.last_poll_at
      ? `上次检查: ${st.last_poll_at}`
      : "尚未检查";
    const box = el("watcher-results");
    box.innerHTML = "";
    for (const r of st.last_results ?? []) {
      const div = document.createElement("div");
      div.className =
        "result-item" + (r.updated ? " updated" : r.message.includes("失败") ? " error" : "");
      div.textContent = `[${r.repo}] ${r.message}`;
      box.appendChild(div);
    }
  } catch {
    /* 忽略 */
  }
}

function syncSettingsForm(st: DshStatus) {
  const mode = st.connection_mode === "remote" ? "remote" : "local";
  const radios = document.querySelectorAll<HTMLInputElement>('input[name="conn-mode"]');
  for (const r of radios) r.checked = r.value === mode;
  toggleRemoteFields(mode === "remote");
  const urlInput = el<HTMLInputElement>("remote-url");
  const userInput = el<HTMLInputElement>("remote-username");
  const passInput = el<HTMLInputElement>("remote-password");
  if (document.activeElement !== urlInput) urlInput.value = st.remote_url ?? "";
  if (document.activeElement !== userInput) userInput.value = st.remote_username ?? "";
  // 密码不回显（避免明文泄漏到 UI 状态），留空表示保持原值
  if (passInput.value === "" && document.activeElement !== passInput) passInput.placeholder = "••••••••";
}

function toggleRemoteFields(show: boolean) {
  el("remote-fields").classList.toggle("hidden", !show);
}

function setupTabs() {
  const status = el("tab-status");
  const settings = el("tab-settings");
  const showPanel = (active: HTMLElement, panel: HTMLElement) => {
    document
      .querySelectorAll(".tab")
      .forEach((t) => t.classList.remove("active"));
    document
      .querySelectorAll(".panel")
      .forEach((p) => p.classList.add("hidden"));
    active.classList.add("active");
    panel.classList.remove("hidden");
    if (panel.id === "panel-settings" && currentStatus) syncSettingsForm(currentStatus);
  };
  status.addEventListener("click", () => {
    showPanel(status, el("panel-status"));
    refreshStatus();
    refreshWatcher();
  });
  settings.addEventListener("click", () => {
    showPanel(settings, el("panel-settings"));
    refreshStatus();
  });
}

async function saveConnection() {
  const mode = (
    document.querySelector<HTMLInputElement>('input[name="conn-mode"]:checked') ?? { value: "local" }
  ).value;
  const url = el<HTMLInputElement>("remote-url").value.trim();
  const username = el<HTMLInputElement>("remote-username").value.trim();
  const password = el<HTMLInputElement>("remote-password").value;
  try {
    const msg: string = await invoke("save_connection", {
      mode,
      remoteUrl: url,
      remoteUsername: username,
      remotePassword: password,
    });
    el("conn-save-result").textContent = "✓ " + msg;
    setTimeout(() => (el("conn-save-result").textContent = ""), 4000);
    toast(msg);
    // 模式切换后的进程编排（切远程→停本地 dsh；切本地→重启），由 Rust 统一处理
    refreshStatus();
    await invoke("apply_connection_mode").catch(() => {});
  } catch (e) {
    el("conn-save-result").textContent = `✗ ${e}`;
    toast(`保存失败: ${e}`);
  }
}

/** 右键菜单桥接：收集目标信息并交给 Rust 弹原生菜单（desktop-polish F-01） */
function setupContextMenu() {
  window.addEventListener("contextmenu", (e) => {
    const target = e.target as HTMLElement;
    const isEditable =
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target.isContentEditable;
    const selection = window.getSelection()?.toString() ?? "";
    let linkURL = "";
    let imageURL = "";
    const linkEl = target.closest("a");
    if (linkEl) linkURL = (linkEl as HTMLAnchorElement).href;
    const imgEl = target.closest("img");
    if (imgEl) imageURL = (imgEl as HTMLImageElement).src;
    e.preventDefault();
    void invoke("show_context_menu", {
      target: {
        isEditable,
        selectionText: selection,
        linkURL,
        imageURL,
        x: e.clientX,
        y: e.clientY,
      },
    });
  });
}

async function init() {
  setupTabs();
  setupContextMenu();

  el("btn-open-dsh").addEventListener("click", async () => {
    try {
      const msg: string = await invoke("open_dsh_ui");
      toast(msg);
    } catch (e) {
      toast(`打开失败: ${e}`);
    }
  });

  // 「连接其他地址」：本地 3080 不可用时手动连接指定地址（仅本次会话，不写配置）
  el("btn-connect-addr").addEventListener("click", async () => {
    const url = el<HTMLInputElement>("connect-url").value.trim();
    const username = el<HTMLInputElement>("connect-username").value.trim();
    const password = el<HTMLInputElement>("connect-password").value;
    const result = el("connect-result");
    if (!url) {
      result.textContent = "✗ 请输入连接地址";
      result.className = "save-result err";
      return;
    }
    try {
      const msg: string = await invoke("connect_to_address", {
        url,
        username,
        password,
      });
      result.textContent = "✓ " + msg;
      result.className = "save-result";
    } catch (e) {
      result.textContent = `✗ ${e}`;
      result.className = "save-result err";
    }
  });

  // 重试本地 3080：重启本地 dsh，就绪后自动打开 Web UI
  el("btn-retry-local").addEventListener("click", async () => {
    const result = el("connect-result");
    result.textContent = "正在重试本地服务…";
    result.className = "save-result";
    try {
      await invoke("restart_dsh");
      setTimeout(async () => {
        await refreshStatus();
        if (currentStatus?.dsh_ready) {
          await invoke("open_dsh_ui").catch(() => {});
        }
      }, 1500);
    } catch (e) {
      result.textContent = `✗ ${e}`;
      result.className = "save-result err";
    }
  });

  el("btn-restart-dsh").addEventListener("click", async () => {
    try {
      await invoke("restart_dsh");
      toast("dsh 已重启");
      setTimeout(refreshStatus, 2000);
    } catch (e) {
      toast(`重启失败: ${e}`);
    }
  });

  el("btn-poll").addEventListener("click", async () => {
    await invoke("poll_now");
    toast("已触发检查，结果稍后刷新");
    setTimeout(refreshWatcher, 4000);
  });

  // 设置页事件
  document.querySelectorAll<HTMLInputElement>('input[name="conn-mode"]').forEach((r) => {
    r.addEventListener("change", () => toggleRemoteFields(r.value === "remote"));
  });
  el("btn-save-connection").addEventListener("click", saveConnection);
  el("btn-open-config").addEventListener("click", async () => {
    try {
      await invoke("open_config");
    } catch (e) {
      toast(`打开失败: ${e}`);
    }
  });
  el("btn-open-gateway-config").addEventListener("click", async () => {
    try {
      await invoke("open_config");
      toast("网关配置在配置文件的 remote 段，请编辑保存");
    } catch (e) {
      toast(`打开失败: ${e}`);
    }
  });

  // 事件监听
  await listen("watcher-poll", () => refreshWatcher());
  await listen("dsh-ready", () => {
    refreshStatus();
  });
  await listen("dsh-crashed", () => {
    // 本地 dsh 起不来（3080 不可用）时自动切到设置页，方便直接配置/连接远程
    refreshStatus();
    el("tab-settings").click();
  });
  await listen("show-settings", () => {
    el("tab-settings").click();
  });

  refreshStatus();
  refreshWatcher();
}

init();
