// ============================================================
// ConwayDuel - 极端诊断版加载器
// 针对 Via 浏览器优化：兼容性优先，任何失败都弹窗+可复制
// ============================================================

// ---- 全局诊断日志收集 ----
const diagLog: string[] = [];
const diagStart = Date.now();

function diag(msg: string) {
  const ts = ((Date.now() - diagStart) / 1000).toFixed(3);
  const line = `[${ts}s] ${msg}`;
  diagLog.push(line);
  try { console.log(line); } catch (_) {}
}

function getDiagReport(extra?: string): string {
  const ua = navigator.userAgent || "unknown";
  const url = location.href || "unknown";
  const screen = `${screen.width}x${screen.height}`;
  const dpr = window.devicePixelRatio || 1;
  const canvas = document.getElementById("game-canvas") as HTMLCanvasElement | null;
  const canvasInfo = canvas
    ? `canvas: ${canvas.clientWidth}x${canvas.clientHeight}, offset: ${canvas.offsetWidth}x${canvas.offsetHeight}`
    : "canvas: NOT FOUND";

  let report = `===== ConwayDuel 诊断报告 =====\n`;
  report += `时间: ${new Date().toISOString()}\n`;
  report += `UA: ${ua}\n`;
  report += `URL: ${url}\n`;
  report += `屏幕: ${screen}, DPR: ${dpr}\n`;
  report += `${canvasInfo}\n`;
  report += `协议: ${location.protocol}\n`;
  report += `HTTPS: ${location.protocol === "https:"}\n`;
  report += `\n--- 加载日志 ---\n`;
  report += diagLog.join("\n");
  if (extra) {
    report += `\n\n--- 错误详情 ---\n${extra}`;
  }
  report += `\n===== END =====`;
  return report;
}

// ---- Via 兼容的复制函数 ----
function copyText(text: string): boolean {
  // 方法1: execCommand (Via 兼容性最好)
  try {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.style.cssText = "position:fixed;left:-9999px;top:-9999px;opacity:0";
    document.body.appendChild(ta);
    ta.select();
    ta.setSelectionRange(0, text.length);
    const ok = document.execCommand("copy");
    document.body.removeChild(ta);
    if (ok) return true;
  } catch (_) {}

  // 方法2: navigator.clipboard (现代浏览器)
  try {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text);
      return true;
    }
  } catch (_) {}

  return false;
}

