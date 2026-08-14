import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface DshStatus {
  dsh_ready: boolean;
  dsh_error: string | null;
  dsh_port: number;
  poll_interval_sec: number;
  notify_enabled: boolean;
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

async function refreshStatus() {
  try {
    const st: DshStatus = await invoke("get_status");
    const line = el("dsh-status");
    if (st.dsh_ready) {
      line.className = "status-line ok";
      line.textContent = `运行中 · http://127.0.0.1:${st.dsh_port}`;
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
  } catch (e) {
    el("dsh-status").className = "status-line err";
    el("dsh-status").textContent = `查询状态失败: ${e}`;
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

function setupTabs() {
  const status = el("tab-status");
  status.addEventListener("click", () => {
    status.classList.add("active");
    el("panel-status").classList.remove("hidden");
    refreshStatus();
    refreshWatcher();
  });
}

async function init() {
  setupTabs();

  el("btn-open-dsh").addEventListener("click", async () => {
    try {
      const msg: string = await invoke("open_dsh_ui");
      toast(msg);
    } catch (e) {
      toast(`打开失败: ${e}`);
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

  // 事件监听
  await listen("watcher-poll", () => refreshWatcher());
  await listen("dsh-ready", () => {
    refreshStatus();
  });
  await listen("dsh-crashed", () => refreshStatus());

  refreshStatus();
  refreshWatcher();
}

init();
