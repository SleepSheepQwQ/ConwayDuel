use glam::Vec2;
use hecs::World;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::config::{Faction, GameConfig};
use crate::core::ai::ai_system;
use crate::core::combat::{cleanup_system, damage_system, spawn_explosion, weapon_system};
use crate::core::physics::{boundary_system, collision_system, movement_system};
use crate::core::render::Renderer;
use crate::ecs::components::*;
use crate::ecs::events::EventBus;

pub struct GameAppInner {
    config: GameConfig,
    world: World,
    event_bus: EventBus,
    renderer: Renderer,
    running: bool,
    animation_handle: Option<i32>,
    last_frame_time: Option<f64>,
    accumulated_time: Duration,
    canvas: web_sys::HtmlCanvasElement,
    dpr: f32,
}

impl GameAppInner {
    /// 创建游戏实例，每个阶段独立捕获错误
    pub fn new(canvas: web_sys::HtmlCanvasElement, dpr: f32) -> Result<Self, String> {
        let config = GameConfig::default();

        // 阶段1: 初始化渲染器（WebGL 上下文 + 着色器编译）
        let renderer = Renderer::new(canvas.clone(), &config).map_err(|e| {
            format!("渲染器初始化失败: {}", e)
        })?;

        let mut world = World::new();
        let event_bus = EventBus::default();

        // 阶段2: 初始化画布尺寸
        let client_width = canvas.client_width() as f32;
        let client_height = canvas.client_height() as f32;

        if client_width == 0.0 || client_height == 0.0 {
            log::warn!(
                "Canvas 尺寸异常: {}x{}, DPR: {}",
                client_width, client_height, dpr
            );
        }

        renderer.resize(client_width, client_height, dpr);

        // 阶段3: 生成三艘初始飞船
        spawn_ship(&mut world, &config, Vec2::new(20.0, 30.0), Faction::Red);
        spawn_ship(&mut world, &config, Vec2::new(50.0, 30.0), Faction::Green);
        spawn_ship(&mut world, &config, Vec2::new(80.0, 30.0), Faction::Blue);

        log::info!(
            "游戏初始化完成: canvas={}x{}, dpr={}, 世界={}x{}",
            client_width, client_height, dpr,
            config.world_width, config.world_height
        );

        Ok(Self {
            config,
            world,
            event_bus,
            renderer,
            running: false,
            animation_handle: None,
            last_frame_time: None,
            accumulated_time: Duration::ZERO,
            canvas,
            dpr,
        })
    }

    pub fn start(&mut self) {
        if self.running {
            return;
        }
        self.running = true;
        let now_ms = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now() as f64)
            .unwrap_or(0.0);
        self.last_frame_time = Some(now_ms);

        // 使用 Rc<RefCell<>> 包裹 self，实现闭包中的共享所有权
        let app = Rc::new(RefCell::new(self as *mut GameAppInner));
        let app_clone = app.clone();

        let f = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
        let g = f.clone();

        *g.borrow_mut() = Some(Closure::wrap(Box::new(move |_time: f64| {
            let app_ptr = *app_clone.borrow();
            let app = unsafe { &mut *app_ptr };

            if !app.running {
                return;
            }

            // 计算实际帧时间
            let now_ms = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now() as f64)
                .unwrap_or(0.0);
            let frame_time = if let Some(last_ms) = app.last_frame_time {
                let delta_ms = now_ms - last_ms;
                Duration::from_secs_f64(delta_ms / 1000.0)
            } else {
                Duration::from_secs_f64(1.0 / 60.0)
            };
            app.last_frame_time = Some(now_ms);

            // 累积帧时间
            app.accumulated_time += frame_time;

            // 固定时间步长更新
            let fixed_dt = Duration::from_secs_f32(1.0 / app.config.fixed_update_rate);
            let max_accumulation = Duration::from_secs_f32(app.config.max_frame_accumulation);
            if app.accumulated_time > max_accumulation {
                app.accumulated_time = max_accumulation;
            }

            while app.accumulated_time >= fixed_dt {
                app.fixed_update(fixed_dt);
                app.accumulated_time -= fixed_dt;
            }

            // 渲染
            app.render();

            // 继续下一帧
            let window = web_sys::window().unwrap();
            let handle = window
                .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
                .unwrap();
            app.animation_handle = Some(handle);
        }) as Box<dyn FnMut(f64)>));

        let window = web_sys::window().unwrap();
        let handle = window
            .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
        self.animation_handle = Some(handle);
    }

    fn fixed_update(&mut self, dt: Duration) {
        ai_system(&mut self.world, dt, &self.config);
        weapon_system(&mut self.world, dt, &mut self.event_bus, &self.config);
        movement_system(&mut self.world, dt);
        boundary_system(&mut self.world, &mut self.event_bus, &self.config);
        collision_system(&mut self.world, &mut self.event_bus, &self.config);
        damage_system(&mut self.world, &mut self.event_bus);

        // 处理爆炸事件，生成特效
        self.process_explosions();

        cleanup_system(&mut self.world, dt);
        self.event_bus.clear();
    }

    fn process_explosions(&mut self) {
        use crate::ecs::events::GameEvent;

        let mut explosions = Vec::new();
        for event in self.event_bus.events() {
            if let GameEvent::EntityDestroyed { position, faction } = event {
                explosions.push((*position, *faction));
            }
        }

        for (position, faction) in explosions {
            spawn_explosion(&mut self.world, &self.config, position, *faction);
        }
    }

    fn render(&mut self) {
        self.renderer.render(&self.world, &self.config);
    }

    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        self.dpr = dpr;
        self.renderer.resize(width, height, dpr);
    }

    pub fn destroy(&mut self) {
        self.running = false;
        if let Some(h) = self.animation_handle {
            let _ = web_sys::window().unwrap().cancel_animation_frame(h);
        }
    }
}

/// 生成飞船的工具函数
pub fn spawn_ship(world: &mut World, config: &GameConfig, position: Vec2, faction: Faction) {
    let transform = Transform {
        position,
        rotation: 0.0,
        scale: Vec2::splat(config.ship_size),
    };

    let velocity = Velocity {
        linear: Vec2::ZERO,
        angular: 0.0,
        max_speed: config.ship_max_speed,
    };

    let health = Health {
        current: config.ship_max_health,
        max: config.ship_max_health,
        is_dead: false,
    };

    let faction_component = FactionComponent { faction };

    let collider = Collider {
        radius: config.ship_size * 0.5,
        layer: CollisionLayer::Ship,
    };

    let weapon = Weapon::from_config(config);
    let ai_state = AiState::default();

    let color = faction.color();

    let renderable = Renderable {
        color,
        shape: RenderShape::Ship,
    };

    world.spawn((
        transform,
        velocity,
        health,
        faction_component,
        collider,
        weapon,
        ai_state,
        renderable,
    ));
}

fn rand_random() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f32 / u32::MAX as f32
}
