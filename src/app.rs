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
    pub fn new(canvas: web_sys::HtmlCanvasElement, dpr: f32) -> Result<Self, String> {
        let config = GameConfig::default();
        let mut renderer = Renderer::new(canvas.clone(), &config)
            .map_err(|e| format!("渲染器初始化失败: {}", e))?;
        let mut world = World::new();
        let event_bus = EventBus::default();

        let client_width = canvas.client_width() as f32;
        let client_height = canvas.client_height() as f32;
        renderer.resize(client_width, client_height, dpr);

        spawn_ship(&mut world, &config, Vec2::new(20.0, 30.0), Faction::Red);
        spawn_ship(&mut world, &config, Vec2::new(50.0, 30.0), Faction::Green);
        spawn_ship(&mut world, &config, Vec2::new(80.0, 30.0), Faction::Blue);

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

            app.accumulated_time += frame_time;

            let fixed_dt = Duration::from_secs_f32(1.0 / app.config.fixed_update_rate);
            let max_accumulation = Duration::from_secs_f32(app.config.max_frame_accumulation);
            if app.accumulated_time > max_accumulation {
                app.accumulated_time = max_accumulation;
            }

            while app.accumulated_time >= fixed_dt {
                app.fixed_update(fixed_dt);
                app.accumulated_time -= fixed_dt;
            }

            app.render();

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
        self.respawn_dead_ships(dt);

        ai_system(&mut self.world, dt, &self.config);
        weapon_system(&mut self.world, dt, &mut self.event_bus, &self.config);
        movement_system(&mut self.world, dt);
        boundary_system(&mut self.world, &mut self.event_bus, &self.config);
        collision_system(&mut self.world, &mut self.event_bus, &self.config);
        damage_system(&mut self.world, &self.event_bus);

        self.process_explosions();

        cleanup_system(&mut self.world, dt);
        self.event_bus.clear();
    }

    fn render(&mut self) {
        self.renderer.render(&self.world, &self.config);
    }

    pub fn resize(&mut self, width: f32, height: f32, dpr: f32) {
        self.renderer.resize(width, height, dpr);
    }

    pub fn destroy(&mut self) {
        self.running = false;
        if let Some(handle) = self.animation_handle {
            if let Some(window) = web_sys::window() {
                window.cancel_animation_frame(handle).ok();
            }
        }
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
            spawn_explosion(&mut self.world, position, faction);
        }
    }

    fn respawn_dead_ships(&mut self, dt: Duration) {
        let mut to_respawn = Vec::new();

        for (entity, (respawn,)) in self.world.query::<(&RespawnTimer,)>().iter() {
            let mut timer = *respawn;
            timer.remaining = timer.remaining.saturating_sub(dt);
            if timer.remaining.is_zero() {
                to_respawn.push(entity);
            } else {
                if let Ok(mut r) = self.world.query_one_mut::<&mut RespawnTimer>(entity) {
                    r.remaining = timer.remaining;
                }
            }
        }

        for entity in to_respawn {
            let faction = if let Ok(ship) = self.world.query_one_mut::<&Ship>(entity) {
                ship.faction
            } else {
                Faction::Red
            };

            self.world.despawn(entity).ok();

            let x = rand_random() * self.config.world_width;
            let y = rand_random() * self.config.world_height;
            spawn_ship(&mut self.world, &self.config, Vec2::new(x, y), faction);
        }
    }
}

fn spawn_ship(world: &mut World, config: &GameConfig, position: Vec2, faction: Faction) -> hecs::Entity {
    world.spawn((
        Transform {
            position,
            rotation: rand_random() * std::f32::consts::TAU,
            scale: Vec2::ONE,
        },
        Velocity {
            linear: Vec2::ZERO,
            angular: 0.0,
        },
        Ship {
            health: config.ship_max_health,
            max_health: config.ship_max_health,
            faction,
        },
        Collider {
            radius: config.ship_size * 0.5,
            layer: CollisionLayer::Ship,
        },
        Renderable {
            color: faction.to_color(),
            layer: RenderLayer::Ship,
            visible: true,
        },
        AiState::default(),
        Weapon {
            cooldown: Duration::from_secs_f32(1.0 / config.ship_fire_rate),
            remaining_cooldown: Duration::ZERO,
            active: false,
        },
    ))
}

fn rand_random() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f32 / u32::MAX as f32
}
