// 导入Wasm模块，Trunk会自动处理打包
import init, { GameApp } from "../pkg/conway_duel.js";

// 全局错误捕获，安卓端调试友好，控制台可查看完整报错
window.addEventListener("error", (e) => {
  console.error("【全局错误】", e.error);
});

window.addEventListener("unhandledrejection", (e) => {
  console.error("【Promise异常】", e.reason);
});

// 游戏初始化主逻辑
async function initGame() {
  console.log("开始加载Wasm模块...");
  
  try {
    // 1. 初始化Wasm模块，同时触发panic钩子初始化
    await init();
    console.log("Wasm模块加载完成");

    // 2. 获取画布元素
    const canvas = document.getElementById("game-canvas") as HTMLCanvasElement;
    if (!canvas) {
      throw new Error("画布元素 #game-canvas 未找到");
    }

    // 3. 获取设备像素比，安卓端防止画面模糊
    const dpr = window.devicePixelRatio || 1;
    console.log("设备像素比:", dpr);

    // 4. 初始化Rust侧游戏应用实例
    const game = GameApp.new(canvas, dpr);
    if (!game) {
      throw new Error("游戏实例初始化失败");
    }
    console.log("游戏实例初始化完成");

    // 5. 屏幕尺寸变化监听，适配安卓屏幕旋转、窗口大小变化
    const resizeObserver = new ResizeObserver(() => {
      const { clientWidth, clientHeight } = canvas;
      console.log("屏幕尺寸变化:", clientWidth, "x", clientHeight);
      game.resize(clientWidth, clientHeight, dpr);
    });
    resizeObserver.observe(canvas);

    // 6. 屏幕旋转专属适配，安卓端横屏/竖屏切换强制重绘
    window.addEventListener("orientationchange", () => {
      setTimeout(() => {
        const { clientWidth, clientHeight } = canvas;
        game.resize(clientWidth, clientHeight, dpr);
      }, 100);
    });

    // 7. 启动游戏主循环
    game.start();
    console.log("游戏主循环已启动");

    // 8. 页面卸载时资源释放，避免内存泄漏
    window.addEventListener("beforeunload", () => {
      console.log("页面卸载，释放游戏资源");
      game.destroy();
      resizeObserver.disconnect();
    });

  } catch (error) {
    console.error("游戏初始化失败:", error);
    // 显示用户友好的错误信息
    const errorDiv = document.createElement("div");
    errorDiv.style.cssText = `
      position: fixed;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      background: rgba(0,0,0,0.8);
      color: white;
      padding: 20px;
      border-radius: 10px;
      font-family: monospace;
      text-align: center;
      z-index: 9999;
    `;
    errorDiv.innerHTML = `
      <h3>游戏启动失败</h3>
      <p>${error.message}</p>
      <p>请检查浏览器控制台获取详细信息</p>
    `;
    document.body.appendChild(errorDiv);
  }
}

// 页面加载完成后启动游戏
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initGame);
} else {
  // 页面已经加载完成，直接初始化
  initGame();
}
