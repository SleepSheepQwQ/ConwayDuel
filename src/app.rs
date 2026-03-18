use glam::Vec2;
use hecs::World;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::config::GameConfig;
use crate::core::ai::ai_system;
use crate::core::combat::{cleanup_system, damage_system, weapon_system};
use crate::core::physics::{boundary_system, collision_system, movement_system};
use crate::core::render::{Renderer, Renderable};
use crate::ecs::components::{Collider, FactionComponent, Health, Transform, Velocity, Weapon};
use crate::ecs::events::EventBus;

pub struct GameAppInner {
    config: GameConfig,
    world: World,
    event_bus: EventBus,
    renderer: Renderer,
    running: bool,
    animation_handle: Option<i32>,
}

impl GameAppInner {
    pub fn new(canvas: web_sys::HtmlCanvasElement, dpr: f32) -> Self {
        let config = GameConfig::default();
        let renderer = Renderer::new(canvas, &config).unwrap();
        let mut world = World::new();
        let event_bus = EventBus::default();

        Self {
            config,
            world,
            event_bus,
            renderer,
            running: false,
            animation_handle: None,
        }
    }

    pub fn start(&mut self) {
        if self.running {
            return;
        }
        self.running = true;

        let f = Rc::new(RefCell::new(None));
        let g = f.clone();
        let mut app = self;

        *g.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
            if !app.running {
                return;
            }

            let dt = std::time::Duration::from_secs_f64(1.0 / 60.0);
            app.fixed_update(dt);
            app.render();

            let window = web_sys::window().unwrap();
            app.animation_handle = Some(
                window.request_animation_frame(f.borrow().as_ref().unwrap()).unwrap()
            );
        }) as Box<dyn FnMut(f64)>));

        let window = web_sys::window().unwrap();
        self.animation_handle = Some(
            window.request_animation_frame(g.borrow().as_ref().unwrap()).unwrap()
        );
    }

    fn fixed_update(&mut self, dt: std::time::Duration) {
        ai_system(&mut self.world, dt, &self.config);
        weapon_system(&mut self.world, dt, &mut self.event_bus, &self.config);
        movement_system(&mut self.world, dt);
        boundary_system(&mut self.world, &self.config);
        collision_system(&mut self.world, &mut self.event_bus, &self.config);
        damage_system(&mut self.world, &mut self.event_bus);
        cleanup_system(&mut self.world, dt);
        self.event_bus.clear();
    }

    fn render(&mut self) {
        self.renderer.render(&self.world);
    }

    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        self.renderer.resize(width, height, dpr);
    }

    pub fn destroy(&mut self) {
        self.running = false;
        if let Some(h) = self.animation_handle {
            let _ = web_sys::window().unwrap().cancel_animation_frame(h);
        }
    }
}
