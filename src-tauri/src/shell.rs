//! 注入到 dsh Web UI 页面的壳层 UI（单窗口方案）：
//! 主窗口直接加载 dsh 页面（本地 3080 / 远程 URL）。此脚本在**顶部中间**注入
//! 一个"灵动岛"式悬浮手柄：
//! - 平时收成小胶囊（鼠标移开自动收起），鼠标靠近自动展开完整工具条
//! - 展开后可见：品牌、模式标签、"设置"按钮、窗口控制（缩小/放大/关闭）
//! - **长按（按住约 250ms）灵动岛 = 拖动整个窗口**（调用 Rust start_dragging）
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

  var ICON = 'M78.6785 18.6823H77.1078V16.2472H78.6785C79.6513 16.2472 80.6342 16.0047 81.2672 15.3308C81.9009 14.6569 82.14 13.6232 82.14 12.59C82.14 11.5569 81.9109 10.5231 81.2672 9.84981C80.6247 9.17594 79.6513 8.93343 78.6785 8.93343C77.7056 8.93343 76.7228 9.17594 76.0886 9.84981C75.4549 10.5237 75.2158 11.5569 75.2158 12.59V22.5909H72.4605V6.50781H75.2158V7.53204H75.7209C75.7757 7.4689 75.8304 7.41525 75.8857 7.36161C76.5752 6.73244 77.6308 6.50781 78.6684 6.50781C80.2944 6.50781 81.9193 6.91236 82.9849 8.03549C84.0499 9.15861 84.4265 10.8835 84.4265 12.6001C84.4265 14.3166 84.0404 16.0326 82.9849 17.1647C81.9288 18.2967 80.2944 18.6834 78.6785 18.6834V18.6823Z'
  var WORDMARK = 'dsh desktop'

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
  #dsh-island.dragging { cursor: grabbing; opacity: 0.85; }
  #dsh-island .brand svg { width: 17px; height: 17px; fill: #4d6bfe; display: block; }
  #dsh-island .brand { display: flex; align-items: center; gap: 7px; }
  #dsh-island .brand b { font-weight: 700; font-size: 13px; white-space: nowrap; }
  #dsh-island .mode-tag { font-size: 11px; color: #8b93a3; white-space: nowrap;
    background: rgba(76, 141, 255, 0.14); border-radius: 5px; padding: 1px 7px; }
  #dsh-island .sep { width: 1px; height: 16px; background: rgba(255, 255, 255, 0.14); }
  #dsh-island button { background: transparent; border: 0; color: #8b93a3; cursor: pointer;
    font-size: 12px; padding: 4px 8px; border-radius: 6px; line-height: 1; white-space: nowrap; }
  #dsh-island button:hover { background: rgba(255, 255, 255, 0.1); color: #e6e9ef; }
  #dsh-island .win-btn { font-size: 14px; padding: 3px 7px; }
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
  #dsh-splash .wordmark { position: relative; display: flex; align-items: baseline; gap: 9px;
    font-size: 30px; letter-spacing: 0.5px; overflow: hidden; padding: 8px 14px; }
  #dsh-splash .wordmark svg { width: 30px; height: 30px; fill: #4d6bfe; align-self: center; }
  #dsh-splash .wordmark b { font-weight: 800; font-size: 27px; color: #e6e9ef; }
  #dsh-splash .wordmark i { font-style: normal; color: #8b93a3; font-weight: 500; }
  #dsh-splash .band { position: absolute; inset: 0; pointer-events: none;
    background: linear-gradient(100deg, transparent 20%, rgba(255,255,255,0.16) 50%, transparent 80%);
    transform: translateX(-130%); animation: dsh-band-sweep 1.7s ease-in-out infinite; }
  @keyframes dsh-band-sweep {
    0% { transform: translateX(-130%); }
    55% { transform: translateX(130%); }
    100% { transform: translateX(130%); }
  }
  #dsh-splash .hint { color: #8b93a3; font-size: 12.5px; }
  `;
  var style = document.createElement('style')
  style.textContent = css
  ;(document.head || document.documentElement).appendChild(style)

  // ---- 启动 splash（dsh 页面加载时出现；启动页 index.html 自带 splash，跳过）----
  if (isDshPage) {
  var splash = document.createElement('div')
  splash.id = 'dsh-splash'
  splash.innerHTML = '<div class="wordmark"><svg viewBox="0 0 143 23" width="143" height="23" xmlns="http://www.w3.org/2000/svg"><path d="' + ICON + '"/></svg><b>DeepSeek Harness</b><i>desktop</i><div class="band"></div></div><div class="hint">正在启动服务…</div>'
  document.documentElement.appendChild(splash)
  setTimeout(function () {
    splash.classList.add('fade')
    setTimeout(function () { splash.classList.add('hide') }, 450)
  }, 1400)
  } // end if (isDshPage) splash

  // ---- 灵动岛：顶部中间悬浮手柄 + 工具条 ----
  if (isDshPage) {
  var island = document.createElement('div')
  island.id = 'dsh-island'
  island.innerHTML =
    '<span class="brand"><svg viewBox="0 0 143 23" xmlns="http://www.w3.org/2000/svg"><path d="' + ICON + '"/></svg><b>DeepSeek Harness</b></span>' +
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
  var pressTimer = null
  var dragStarted = false
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

  // 长按拖动：core:window start_dragging
  island.addEventListener('mousedown', function (e) {
    if (e.button !== 0) return
    dragStarted = false
    island.classList.add('dragging')
    e.preventDefault()
    pressTimer = setTimeout(function () {
      dragStarted = true
      invoke('plugin:window|start_dragging').catch(function () {})
    }, 250)
  })
  window.addEventListener('mouseup', function () {
    if (pressTimer) { clearTimeout(pressTimer); pressTimer = null }
    island.classList.remove('dragging')
    dragStarted = false
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
