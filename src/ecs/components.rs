use glam::Vec2;
use std::time::Duration;
use crate::config::GameConfig;

/// 变换组件：位置、旋转、缩放
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

unsafe impl hecs::Component for Transform {}

/// 速度组件：线速度、角速度、最大速度
#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub linear: Vec2,
    pub angular: f32,
    pub max_speed: f32,
}

unsafe impl hecs::Component for Velocity {}

impl Default for Velocity {
    fn default() -> Self {
        Self {
            linear: Vec2::ZERO,
            angular: 0.0,
            max_speed: 8.0,
        }
    }
}

/// 生命值组件
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    pub is_dead: bool,
}

unsafe impl hecs::Component for Health {}

impl Health {
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
            is_dead: false,
        }
    }

    pub fn take_damage(&mut self, damage: f32) {
        self.current = (self.current - damage).max(0.0);
        if self.current <= 0.0 {
            self.is_dead = true;
        }
    }

    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
        if self.current > 0.0 {
            self.is_dead = false;
        }
    }
}

/// 阵营组件
#[derive(Debug, Clone, Copy)]
pub struct FactionComponent {
    pub faction: crate::config::Faction,
}

unsafe impl hecs::Component for FactionComponent {}

/// 武器组件
#[derive(Debug, Clone)]
pub struct Weapon {
    pub fire_cooldown: Duration,
    pub cooldown_remaining: Duration,
    pub bullet_speed: f32,
    pub bullet_damage: f32,
    pub bullet_lifetime: Duration,
}

unsafe impl hecs::Component for Weapon {}

impl Weapon {
    pub fn from_config(config: &GameConfig) -> Self {
        Self {
            fire_cooldown: Duration::from_secs_f32(config.fire_cooldown),
            cooldown_remaining: Duration::ZERO,
            bullet_speed: config.ship_max_speed * config.bullet_speed_multiplier,
            bullet_damage: config.bullet_damage,
            bullet_lifetime: Duration::from_secs_f32(config.bullet_lifetime),
        }
    }

    pub fn can_fire(&self) -> bool {
        self.cooldown_remaining.is_zero()
    }

    pub fn fire(&mut self) {
        self.cooldown_remaining = self.fire_cooldown;
    }

    pub fn update(&mut self, dt: Duration) {
        self.cooldown_remaining = self.cooldown_remaining.saturating_sub(dt);
    }
}

/// 碰撞层
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionLayer {
    Ship,
    Bullet,
    Effect,
}

unsafe impl hecs::Component for CollisionLayer {}

impl CollisionLayer {
    pub fn can_collide_with(&self, other: &Self) -> bool {
        match (self, other) {
            (CollisionLayer::Ship, CollisionLayer::Bullet) => true,
            (CollisionLayer::Bullet, CollisionLayer::Ship) => true,
            (CollisionLayer::Ship, CollisionLayer::Ship) => true,
            _ => false,
        }
    }
}

/// 碰撞体组件
#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub radius: f32,
    pub layer: CollisionLayer,
}

unsafe impl hecs::Component for Collider {}

/// 子弹组件
#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    pub shooter: hecs::Entity,
    pub lifetime: Duration,
    pub damage: f32,
}

unsafe impl hecs::Component for Bullet {}

/// 渲染层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Background = 0,
    Boundary = 1,
    Nebula = 2,
    Bullet = 3,
    Ship = 4,
    Effect = 5,
}

unsafe impl hecs::Component for RenderLayer {}

/// 可渲染组件
#[derive(Debug, Clone, Copy)]
pub struct Renderable {
    pub color: [f32; 4],
    pub layer: RenderLayer,
    pub visible: bool,
}

unsafe impl hecs::Component for Renderable {}

/// AI行为状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBehaviorState {
    Idle,
    Seeking,
    Chasing,
    Attacking,
    Retreating,
}

unsafe impl hecs::Component for AiBehaviorState {}

/// AI状态组件
#[derive(Debug, Clone)]
pub struct AiState {
    pub current_state: AiBehaviorState,
    pub target: Option<hecs::Entity>,
    pub target_lock_timer: Duration,
}

unsafe impl hecs::Component for AiState {}

impl Default for AiState {
    fn default() -> Self {
        Self {
            current_state: AiBehaviorState::Idle,
            target: None,
            target_lock_timer: Duration::ZERO,
        }
    }
}

/// 特效组件
#[derive(Debug, Clone, Copy)]
pub struct Effect {
    pub lifetime: Duration,
    pub max_lifetime: Duration,
    pub start_scale: f32,
    pub end_scale: f32,
}

unsafe impl hecs::Component for Effect {}

/// 重生计时器组件
#[derive(Debug, Clone, Copy)]
pub struct RespawnTimer {
    pub remaining: Duration,
}

unsafe impl hecs::Component for RespawnTimer {}

impl RespawnTimer {
    pub fn new(delay: Duration) -> Self {
        Self { remaining: delay }
    }
}