// ---- 全屏错误弹窗 ----
function showErrorPopup(title: string, message: string, report: string) {
  // 隐藏 canvas
  const canvas = document.getElementById("game-canvas");
  if (canvas) canvas.style.display = "none";

  // 移除已有弹窗
  const old = document.getElementById("diag-popup");
  if (old) old.remove();

  const overlay = document.createElement("div");
  overlay.id = "diag-popup";
  overlay.style.cssText = [
    "position:fixed", "top:0", "left:0", "right:0", "bottom:0",
    "background:rgba(0,0,0,0.92)", "z-index:99999",
    "display:flex", "flex-direction:column",
    "align-items:center", "justify-content:center",
    "padding:16px", "overflow-y:auto",
    "font-family:monospace", "color:#e0e0e0",
    "-webkit-overflow-scrolling:touch",
  ].join(";");

  // 标题
  const h = document.createElement("div");
  h.style.cssText = "color:#ff4444;font-size:18px;font-weight:bold;margin-bottom:12px;text-align:center;";
  h.textContent = title;
  overlay.appendChild(h);

  // 错误消息
  const msg = document.createElement("div");
  msg.style.cssText = "color:#ffaa44;font-size:14px;margin-bottom:16px;text-align:center;word-break:break-all;max-width:90vw;";
  msg.textContent = message;
  overlay.appendChild(msg);

  // 诊断报告区域
  const reportBox = document.createElement("textarea");
  reportBox.readOnly = true;
  reportBox.value = report;
  reportBox.style.cssText = [
    "width:90vw", "max-width:500px", "height:200px",
    "background:#1a1a2e", "color:#aaa", "border:1px solid #333",
    "border-radius:6px", "padding:10px", "font-size:11px",
    "font-family:monospace", "resize:vertical",
    "-webkit-appearance:none",
  ].join(";");
  overlay.appendChild(reportBox);

  // 按钮容器
  const btnRow = document.createElement("div");
  btnRow.style.cssText = "display:flex;gap:10px;margin-top:14px;flex-wrap:wrap;justify-content:center;";

  // 复制按钮
  const copyBtn = document.createElement("button");
  copyBtn.textContent = "复制诊断报告";
  copyBtn.style.cssText = [
    "padding:10px 20px", "font-size:14px",
    "background:#2196F3", "color:white", "border:none",
    "border-radius:6px", "cursor:pointer",
    "-webkit-appearance:none",
  ].join(";");
  copyBtn.onclick = () => {
    const ok = copyText(report);
    copyBtn.textContent = ok ? "已复制!" : "复制失败,请手动全选";
    copyBtn.style.background = ok ? "#4CAF50" : "#ff9800";
    if (!ok) {
      reportBox.select();
      reportBox.setSelectionRange(0, report.length);
    }
    setTimeout(() => {
      copyBtn.textContent = "复制诊断报告";
      copyBtn.style.background = "#2196F3";
    }, 2000);
  };
  btnRow.appendChild(copyBtn);

  // 重试按钮
  const retryBtn = document.createElement("button");
  retryBtn.textContent = "重试加载";
  retryBtn.style.cssText = [
    "padding:10px 20px", "font-size:14px",
    "background:#FF9800", "color:white", "border:none",
    "border-radius:6px", "cursor:pointer",
    "-webkit-appearance:none",
  ].join(";");
  retryBtn.onclick = () => {
    overlay.remove();
    if (canvas) canvas.style.display = "block";
    location.reload();
  };
  btnRow.appendChild(retryBtn);

  overlay.appendChild(btnRow);

  // WebGL 检测提示
  const webglInfo = document.createElement("div");
  webglInfo.style.cssText = "color:#666;font-size:11px;margin-top:16px;text-align:center;max-width:90vw;";
  try {
    const testCanvas = document.createElement("canvas");
    const gl = testCanvas.getContext("webgl2") || testCanvas.getContext("webgl") || testCanvas.getContext("experimental-webgl");
    webglInfo.textContent = gl
      ? `WebGL 检测: 可用 (${gl instanceof WebGL2RenderingContext ? "WebGL2" : "WebGL1"})`
      : "WebGL 检测: 不可用 (浏览器或设备不支持)";
  } catch (e) {
    webglInfo.textContent = `WebGL 检测: 异常 (${e})`;
  }
  overlay.appendChild(webglInfo);

  document.body.appendChild(overlay);

  // 同时 alert 一次确保 Via 用户能看到
  try {
    alert(`${title}\n\n${message}\n\n详细诊断信息已显示在页面上，请点击"复制诊断报告"按钮。`);
  } catch (_) {}
}

// ---- 全局错误捕获 ----
window.addEventListener("error", (e) => {
  diag(`[全局错误] ${e.message} @ ${e.filename}:${e.lineno}:${e.colno}`);
});

window.addEventListener("unhandledrejection", (e) => {
  const reason = e.reason;
  const msg = reason instanceof Error ? `${reason.message}\n${reason.stack}` : String(reason);
  diag(`[Promise异常] ${msg}`);
  showErrorPopup("游戏运行时异常", String(reason).slice(0, 200), getDiagReport(msg));
});

// ---- 带超时的 Promise 包装 ----
function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error(`${label} 超时 (${ms}ms)`));
    }, ms);
    promise.then(
      (v) => { clearTimeout(timer); resolve(v); },
      (e) => { clearTimeout(timer); reject(e); },
    );
  });
}

