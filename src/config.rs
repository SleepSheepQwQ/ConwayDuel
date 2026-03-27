use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub pixels_per_unit: f32,
    pub fixed_update_rate: f32,
    pub max_frame_accumulation: f32,
    pub ship_size: f32,
    pub ship_max_speed: f32,
    pub ship_turn_speed: f32,
    pub ship_max_health: f32,
    pub ship_bounce_damping: f32,
    pub bullet_speed_multiplier: f32,
    pub bullet_size: f32,
    pub bullet_damage: f32,
    pub bullet_lifetime: f32,
    pub ship_fire_rate: f32,
    pub ai_detection_range: f32,
    pub ai_engagement_range: f32,
    pub ai_flee_threshold: f32,
    pub ai_aggressiveness: f32,
    pub ai_target_lock_time: f32,
    pub collision_margin: f32,
    pub ship_ship_collision_enabled: bool,
    pub nebula_count: u32,
    pub blur_radius: i32,
    pub bloom_strength: f32,
    pub low_performance_mode: bool,
    pub respawn_delay: f32,
}
impl Default for GameConfig {
    fn default() -> Self {
        Self {
            world_width: 100.0,
            world_height: 60.0,
            pixels_per_unit: 10.0,
            fixed_update_rate: 60.0,
            max_frame_accumulation: 0.25,
            ship_size: 2.0,
            ship_max_speed: 20.0,
            ship_turn_speed: 3.0,
            ship_max_health: 100.0,
            ship_bounce_damping: 0.8,
            bullet_speed_multiplier: 2.0,
            bullet_size: 0.3,
            bullet_damage: 25.0,
            bullet_lifetime: 2.0,
            ship_fire_rate: 2.0,
            ai_detection_range: 30.0,
            ai_engagement_range: 20.0,
            ai_flee_threshold: 0.3,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    Red,
    Green,
    Blue,
}
impl Faction {
    pub fn to_color(&self) -> [f32; 4] {
        match self {
            Faction::Red => [1.0, 0.2, 0.2, 1.0],
            Faction::Green => [0.2, 1.0, 0.3, 1.0],
            Faction::Blue => [0.2, 0.4, 1.0, 1.0],
        }
    }
    pub fn is_enemy(&self, other: &Self) -> bool {
        self != other
    }
}
