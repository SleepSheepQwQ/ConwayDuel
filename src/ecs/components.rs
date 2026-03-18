use glam::Vec2;
use hecs::Component;
use std::time::Duration;

use crate::config::{Faction, GameConfig};

// 变换组件：所有实体必备，定义位置、旋转、缩放
#[derive(Debug, Clone, Copy, Component)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32, // 单位：弧度，0为向右
    pub scale: Vec2,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

impl Transform {
    // 获取当前朝向的单位向量
    pub fn forward(&self) -> Vec2 {
        Vec2::new(self.rotation.cos(), self.rotation.sin())
    }
}

// 速度组件：定义实体的线性运动和旋转运动
#[derive(Debug, Clone, Copy, Component)]
pub struct Velocity {
    pub linear: Vec2,    // 线速度
    pub angular: f32,    // 角速度（弧度/秒）
    pub max_speed: f32,  // 最大线速度限制
}

impl Default for Velocity {
    fn default() -> Self {
        Self {
            linear: Vec2::ZERO,
            angular: 0.0,
            max_speed: f32::INFINITY,
        }
    }
}

// 生命组件：管理实体血量和死亡状态
#[derive(Debug, Clone, Copy, Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    pub is_dead: bool,
}

impl Health {
    pub fn new(max_health: f32) -> Self {
        Self {
            current: max_health,
            max: max_health,
            is_dead: false,
        }
    }

    // 受到伤害，返回是否死亡
    pub fn take_damage(&mut self, damage: f32) -> bool {
        self.current = (self.current - damage).max(0.0);
        self.is_dead = self.current <= 0.0;
        self.is_dead
    }
}

// 阵营组件：标记实体所属阵营，用于敌我判断
#[derive(Debug, Clone, Copy, Component)]
pub struct FactionComponent {
    pub faction: Faction,
}

// 武器组件：管理射击冷却、子弹属性
#[derive(Debug, Clone, Copy, Component)]
pub struct Weapon {
    pub cooldown: Duration,
    pub current_cooldown: Duration,
    pub bullet_speed: f32,
    pub bullet_damage: f32,
}

impl Weapon {
    // 从全局配置生成默认武器
    pub fn from_config(config: &GameConfig) -> Self {
        Self {
            cooldown: Duration::from_secs_f32(config.fire_cooldown),
            current_cooldown: Duration::ZERO,
            bullet_speed: config.ship_max_speed * config.bullet_speed_multiplier,
            bullet_damage: config.bullet_damage,
        }
    }

    // 更新冷却，返回是否可以开火
    pub fn update(&mut self, dt: Duration) -> bool {
        if self.current_cooldown > Duration::ZERO {
            self.current_cooldown = self.current_cooldown.saturating_sub(dt);
        }
        self.current_cooldown.is_zero()
    }

    // 开火，重置冷却
    pub fn fire(&mut self) {
        self.current_cooldown = self.cooldown;
    }
}

// 碰撞体组件：定义碰撞形状、层级
#[derive(Debug, Clone, Copy, Component)]
pub struct Collider {
    pub radius: f32,
    pub layer: CollisionLayer,
}

// 碰撞层级：定义哪些实体之间可以碰撞
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionLayer {
    Ship,
    Bullet,
}

impl CollisionLayer {
    // 判断两个层级是否允许碰撞
    pub fn can_collide_with(&self, other: &Self) -> bool {
        match (self, other) {
            (CollisionLayer::Ship, CollisionLayer::Bullet) => true,
            (CollisionLayer::Bullet, CollisionLayer::Ship) => true,
            (CollisionLayer::Ship, CollisionLayer::Ship) => true,
            _ => false,
        }
    }
}

// 可渲染组件：定义渲染属性、层级、可见性
#[derive(Debug, Clone, Component)]
pub struct Renderable {
    pub color: [f32; 4],
    pub layer: RenderLayer,
    pub visible: bool,
}

// 渲染层级：从后往前渲染，保证层级正确
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Background = 0,
    Boundary = 1,
    Bullet = 2,
    Ship = 3,
    Effect = 4,
}

impl Default for Renderable {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            layer: RenderLayer::Ship,
            visible: true,
        }
    }
}

// AI状态组件：管理AI行为状态、目标锁定
#[derive(Debug, Clone, Component)]
pub struct AiState {
    pub current_state: AiBehaviorState,
    pub target: Option<hecs::Entity>,
    pub target_lock_timer: Duration,
    pub desired_direction: Vec2,
    pub should_fire: bool,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            current_state: AiBehaviorState::Seeking,
            target: None,
            target_lock_timer: Duration::ZERO,
            desired_direction: Vec2::X,
            should_fire: false,
        }
    }
}

// AI行为状态枚举：分层状态机核心
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBehaviorState {
    Seeking,    // 索敌状态
    Chasing,    // 追击状态
    Attacking,  // 攻击状态
    Evading,    // 规避状态
    Retreating, // 撤退状态
}

// 子弹组件：标记子弹实体，管理生命周期
#[derive(Debug, Clone, Copy, Component)]
pub struct Bullet {
    pub lifetime: Duration,
    pub max_lifetime: Duration,
    pub shooter: hecs::Entity,
}

// 特效组件：管理爆炸、尾焰等特效的生命周期
#[derive(Debug, Clone, Copy, Component)]
pub struct Effect {
    pub lifetime: Duration,
    pub max_lifetime: Duration,
    pub start_scale: f32,
    pub end_scale: f32,
}
