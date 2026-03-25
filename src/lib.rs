mod app;
mod config;
mod ecs;
mod core;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
}

#[wasm_bindgen]
pub struct GameApp {
    inner: app::GameAppInner,
}

#[wasm_bindgen]
impl GameApp {
    /// 创建游戏实例，失败时返回 JS 错误（含详细诊断信息）
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: web_sys::HtmlCanvasElement, dpr: f32) -> Result<GameApp, JsValue> {
        match app::GameAppInner::new(canvas, dpr) {
            Ok(inner) => Ok(GameApp { inner }),
            Err(e) => Err(JsValue::from_str(&format!(
                "[Rust] 游戏初始化失败: {}\n\
                 请将此信息连同浏览器控制台日志一起反馈。",
                e
            ))),
        }
    }

    pub fn start(&mut self) {
        self.inner.start();
    }

    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        self.inner.resize(width, height, dpr);
    }

    pub fn destroy(&mut self) {
        self.inner.destroy();
    }
}
