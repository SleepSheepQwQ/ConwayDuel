use glam::Vec2;
use serde::{Deserialize, Serialize};

// 全局游戏配置，所有参数统一管理，修改这里即可调整全局玩法，无需改业务逻辑
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    // 世界与坐标系配置
    pub world_width: f32,
    pub world_height: f32,
    pub pixels_per_unit: f32,
    pub fixed_update_rate: f32,
    pub max_frame_accumulation: f32,

    // 飞船通用配置
    pub ship_size: f32,
    pub ship_max_speed: f32,
    pub ship_turn_speed: f32,
    pub ship_max_health: f32,
    pub ship_bounce_damping: f32,

    // 子弹配置（严格遵循子弹速度为飞船2倍的要求）
    pub bullet_speed_multiplier: f32,
    pub bullet_size: f32,
    pub bullet_damage: f32,
    pub bullet_lifetime: f32,

    // 武器冷却配置
    pub fire_cooldown: f32,

    // AI行为配置
    pub ai_view_range: f32,
    pub ai_attack_range: f32,
    pub ai_evade_threshold: f32,
    pub ai_aggressiveness: f32,
    pub ai_target_lock_time: f32,

    // 物理碰撞配置
    pub collision_margin: f32,
    pub ship_ship_collision_enabled: bool,

    // 渲染特效配置
    pub nebula_count: usize,
    pub blur_radius: i32,
    pub bloom_strength: f32,
    pub low_performance_mode: bool,

    // 飞船重生配置
    pub respawn_delay: f32,
}

// 默认配置，完全匹配项目需求，开箱即用
impl Default for GameConfig {
    fn default() -> Self {
        Self {
            world_width: 100.0,
            world_height: 60.0,
            pixels_per_unit: 16.0,
            fixed_update_rate: 30.0,
            max_frame_accumulation: 0.25,

            ship_size: 1.5,
            ship_max_speed: 8.0,
            ship_turn_speed: 3.0,
            ship_max_health: 100.0,
            ship_bounce_damping: 0.8,

            bullet_speed_multiplier: 2.0,
            bullet_size: 0.4,
            bullet_damage: 10.0,
            bullet_lifetime: 3.0,

            fire_cooldown: 0.5,

            ai_view_range: 30.0,
            ai_attack_range: 20.0,
            ai_evade_threshold: 0.3,
            ai_aggressiveness: 0.8,
            ai_target_lock_time: 1.0,

            collision_margin: 0.1,
            ship_ship_collision_enabled: true,

            nebula_count: 20,
            blur_radius: 4,
            bloom_strength: 0.6,
            low_performance_mode: false,

            respawn_delay: 3.0,
        }
    }
}

// 阵营枚举，三原色飞船，区分敌我
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    Red,
    Green,
    Blue,
}

impl Faction {
    // 获取阵营对应的RGBA颜色，用于渲染
    pub fn to_color(&self) -> [f32; 4] {
        match self {
            Faction::Red => [1.0, 0.2, 0.2, 1.0],
            Faction::Green => [0.2, 1.0, 0.3, 1.0],
            Faction::Blue => [0.2, 0.4, 1.0, 1.0],
        }
    }

    // 判断是否为敌方阵营
    pub fn is_enemy(&self, other: &Self) -> bool {
        self != other
    }
}
