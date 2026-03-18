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
    last_frame_time: Option<f64>,  // milliseconds
    accumulated_time: Duration,
    canvas: web_sys::HtmlCanvasElement,
    dpr: f32,
}

impl GameAppInner {
    pub fn new(canvas: web_sys::HtmlCanvasElement, dpr: f32) -> Self {
        let config = GameConfig::default();
        let mut renderer = Renderer::new(canvas.clone(), &config).unwrap();
        let mut world = World::new();
        let event_bus = EventBus::default();

        // 初始化画布尺寸
        let client_width = canvas.client_width() as f32;
        let client_height = canvas.client_height() as f32;
        renderer.resize(client_width, client_height, dpr);

        // 生成三艘初始飞船
        spawn_ship(&mut world, &config, Vec2::new(20.0, 30.0), Faction::Red);
        spawn_ship(&mut world, &config, Vec2::new(50.0, 30.0), Faction::Green);
        spawn_ship(&mut world, &config, Vec2::new(80.0, 30.0), Faction::Blue);

        Self {
            config,
            world,
            event_bus,
            renderer,
            running: false,
            animation_handle: None,
            last_frame_time: None,  // Performance.now() milliseconds
            accumulated_time: Duration::ZERO,
            canvas,
            dpr,
        }
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
            app.animation_handle = Some(
                window.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap()
            );
        }) as Box<dyn FnMut(f64)>));

        let window = web_sys::window().unwrap();
        self.animation_handle = Some(
            window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap()
        );
    }

    fn fixed_update(&mut self, dt: Duration) {
        // 检查并重生死亡的飞船
        self.respawn_dead_ships(dt);

        ai_system(&mut self.world, dt, &self.config);
        weapon_system(&mut self.world, dt, &mut self.event_bus, &self.config);
        movement_system(&mut self.world, dt);
        boundary_system(&mut self.world, &mut self.event_bus, &self.config);
        collision_system(&mut self.world, &mut self.event_bus, &self.config);
        damage_system(&mut self.world, &self.event_bus);

        // 处理爆炸事件，生成特效
        self.process_explosions();

        cleanup_system(&mut self.world, dt);
        self.event_bus.clear();
    }

    fn process_explosions(&mut self) {
        use crate::ecs::events::GameEvent;

        let mut explosions = Vec::new();
        for event in self.event_bus.events() {
            if let GameEvent::Death { position, faction } = event {
                explosions.push((*position, *faction));
            }
        }

        for (position, faction) in explosions {
            spawn_explosion(&mut self.world, &position, faction, &self.config);
        }
    }

    fn respawn_dead_ships(&mut self, _dt: Duration) {
        // 收集需要重生的飞船信息
        let mut to_respawn = Vec::new();
        for (entity, (health, faction)) in self.world.query::<(&Health, &FactionComponent)>().iter() {
            if health.is_dead {
                to_respawn.push((entity, faction.faction));
            }
        }

        // 处理重生
        for (entity, faction) in to_respawn {
            // 移除旧实体
            let _ = self.world.despawn(entity);

            // 在随机位置生成新飞船
            let x = rand_random() * self.config.world_width * 0.8 + self.config.world_width * 0.1;
            let y = rand_random() * self.config.world_height * 0.8 + self.config.world_height * 0.1;
            spawn_ship(&mut self.world, &self.config, Vec2::new(x, y), faction);
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

    let health = Health::new(config.ship_max_health);
    let faction_component = FactionComponent { faction };
    let weapon = Weapon::from_config(config);
    let collider = Collider {
        radius: config.ship_size / 1.5,
        layer: CollisionLayer::Ship,
    };
    let renderable = Renderable {
        color: faction.to_color(),
        layer: RenderLayer::Ship,
        visible: true,
    };
    let ai_state = AiState::default();

    world.spawn((
        transform,
        velocity,
        health,
        faction_component,
        weapon,
        collider,
        renderable,
        ai_state,
    ));
}

/// 简单的随机数生成（WASM环境）
fn rand_random() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos as f32 / u32::MAX as f32)
}