// ---- 主加载流程 ----
async function initGame() {
  diag("页面就绪，开始加载");

  // ---- 阶段0: 环境检测 ----
  diag(`UA: ${navigator.userAgent}`);
  diag(`协议: ${location.protocol}`);
  diag(`屏幕: ${screen.width}x${screen.height}, DPR: ${window.devicePixelRatio}`);

  if (location.protocol === "file:") {
    diag("警告: file:// 协议，WASM 加载可能失败");
  }

  // ---- 阶段1: 动态 import WASM 模块 ----
  let init: () => Promise<void>;
  let GameApp: any;

  try {
    diag("开始加载 WASM 模块...");

    // 使用动态 import 以便捕获加载失败
    const wasmModule = await withTimeout(
      import("../pkg/conway_duel.js"),
      30000,
      "WASM 模块加载"
    );

    diag("WASM JS 胶水层加载成功");

    init = wasmModule.init || wasmModule.default;
    GameApp = wasmModule.GameApp;

    if (typeof init !== "function") {
      throw new Error(`init 不是函数: ${typeof init}`);
    }
    if (!GameApp) {
      throw new Error("GameApp 未导出");
    }

    diag("开始初始化 WASM 运行时...");
    await withTimeout(init(), 15000, "WASM 运行时初始化");
    diag("WASM 运行时初始化完成");

  } catch (e) {
    const err = e instanceof Error ? e : new Error(String(e));
    diag(`WASM 加载失败: ${err.message}`);

    let hint = "";
    if (location.protocol === "file:") {
      hint = "\n\n原因: file:// 协议下浏览器禁止加载 WASM。\n请通过 HTTP 服务器访问 (如 python3 -m http.server 8080)";
    } else if (err.message.includes("Failed to fetch") || err.message.includes("NetworkError")) {
      hint = "\n\n原因: 网络请求失败，.wasm 文件可能路径不正确或服务器未正确配置 MIME 类型。";
    } else if (err.message.includes("WebAssembly")) {
      hint = "\n\n原因: 当前浏览器不支持 WebAssembly。";
    } else if (err.message.includes("超时")) {
      hint = "\n\n原因: 加载时间过长，可能是网络慢或设备性能不足。";
    }

    showErrorPopup(
      "游戏加载失败",
      err.message + hint,
      getDiagReport(err.stack || err.message)
    );
    return;
  }

  // ---- 阶段2: Canvas 检测 ----
  const canvas = document.getElementById("game-canvas") as HTMLCanvasElement | null;
  if (!canvas) {
    showErrorPopup("初始化失败", "画布元素 #game-canvas 未找到", getDiagReport());
    return;
  }
  diag(`Canvas 找到: ${canvas.clientWidth}x${canvas.clientHeight}`);

  // 检查 canvas 实际尺寸
  if (canvas.clientWidth === 0 || canvas.clientHeight === 0) {
    diag(`Canvas 尺寸为 0! clientWidth=${canvas.clientWidth}, clientHeight=${canvas.clientHeight}`);
    diag(`Canvas offsetWidth=${canvas.offsetWidth}, offsetHeight=${canvas.offsetHeight}`);
    diag(`Canvas style.width=${canvas.style.width}, style.height=${canvas.style.height}`);
    // 尝试强制设置尺寸
    canvas.style.width = "100vw";
    canvas.style.height = "100vh";
    diag(`强制设置后: ${canvas.clientWidth}x${canvas.clientHeight}`);
  }

  const dpr = window.devicePixelRatio || 1;
  diag(`DPR: ${dpr}`);

  // ---- 阶段3: WebGL 上下文预检测 ----
  try {
    const testCtx = canvas.getContext("webgl2");
    if (!testCtx) {
      const testCtx1 = canvas.getContext("webgl");
      diag(`WebGL2 不可用, WebGL1: ${testCtx1 ? "可用" : "不可用"}`);
      if (!testCtx1) {
        showErrorPopup(
          "WebGL 不可用",
          "当前浏览器/设备不支持 WebGL，无法运行游戏。\n请尝试使用 Chrome、Firefox 或 Edge。",
          getDiagReport()
        );
        return;
      }
      diag("警告: 仅 WebGL1 可用，游戏需要 WebGL2");
    } else {
      diag("WebGL2 上下文预检测通过");
      // 释放预检测上下文，让 Rust 侧重新获取
      testCtx.getExtension("WEBGL_lose_context")?.loseContext();
    }
  } catch (e) {
    diag(`WebGL 预检测异常: ${e}`);
  }

  // ---- 阶段4: 创建游戏实例 ----
  let game: any;
  try {
    diag("开始创建游戏实例...");
    game = GameApp.new(canvas, dpr);
    if (!game) {
      throw new Error("GameApp.new() 返回 null/undefined");
    }
    diag("游戏实例创建成功");
  } catch (e) {
    const err = e instanceof Error ? e : new Error(String(e));
    diag(`游戏实例创建失败: ${err.message}`);

    let hint = "";
    if (err.message.includes("WebGL") || err.message.includes("webgl")) {
      hint = "\n\n原因: WebGL 初始化失败，可能是浏览器版本过旧或 GPU 驱动问题。";
    } else if (err.message.includes("shader") || err.message.includes("着色器")) {
      hint = "\n\n原因: 着色器编译失败，可能是 GPU 不支持所需特性。";
    }

    showErrorPopup(
      "游戏初始化失败",
      err.message + hint,
      getDiagReport(err.stack || err.message)
    );
    return;
  }

  // ---- 阶段5: 启动游戏循环 ----
  try {
    diag("启动游戏主循环...");
    game.start();
    diag("游戏主循环已启动");
  } catch (e) {
    const err = e instanceof Error ? e : new Error(String(e));
    diag(`游戏启动失败: ${err.message}`);
    showErrorPopup(
      "游戏启动失败",
      err.message,
      getDiagReport(err.stack || err.message)
    );
    return;
  }

  // ---- 阶段6: 首帧渲染验证 ----
  try {
    await new Promise<void>((resolve) => setTimeout(resolve, 500));
    diag("首帧等待完成，检查 canvas 像素...");

    // 尝试读取像素判断是否真的渲染了
    const ctx = canvas.getContext("webgl2");
    if (ctx) {
      const pixel = new Uint8Array(4);
      ctx.readPixels(0, 0, 1, 1, ctx.RGBA, ctx.UNSIGNED_BYTE, pixel);
      diag(`像素采样 (0,0): rgba(${pixel[0]},${pixel[1]},${pixel[2]},${pixel[3]})`);

      if (pixel[0] === 0 && pixel[1] === 0 && pixel[2] === 0 && pixel[3] === 0) {
        diag("警告: 首帧像素全黑，可能渲染未生效");
        // 不弹窗，因为背景确实是深蓝色接近黑色，但记录日志
      } else {
        diag("首帧渲染检测: 有像素输出");
      }
    }
  } catch (e) {
    diag(`首帧验证异常: ${e}`);
  }

  // ---- 阶段7: Resize 监听 ----
  try {
    if (typeof ResizeObserver !== "undefined") {
      const resizeObserver = new ResizeObserver(() => {
        const { clientWidth, clientHeight } = canvas;
        diag(`Resize: ${clientWidth}x${clientHeight}`);
        try { game.resize(clientWidth, clientHeight, dpr); } catch (_) {}
      });
      resizeObserver.observe(canvas);

      window.addEventListener("beforeunload", () => {
        diag("页面卸载");
        try { game.destroy(); } catch (_) {}
        resizeObserver.disconnect();
      });
    } else {
      diag("ResizeObserver 不可用，使用 window.resize");
      window.addEventListener("resize", () => {
        const { clientWidth, clientHeight } = canvas;
        try { game.resize(clientWidth, clientHeight, dpr); } catch (_) {}
      });
    }
  } catch (e) {
    diag(`Resize 监听设置异常: ${e}`);
  }

  diag("全部加载阶段完成");
}

// ---- 启动 ----
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initGame);
} else {
  initGame();
}
