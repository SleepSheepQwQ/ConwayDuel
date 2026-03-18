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
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: web_sys::HtmlCanvasElement, dpr: f32) -> Self {
        Self {
            inner: app::GameAppInner::new(canvas, dpr),
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
