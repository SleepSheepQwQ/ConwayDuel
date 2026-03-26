use glam::Vec2;
use std::time::Duration;
use crate::config::{Faction, GameConfig};

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub linear: Vec2,
    pub angular: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Ship {
    pub health: f32,
    pub max_health: f32,
    pub faction: Faction,
}

#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub radius: f32,
    pub layer: CollisionLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionLayer {
    Ship,
    Bullet,
}

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    pub shooter: hecs::Entity,
    pub lifetime: Duration,
    pub damage: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Background = 0,
    Boundary = 1,
    Nebula = 2,
    Bullet = 3,
    Ship = 4,
    Effect = 5,
}

#[derive(Debug, Clone, Copy)]
pub struct Renderable {
    pub color: [f32; 4],
    pub layer: RenderLayer,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBehaviorState {
    Idle,
    Seeking,
    Chasing,
    Attacking,
    Retreating,
}

#[derive(Debug, Clone, Copy)]
pub struct AiState {
    pub current_state: AiBehaviorState,
    pub target: Option<hecs::Entity>,
    pub target_lock_timer: Duration,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            current_state: AiBehaviorState::Idle,
            target: None,
            target_lock_timer: Duration::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Weapon {
    pub cooldown: Duration,
    pub remaining_cooldown: Duration,
    pub active: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Effect {
    pub lifetime: Duration,
    pub max_lifetime: Duration,
    pub start_scale: f32,
    pub end_scale: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct RespawnTimer {
    pub remaining: Duration,
}

impl RespawnTimer {
    pub fn new(delay: Duration) -> Self {
        Self { remaining: delay }
    }
}
