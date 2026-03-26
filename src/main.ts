// ============================================================
// ConwayDuel - 主加载器
// Trunk 会自动编译此文件并注入 WASM 加载脚本
// WASM 模块通过 <link data-trunk rel="rust"> 自动加载
// ============================================================

// ---- 诊断日志 ----
const diagLog: string[] = [];
const diagStart = Date.now();

function diag(msg: string) {
  const ts = ((Date.now() - diagStart) / 1000).toFixed(3);
  const line = `[${ts}s] ${msg}`;
  diagLog.push(line);
  console.log("[ConwayDuel]", line);
}

function getDiagReport(extra?: string): string {
  let report = `===== ConwayDuel 诊断报告 =====\n`;
  report += `时间: ${new Date().toISOString()}\n`;
  report += `UA: ${navigator.userAgent}\n`;
  report += `屏幕: ${screen.width}x${screen.height}, DPR: ${window.devicePixelRatio}\n`;
  report += `协议: ${location.protocol}\n`;
  report += `\n--- 日志 ---\n`;
  for (const l of diagLog) report += l + "\n";
  if (extra) report += `\n--- 错误详情 ---\n${extra}`;
  report += `\n===== END =====`;
  return report;
}

// ---- 错误弹窗 ----
function showErrorPopup(title: string, message: string, report: string) {
  const canvas = document.getElementById("game-canvas");
  if (canvas) canvas.style.display = "none";

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

  const h = document.createElement("div");
  h.style.cssText = "color:#ff4444;font-size:18px;font-weight:bold;margin-bottom:12px;text-align:center;";
  h.textContent = title;
  overlay.appendChild(h);

  const msg = document.createElement("div");
  msg.style.cssText = "color:#ffaa44;font-size:14px;margin-bottom:16px;text-align:center;word-break:break-all;max-width:90vw;";
  msg.textContent = message;
  overlay.appendChild(msg);

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

  const btnRow = document.createElement("div");
  btnRow.style.cssText = "display:flex;gap:10px;margin-top:14px;flex-wrap:wrap;justify-content:center;";

  const copyBtn = document.createElement("button");
  copyBtn.textContent = "复制诊断报告";
  copyBtn.style.cssText = [
    "padding:10px 20px", "font-size:14px",
    "background:#2196F3", "color:white", "border:none",
    "border-radius:6px", "cursor:pointer",
    "-webkit-appearance:none",
  ].join(";");
  copyBtn.onclick = () => {
    try {
      const ta = document.createElement("textarea");
      ta.value = report;
      ta.style.cssText = "position:fixed;left:-9999px;top:-9999px;opacity:0";
      document.body.appendChild(ta);
      ta.select();
      ta.setSelectionRange(0, report.length);
      const ok = document.execCommand("copy");
      document.body.removeChild(ta);
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
    } catch (_) {}
  };
  btnRow.appendChild(copyBtn);

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
  document.body.appendChild(overlay);

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

// ---- 主加载流程 ----
async function initGame() {
  diag("页面就绪，开始加载");
  diag(`UA: ${navigator.userAgent}`);
  diag(`协议: ${location.protocol}`);
  diag(`屏幕: ${screen.width}x${screen.height}, DPR: ${window.devicePixelRatio}`);

  if (location.protocol === "file:") {
    diag("警告: file:// 协议，WASM 加载可能失败");
  }

  // ---- 阶段1: 初始化 WASM ----
  let wasmExports: any;
  try {
    diag("开始初始化 WASM...");

    // @ts-ignore - Trunk 注入的全局变量
    const conwayDuelModule = window.conway_duel;

    if (conwayDuelModule) {
      await conwayDuelModule.default();
      wasmExports = conwayDuelModule;
      diag("WASM 初始化完成 (Trunk 模式)");
    } else {
      diag("Trunk 全局导出未找到，尝试 pkg 目录导入...");
      const module = await import("../pkg/conway_duel.js");
      await module.init();
      wasmExports = module;
      diag("WASM 初始化完成 (pkg 模式)");
    }
  } catch (e) {
    const err = e instanceof Error ? e : new Error(String(e));
    diag(`WASM 初始化失败: ${err.message}`);

    let hint = "";
    if (location.protocol === "file:") {
      hint = "\n\n原因: file:// 协议下浏览器禁止加载 WASM。\n请通过 HTTP 服务器访问 (如 trunk serve)";
    } else if (err.message.includes("Failed to fetch") || err.message.includes("NetworkError")) {
      hint = "\n\n原因: 网络请求失败，.wasm 文件可能路径不正确或服务器未正确配置 MIME 类型。";
    } else if (err.message.includes("WebAssembly")) {
      hint = "\n\n原因: 当前浏览器不支持 WebAssembly。";
    }

    showErrorPopup("游戏加载失败", err.message + hint, getDiagReport(err.stack || err.message));
    return;
  }

  // ---- 阶段2: Canvas 检测 ----
  const canvas = document.getElementById("game-canvas") as HTMLCanvasElement | null;
  if (!canvas) {
    showErrorPopup("初始化失败", "画布元素 #game-canvas 未找到", getDiagReport());
    return;
  }
  diag(`Canvas 找到: ${canvas.clientWidth}x${canvas.clientHeight}`);

  if (canvas.clientWidth === 0 || canvas.clientHeight === 0) {
    diag("Canvas 尺寸为 0! 尝试强制设置...");
    canvas.style.width = "100vw";
    canvas.style.height = "100vh";
    diag(`强制设置后: ${canvas.clientWidth}x${canvas.clientHeight}`);
  }

  const dpr = window.devicePixelRatio || 1;
  diag(`DPR: ${dpr}`);

  // ---- 阶段3: WebGL 预检测 ----
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
      testCtx.getExtension("WEBGL_lose_context")?.loseContext();
    }
  } catch (e) {
    diag(`WebGL 预检测异常: ${e}`);
  }

  // ---- 阶段4: 创建游戏实例 ----
  let game: any;
  try {
    diag("开始创建游戏实例...");
    const GameAppClass = wasmExports.GameApp;
    if (!GameAppClass) {
      throw new Error("GameApp 类未在 WASM 模块中找到");
    }
    game = new GameAppClass(canvas, dpr);
    if (!game) {
      throw new Error("new GameApp() 返回 null/undefined");
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

    showErrorPopup("游戏初始化失败", err.message + hint, getDiagReport(err.stack || err.message));
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
    showErrorPopup("游戏启动失败", err.message, getDiagReport(err.stack || err.message));
    return;
  }

  // ---- 阶段6: 首帧渲染验证 ----
  try {
    await new Promise<void>((resolve) => setTimeout(resolve, 500));
    diag("首帧等待完成，检查 canvas 像素...");

    const ctx = canvas.getContext("webgl2");
    if (ctx) {
      const pixel = new Uint8Array(4);
      ctx.readPixels(0, 0, 1, 1, ctx.RGBA, ctx.UNSIGNED_BYTE, pixel);
      diag(`像素采样 (0,0): rgba(${pixel[0]},${pixel[1]},${pixel[2]},${pixel[3]})`);

      if (pixel[0] === 0 && pixel[1] === 0 && pixel[2] === 0 && pixel[3] === 0) {
        diag("警告: 首帧像素全黑，可能渲染未生效");
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
