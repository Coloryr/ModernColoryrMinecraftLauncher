// M²L 自定义主页面桥接脚本
//
// 由 `mml-home://` 协议按 `/__mml_bridge.js` 直接返回（内置资源，不在 custom_home/ 里），
// 并自动注入到每个返回的 HTML 的 `</head>` 之前。页面通过全局的 `window.mml` 使用：
//
//   window.mml = {
//     invoke(cmd, args) -> Promise<any>,   // 调任意已注册的启动器命令
//     on(event, cb) -> () => void,         // 订阅白名单事件，返回取消订阅函数
//     ready: Promise,                      // 父窗口握手完成
//   }
//
// 消息全部走 postMessage（iframe 是 opaque origin，没有 allow-same-origin，
// 只能 targetOrigin "*"）。消息格式见 TASK.md 与父窗口侧 `lib/customHomeBridge.ts`。
(function () {
  "use strict";

  // 页面自己提供了同名对象时让位（正常情况不会发生：本脚本注入在 head 末尾）
  if (window.mml) return;

  var PROTOCOL = 1;
  var READY_INTERVAL = 100;
  var READY_TRIES = 100;

  var seq = 0;
  /** invoke 请求 id -> { resolve, reject } */
  var pending = new Map();
  /** 订阅 id -> 回调 */
  var handlers = new Map();

  var readyDone = false;
  var readyTimer = null;
  var readyTries = 0;
  var readyResolve;
  var ready = new Promise(function (resolve) {
    readyResolve = resolve;
  });

  function post(msg) {
    msg.__mml = PROTOCOL;
    window.parent.postMessage(msg, "*");
  }

  function stopReadyRetry() {
    if (readyTimer !== null) {
      clearInterval(readyTimer);
      readyTimer = null;
    }
  }

  function finishReady() {
    if (readyDone) return;
    readyDone = true;
    stopReadyRetry();
    readyResolve();
  }

  function sendReady() {
    if (readyDone) return;
    post({ type: "ready" });
    readyTries += 1;
    if (readyTries >= READY_TRIES) {
      // 父窗口一直没应答（页面不是启动器加载的）：停止重试，让 ready 落地，
      // 之后的 invoke 会因无人应答而一直 pending——调用方自己加超时
      finishReady();
    }
  }

  // 握手带重试：桥接脚本可能先于父窗口的 message 监听就绪
  sendReady();
  readyTimer = setInterval(sendReady, READY_INTERVAL);

  window.addEventListener("message", function (e) {
    // 只认父窗口发来的、带协议标记的消息
    if (e.source !== window.parent) return;
    var d = e.data;
    if (!d || d.__mml !== PROTOCOL) return;

    if (d.type === "ready") {
      finishReady();
      return;
    }

    if (d.type === "event") {
      var cb = handlers.get(d.id);
      if (!cb) return;
      try {
        cb(d.payload);
      } catch (err) {
        console.error("[mml] event handler error:", err);
      }
      return;
    }

    // invoke 应答：{ id, ok: true, data } / { id, ok: false, error }
    if (d.id === undefined) return;
    var p = pending.get(d.id);
    if (!p) return;
    pending.delete(d.id);
    if (d.ok) {
      p.resolve(d.data);
    } else {
      p.reject(new Error(d.error === undefined || d.error === null ? "mml invoke failed" : String(d.error)));
    }
  });

  /**
   * 调用启动器命令
   * @param {string} cmd 命令名（与 `#[tauri::command]` 函数名一致，如 main_get_instances）
   * @param {any} [args] 命令参数对象（键名与 bindings.ts 里的一致）
   * @returns {Promise<any>}
   */
  function invoke(cmd, args) {
    return ready.then(function () {
      return new Promise(function (resolve, reject) {
        var id = ++seq;
        pending.set(id, { resolve: resolve, reject: reject });
        post({
          id: id,
          type: "invoke",
          cmd: String(cmd),
          args: args === undefined ? null : args,
        });
      });
    });
  }

  /**
   * 订阅启动器事件（只支持父窗口白名单里的事件）
   *
   * 同步返回取消订阅函数；真正的 subscribe 会等握手完成后再发，
   * 所以在 ready 之前就取消订阅也不会漏发消息。
   *
   * @param {string} event 事件名（如 launch-state）
   * @param {(payload: any) => void} cb
   * @returns {() => void}
   */
  function on(event, cb) {
    if (typeof cb !== "function") {
      throw new TypeError("mml.on(event, cb): cb 必须是函数");
    }
    var id = ++seq;
    handlers.set(id, cb);
    ready.then(function () {
      if (handlers.has(id)) {
        post({ id: id, type: "subscribe", event: String(event) });
      }
    });
    return function off() {
      if (!handlers.delete(id)) return;
      if (readyDone) {
        post({ type: "unsubscribe", id: id });
      }
    };
  }

  window.mml = {
    invoke: invoke,
    on: on,
    ready: ready,
  };
})();
