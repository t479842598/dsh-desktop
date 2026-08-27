//! 注入到 dsh Web UI 页面的壳层 UI（单窗口方案）：
//! 主窗口直接加载 dsh 页面（本地 3080 / 远程 URL）。此脚本在**顶部中间**注入
//! 一个"灵动岛"式悬浮手柄：
//! - 平时收成小胶囊（鼠标移开自动收起），鼠标靠近自动展开完整工具条
//! - 展开后可见：拖动手柄、品牌、模式标签、"设置"按钮、窗口控制（缩小/放大/关闭）
//! - 按住最左侧"⠿"手柄拖动 = 移动工具条位置（记住位置，刷新后保持）；
//!   按住工具条背景（非按钮区域）拖动 = 拖动整个窗口（调用 Rust start_dragging）
//! - 点击"设置"在岛下方展开设置面板（连接模式 + 远程凭证 + 保存）
//! 右键菜单桥接与启动 splash 同样由本脚本提供。
//! 所有交互通过 `__TAURI_INTERNALS__.invoke` 与 Rust 通信。

/// 注入脚本（对主窗口每次顶层导航生效）
pub const SHELL_SCRIPT: &str = r#"
(function () {
  var invoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke
  if (!invoke) return
  if (window.__dsh_shell_injected__) return
  // document start 注入时 documentElement 尚未创建（html 元素不存在），
  // appendChild 会抛 TypeError；必须等 DOMContentLoaded 后再创建 UI，
  // 否则幂等标志残留且元素未建，on_page_load 重试也被挡。
  function boot() {
    if (window.__dsh_shell_injected__) return
    window.__dsh_shell_injected__ = true

  // 启动页（index.html，tauri:// 或 http://localhost:1420）已自带 UI，
  // 壳层只注入 dsh 页面（3080 本地或远程 URL）
  var isDshPage = location.origin.indexOf('127.0.0.1') !== -1
    || location.origin.indexOf('localhost') !== -1 && location.port !== '1420'
    || /^https:/.test(location.protocol) && location.origin.indexOf('localhost') === -1

  var WHALE = 'M48.8354 10.0479C48.3232 9.79199 48.1025 10.2798 47.8032 10.5278C47.7007 10.6079 47.6143 10.7119 47.5273 10.8076C46.7793 11.624 45.9048 12.1597 44.7622 12.0957C43.0923 12 41.666 12.5356 40.4058 13.8398C40.1377 12.2319 39.2476 11.272 37.8926 10.6558C37.1836 10.3359 36.4668 10.0156 35.9702 9.31982C35.6235 8.82373 35.5293 8.27197 35.356 7.72754C35.2456 7.3999 35.1353 7.06396 34.7651 7.00781C34.3633 6.94385 34.2056 7.2876 34.0479 7.57568C33.418 8.75195 33.1733 10.0479 33.1973 11.3599C33.2524 14.312 34.4736 16.6641 36.8999 18.3359C37.1758 18.5278 37.2466 18.7197 37.1597 19C36.9946 19.5757 36.7974 20.1357 36.624 20.7119C36.5137 21.0801 36.3486 21.1597 35.9624 21C34.6309 20.4321 33.481 19.5918 32.4644 18.5757C30.7393 16.8721 29.1792 14.9917 27.2334 13.52C26.7764 13.1758 26.3193 12.856 25.8467 12.5518C23.8618 10.584 26.1069 8.96777 26.627 8.77588C27.1704 8.57568 26.8159 7.8877 25.0591 7.896C23.3022 7.90381 21.6953 8.50391 19.647 9.30371C19.3477 9.42383 19.0322 9.51172 18.7095 9.58398C16.8501 9.22363 14.9199 9.14355 12.9033 9.37598C9.10596 9.80762 6.07275 11.6396 3.84326 14.7681C1.16455 18.5278 0.53418 22.7998 1.30664 27.2559C2.11768 31.9521 4.46582 35.8398 8.07373 38.8799C11.8159 42.0322 16.1255 43.5762 21.041 43.2803C24.0269 43.104 27.3516 42.6963 31.1016 39.4561C32.0469 39.936 33.0396 40.1279 34.686 40.272C35.9546 40.3921 37.1758 40.208 38.1211 40.0078C39.6021 39.688 39.4995 38.2881 38.9639 38.0322C34.623 35.9678 35.5762 36.8081 34.71 36.1279C36.9155 33.4639 40.2402 30.6958 41.54 21.728C41.6426 21.0161 41.5557 20.5679 41.54 19.9917C41.5322 19.6396 41.6108 19.5039 42.0049 19.4639C43.0923 19.3359 44.1479 19.0317 45.1167 18.4878C47.9292 16.9199 49.064 14.3438 49.3315 11.2559C49.3711 10.7837 49.3237 10.2959 48.8354 10.0479ZM24.3262 37.8398C20.1196 34.4639 18.0791 33.3521 17.2358 33.3999C16.4482 33.4482 16.5898 34.3682 16.7632 34.9678C16.9443 35.5601 17.1812 35.9683 17.5117 36.4878C17.7402 36.832 17.8979 37.3442 17.2832 37.728C15.9282 38.584 13.5728 37.4399 13.4624 37.3838C10.7207 35.7358 8.42822 33.5601 6.81348 30.584C5.25342 27.7197 4.34766 24.6479 4.19775 21.3677C4.1582 20.5757 4.38672 20.2959 5.15869 20.1519C6.17529 19.96 7.22314 19.9199 8.23926 20.0718C12.5327 20.7119 16.1885 22.6719 19.2529 25.7759C21.002 27.5439 22.3252 29.6558 23.6885 31.7202C25.1377 33.9121 26.6978 36 28.6831 37.7119C29.3843 38.312 29.9434 38.7681 30.479 39.104C28.8643 39.2881 26.1699 39.3281 24.3262 37.8398ZM26.3433 24.6001C26.3433 24.248 26.6191 23.9678 26.9658 23.9678C27.0444 23.9678 27.1152 23.9839 27.1782 24.0078C27.2651 24.04 27.3438 24.0879 27.4067 24.1602C27.5171 24.272 27.5801 24.4321 27.5801 24.6001C27.5801 24.9521 27.3042 25.2319 26.9575 25.2319C26.6108 25.2319 26.3433 24.9521 26.3433 24.6001ZM32.6064 27.8799C32.2046 28.0479 31.8027 28.1919 31.4165 28.208C30.8179 28.2397 30.1641 27.9922 29.8096 27.688C29.2583 27.2158 28.8643 26.9521 28.6987 26.1279C28.6279 25.7759 28.6675 25.2319 28.7305 24.9199C28.8721 24.248 28.7144 23.8159 28.2495 23.4238C27.8716 23.104 27.3911 23.0161 26.8633 23.0161C26.666 23.0161 26.4849 22.9277 26.3511 22.856C26.1304 22.7441 25.9492 22.4639 26.1226 22.1201C26.1777 22.0078 26.4458 21.7358 26.5088 21.688C27.2256 21.272 28.0527 21.4077 28.8169 21.7197C29.5259 22.0161 30.0615 22.5601 30.834 23.3281C31.6216 24.2559 31.7632 24.5117 32.2124 25.208C32.5669 25.752 32.8901 26.312 33.1104 26.9521C33.2446 27.3521 33.0713 27.6802 32.6064 27.8799Z'

  var css = `
  /* ---- 灵动岛（悬浮于顶部 70%：避开 dsh 顶部中间与右上角的按钮，平时隐藏、靠近浮现）---- */
  #dsh-island { position: fixed; top: 8px; left: 70%; transform: translateX(-50%);
    z-index: 2147483005; display: flex; align-items: center; gap: 10px;
    height: 36px; padding: 0 16px; border-radius: 18px; box-sizing: border-box;
    background: rgba(23, 26, 33, 0.95); border: 1px solid rgba(76, 141, 255, 0.5);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.45);
    font: 13px/1.4 -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
    color: #e6e9ef; user-select: none; cursor: grab;
    opacity: 0; pointer-events: none;
    transition: opacity 0.16s ease; }
  #dsh-island.visible { opacity: 1; pointer-events: auto; }
  #dsh-island.dragging { cursor: grabbing; opacity: 0.85; }
  #dsh-island .brand svg { width: 17px; height: 17px; fill: #5673ff; display: block; }
  #dsh-island .brand { display: flex; align-items: center; gap: 7px; }
  #dsh-island .brand b { font-weight: 700; font-size: 13px; white-space: nowrap; }
  #dsh-island .mode-tag { font-size: 11px; color: #8b93a3; white-space: nowrap;
    background: rgba(76, 141, 255, 0.14); border-radius: 5px; padding: 1px 7px; }
  #dsh-island .sep { width: 1px; height: 16px; background: rgba(255, 255, 255, 0.14); }
  #dsh-island button { background: transparent; border: 0; color: #8b93a3; cursor: pointer;
    font-size: 12px; padding: 4px 8px; border-radius: 6px; line-height: 1; white-space: nowrap; }
  #dsh-island button:hover { background: rgba(255, 255, 255, 0.1); color: #e6e9ef; }
  #dsh-island .win-btn { font-size: 14px; padding: 3px 7px; }
  #dsh-island .grip { cursor: grab; }
  #dsh-island .grip:active { cursor: grabbing; }
  #dsh-island .quit-btn { color: #f85149; }
  #dsh-island .quit-btn:hover { background: rgba(248, 81, 73, 0.15); color: #f85149; }

  /* ---- 设置面板：从灵动岛下方展开 ---- */
  #dsh-settings { position: fixed; top: 54px; left: 70%; transform: translateX(-50%);
    width: 340px; z-index: 2147483006;
    background: #171a21; border: 1px solid #262b36; border-radius: 12px;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.55); padding: 16px; box-sizing: border-box;
    font: 13px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
    color: #e6e9ef; display: none; }
  #dsh-settings.open { display: block; }
  #dsh-settings h3 { margin: 0 0 10px; font-size: 14px; display: flex; justify-content: space-between; align-items: center; }
  #dsh-settings h3 .x { cursor: pointer; color: #8b93a3; font-size: 15px; padding: 0 4px; }
  #dsh-settings h3 .x:hover { color: #e6e9ef; }
  #dsh-settings .hint { color: #8b93a3; font-size: 11.5px; line-height: 1.6; margin: 0 0 12px; }
  #dsh-settings .row { display: flex; align-items: center; gap: 8px; margin: 7px 0; cursor: pointer; }
  #dsh-settings .row input { accent-color: #4c8dff; }
  #dsh-settings .field { display: flex; flex-direction: column; gap: 5px; margin: 10px 0; }
  #dsh-settings .field span { color: #8b93a3; font-size: 11.5px; }
  #dsh-settings input[type="text"], #dsh-settings input[type="password"] {
    background: #0f1115; border: 1px solid #262b36; border-radius: 6px; color: #e6e9ef;
    padding: 7px 9px; font-size: 12.5px; outline: none; }
  #dsh-settings input:focus { border-color: #4c8dff; }
  #dsh-settings .actions { display: flex; align-items: center; gap: 8px; margin-top: 14px; flex-wrap: wrap; }
  #dsh-settings .save { background: #4c8dff; border: 0; color: #fff; padding: 7px 16px;
    border-radius: 6px; cursor: pointer; font-size: 12.5px; }
  #dsh-settings .save:hover { filter: brightness(1.15); }
  #dsh-settings .ctrl { background: transparent; border: 1px solid #3a4250; color: #aeb6c2;
    padding: 6px 11px; border-radius: 6px; cursor: pointer; font-size: 12px; }
  #dsh-settings .ctrl:hover { background: rgba(255, 255, 255, 0.08); color: #e6e9ef; }
  #dsh-settings .ctrl.quit { border-color: #f85149; color: #f85149; }
  #dsh-settings .ctrl.quit:hover { background: rgba(248, 81, 73, 0.12); }
  #dsh-settings .save-result { font-size: 11.5px; color: #3fb950; width: 100%; }
  #dsh-settings .save-result.err { color: #f85149; }

  /* ---- 启动 splash ---- */
  #dsh-splash { position: fixed; inset: 0; z-index: 2147483100; display: flex;
    flex-direction: column; align-items: center; justify-content: center; gap: 16px;
    background: #0f1115; transition: opacity 0.4s ease; font: 13px/1.5 -apple-system, sans-serif; }
  #dsh-splash.fade { opacity: 0; pointer-events: none; }
  #dsh-splash.hide { display: none; }
  #dsh-splash .wordmark { position: relative; display: flex; align-items: center; gap: 13px;
    font-size: 30px; letter-spacing: 0.5px; overflow: hidden; padding: 8px 14px; }
  #dsh-splash .wordmark svg { width: 36px; height: 36px; fill: #5673ff; flex: none; }
  #dsh-splash .wordmark b { font-weight: 800; font-size: 31px; color: #e6e9ef; letter-spacing: -0.3px; }
  #dsh-splash .wordmark i { font-style: normal; color: #8b93a3; font-weight: 500; font-size: 17px; letter-spacing: 1px; }
  #dsh-splash .band { position: absolute; inset: 0; pointer-events: none;
    background: linear-gradient(100deg, transparent 20%, rgba(255,255,255,0.16) 50%, transparent 80%);
    transform: translateX(-130%); animation: dsh-band-sweep 1.7s ease-in-out infinite; }
  @keyframes dsh-band-sweep {
    0% { transform: translateX(-130%); }
    55% { transform: translateX(130%); }
    100% { transform: translateX(130%); }
  }
  #dsh-splash .hint { color: #8b93a3; font-size: 12.5px; letter-spacing: 0.3px; }
  @keyframes dsh-dots { 0% { content: ""; } 25% { content: "."; } 50% { content: ".."; } 75% { content: "..."; } }
  #dsh-splash .hint::after { content: ""; animation: dsh-dots 1.4s steps(4, end) infinite; }
  `;
  var style = document.createElement('style')
  style.textContent = css
  ;(document.head || document.documentElement).appendChild(style)

  // ---- 启动 splash（dsh 页面加载时出现；启动页 index.html 自带 splash，跳过）----
  if (isDshPage) {
  var splash = document.createElement('div')
  splash.id = 'dsh-splash'
  splash.innerHTML = '<div class="wordmark"><svg viewBox="0 0 50 50" xmlns="http://www.w3.org/2000/svg"><path d="' + WHALE + '"/></svg><b>DeepSeek</b><i>desktop</i><div class="band"></div></div><div class="hint">正在启动服务</div>'
  document.documentElement.appendChild(splash)
  setTimeout(function () {
    splash.classList.add('fade')
    setTimeout(function () { splash.classList.add('hide') }, 450)
  }, 2200)
  } // end if (isDshPage) splash

  // ---- 灵动岛：顶部中间悬浮手柄 + 工具条 ----
  if (isDshPage) {
  var island = document.createElement('div')
  island.id = 'dsh-island'
  island.innerHTML =
    '<button class="win-btn grip" id="dsh-btn-drag" title="按住拖动位置">⠿</button>' +
    '<span class="brand"><svg viewBox="0 0 50 50" xmlns="http://www.w3.org/2000/svg"><path d="' + WHALE + '"/></svg><b>DeepSeek</b></span>' +
    '<span class="mode-tag" id="dsh-island-mode">本地</span>' +
    '<span class="sep"></span>' +
    '<button id="dsh-btn-settings" title="设置">设置</button>' +
    '<span class="sep"></span>' +
    '<button class="win-btn" id="dsh-btn-new" title="新建窗口">⧉</button>' +
    '<button class="win-btn" id="dsh-btn-reload" title="刷新">⟳</button>' +
    '<button class="win-btn" id="dsh-btn-min" title="缩小">─</button>' +
    '<button class="win-btn" id="dsh-btn-max" title="放大">▢</button>' +
    '<button class="win-btn quit-btn" id="dsh-btn-close" title="关闭当前窗口">✕</button>'
  document.documentElement.appendChild(island)

  // ---- 设置面板（从灵动岛下方展开）----
  var panel = document.createElement('div')
  panel.id = 'dsh-settings'
  panel.innerHTML =
    '<h3>连接设置 <span class="x" id="dsh-close-panel">✕</span></h3>' +
    '<p class="hint">本地模式：启动本机 dsh 服务（127.0.0.1:3080）。<br>远程模式：连接你配置的远程 dsh 实例（不启动本机 dsh）。</p>' +
    '<label class="row"><input type="radio" name="dsh-conn-mode" value="local" checked> 本地模式（默认）</label>' +
    '<label class="row"><input type="radio" name="dsh-conn-mode" value="remote"> 远程模式</label>' +
    '<div id="dsh-remote-fields">' +
    '  <div class="field"><span>远程 URL</span><input type="text" id="dsh-remote-url" placeholder="https://…"></div>' +
    '  <div class="field"><span>账号</span><input type="text" id="dsh-remote-user" placeholder="username"></div>' +
    '  <div class="field"><span>密码</span><input type="password" id="dsh-remote-pass" placeholder="password"></div>' +
    '</div>' +
    '<div class="actions">' +
    '  <button class="save" id="dsh-btn-save">保存并应用</button>' +
    '  <span class="save-result" id="dsh-save-result"></span>' +
    '</div>' +
    '<div class="actions">' +
    '  <button class="ctrl" id="dsh-btn-min2">缩小窗口</button>' +
    '  <button class="ctrl" id="dsh-btn-max2">放大/还原</button>' +
    '  <button class="ctrl quit" id="dsh-btn-quit2">退出应用</button>' +
    '</div>'
  document.documentElement.appendChild(panel)

  var modeTag = document.getElementById('dsh-island-mode')
  var remoteFields = document.getElementById('dsh-remote-fields')

  // 读取当前配置填充表单：on_page_load 已注入 window.__dsh_cfg（自定义 command 远程 origin 被 ACL 拒）。
  // 注入时机是 document start（此时 __dsh_cfg 尚未注入，SHELL_SCRIPT 又会被幂等 guard 挡住重入），
  // 因此把“应用配置”挂到全局，on_page_load 注入 cfg 后主动调用，保证模式标签/表单与配置同步。
  function setModeUI(mode) {
    modeTag.textContent = mode === 'remote' ? '远程' : '本地'
    var radios = document.querySelectorAll('input[name="dsh-conn-mode"]')
    for (var i = 0; i < radios.length; i++) radios[i].checked = (radios[i].value === mode)
    remoteFields.style.display = mode === 'remote' ? '' : 'none'
  }
  // 用户手动切换连接模式：实时更新模式标签与远程字段显示
  var connRadios = document.querySelectorAll('input[name="dsh-conn-mode"]')
  for (var i = 0; i < connRadios.length; i++) {
    connRadios[i].addEventListener('change', function () {
      setModeUI(this.value)
    })
  }
  window.__dsh_apply_cfg = function (cfg) {
    if (!cfg) return
    if (cfg.mode) setModeUI(cfg.mode)
    if (cfg.remoteUrl) document.getElementById('dsh-remote-url').value = cfg.remoteUrl
    if (cfg.remoteUser) document.getElementById('dsh-remote-user').value = cfg.remoteUser
  }
  try {
    if (window.__dsh_cfg) window.__dsh_apply_cfg(window.__dsh_cfg)
  } catch (e) {}

  // ---- 灵动岛：平时隐藏，鼠标靠近窗口顶部自动浮现，移开自动收起；长按拖动窗口 ----
  var hideTimer = null
  var inTopZone = false

  function showIsland() {
    if (hideTimer) { clearTimeout(hideTimer); hideTimer = null }
    island.classList.add('visible')
  }
  function scheduleHide() {
    if (hideTimer) clearTimeout(hideTimer)
    hideTimer = setTimeout(function () {
      // 设置面板开着则不收起
      if (panel.classList.contains('open')) return
      island.classList.remove('visible')
    }, 400)
  }
  // 鼠标进入窗口顶部 60px 区域 → 浮现；移出 → 收起（不创建覆盖 div，不挡点击）
  window.addEventListener('mousemove', function (e) {
    var topZone = e.clientY < 60
    if (topZone && !inTopZone) {
      inTopZone = true
      showIsland()
    } else if (!topZone && inTopZone) {
      inTopZone = false
      scheduleHide()
    }
  })
  island.addEventListener('mouseenter', function () { if (hideTimer) { clearTimeout(hideTimer); hideTimer = null } })
  island.addEventListener('mouseleave', scheduleHide)

  // ---- 按钮动作：窗口控制走 core:window 内置命令（capability 已授权），
  //      退出/保存走自定义事件（Rust 侧监听，规避远程 origin 的 command ACL）----
  function emit(event, payload) {
    invoke('plugin:event|emit', { event: event, payload: payload || {} }).catch(function () {})
  }

  document.getElementById('dsh-btn-settings').addEventListener('click', function () {
    panel.classList.toggle('open')
    // 面板与工具条同左对齐（工具条可能被拖到别处）
    if (panel.classList.contains('open')) panel.style.left = island.offsetLeft + 'px'
  })
  document.getElementById('dsh-close-panel').addEventListener('click', function () {
    panel.classList.remove('open')
  })
  // 新建窗口：当前模式下再开一个 dsh Web UI 窗口（多窗口）
  document.getElementById('dsh-btn-new').addEventListener('click', function () { emit('dsh-new-window') })
  // 刷新：重载当前页面（本地/远程页面通用，页面级操作无需桥接）
  document.getElementById('dsh-btn-reload').addEventListener('click', function () { location.reload() })
  document.getElementById('dsh-btn-min').addEventListener('click', function () { invoke('plugin:window|minimize').catch(function () {}) })
  document.getElementById('dsh-btn-min2').addEventListener('click', function () { invoke('plugin:window|minimize').catch(function () {}) })
  document.getElementById('dsh-btn-max').addEventListener('click', function () { invoke('plugin:window|toggle_maximize').catch(function () {}) })
  document.getElementById('dsh-btn-max2').addEventListener('click', function () { invoke('plugin:window|toggle_maximize').catch(function () {}) })
  // ✕ 关闭当前窗口（main 窗口隐藏到托盘，其他窗口直接关闭）；退出应用走设置面板/托盘菜单
  document.getElementById('dsh-btn-close').addEventListener('click', function () { invoke('plugin:window|close').catch(function () {}) })
  document.getElementById('dsh-btn-quit2').addEventListener('click', function () { emit('dsh-quit') })

  // ---- 拖动：⠿ 手柄 = 移动工具条位置（记住位置，刷新后保持）；背景（非按钮区域）= 拖动整个窗口 ----
  var handle = document.getElementById('dsh-btn-drag')
  // 恢复上次保存的位置（localStorage 存的是可视左边缘，left 需加回半宽抵消 translateX(-50%)）
  try {
    var savedLeft = localStorage.getItem('dsh-island-left')
    if (savedLeft !== null) {
      requestAnimationFrame(function () {
        island.style.left = (Number(savedLeft) + island.offsetWidth / 2) + 'px'
      })
    }
  } catch (e) {}
  handle.addEventListener('mousedown', function (e) {
    if (e.button !== 0) return
    e.preventDefault()
    e.stopPropagation()
    if (hideTimer) { clearTimeout(hideTimer); hideTimer = null }
    island.classList.add('dragging')
    var startX = e.clientX
    var startLeft = island.offsetLeft
    var w = island.offsetWidth
    function move(ev) {
      var visual = startLeft + (ev.clientX - startX) - w / 2
      visual = Math.max(100, Math.min(window.innerWidth - w - 8, visual))
      island.style.left = (visual + w / 2) + 'px'
      if (panel.classList.contains('open')) panel.style.left = island.offsetLeft + 'px'
    }
    function up() {
      window.removeEventListener('mousemove', move)
      window.removeEventListener('mouseup', up)
      island.classList.remove('dragging')
      try { localStorage.setItem('dsh-island-left', String(island.offsetLeft - island.offsetWidth / 2)) } catch (e) {}
    }
    window.addEventListener('mousemove', move)
    window.addEventListener('mouseup', up)
  })
  // 工具条背景按住即拖动整个窗口（start_dragging 由系统接管）；按钮区域保持正常点击
  island.addEventListener('mousedown', function (e) {
    if (e.button !== 0) return
    if (e.target.closest && e.target.closest('button')) return
    e.preventDefault()
    island.classList.add('dragging')
    invoke('plugin:window|start_dragging').catch(function () {})
  })
  window.addEventListener('mouseup', function () {
    island.classList.remove('dragging')
  })

  document.getElementById('dsh-btn-save').addEventListener('click', function () {
    var mode = (document.querySelector('input[name="dsh-conn-mode"]:checked') || { value: 'local' }).value
    var url = document.getElementById('dsh-remote-url').value.trim()
    var user = document.getElementById('dsh-remote-user').value.trim()
    var pass = document.getElementById('dsh-remote-pass').value
    var result = document.getElementById('dsh-save-result')
    emit('dsh-save-connection', {
      mode: mode, remoteUrl: url, remoteUsername: user, remotePassword: pass
    })
    result.textContent = '✓ 已保存，正在切换…'
    result.className = 'save-result'
    // Rust 侧保存后会自动导航主窗口到新目标（本地 3080 / 远程 URL），无需前端 reload
    setTimeout(function () { panel.classList.remove('open') }, 800)
  })

  // 点击面板外收起
  document.addEventListener('click', function (e) {
    if (!panel.contains(e.target) && !island.contains(e.target)) {
      panel.classList.remove('open')
    }
  })
  } // end if (isDshPage) 灵动岛

  // ---- 右键菜单桥接（仅 dsh 页面；启动页由 main.ts 处理）----
  // 自定义 command 在远程 origin 被 ACL 拒绝，改为 emit 事件由 Rust 监听弹菜单
  if (isDshPage) {
  window.addEventListener('contextmenu', function (e) {
    var t = e.target
    if (!t || !t.closest) return
    var isEditable = t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable === true
    var selection = window.getSelection ? (window.getSelection().toString() || '') : ''
    var a = t.closest('a')
    var img = t.closest('img')
    e.preventDefault()
    var inv = window.__TAURI_INTERNALS__
    if (inv && inv.invoke) {
      inv.invoke('plugin:event|emit', {
        event: 'dsh-show-context-menu',
        payload: {
          isEditable: !!isEditable,
          selectionText: selection,
          linkURL: a ? (a.href || '') : '',
          imageURL: img ? (img.src || '') : '',
          x: e.clientX,
          y: e.clientY
        }
      }).catch(function () {})
    }
  }, true)
  } // end if (isDshPage) 右键
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot)
  } else {
    boot()
  }
})()
"#;
