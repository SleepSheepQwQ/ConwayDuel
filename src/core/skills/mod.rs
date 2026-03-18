// 技能系统模块 - 预留扩展接口
// 未来可在此实现各种技能效果

/// 技能类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillType {
    Shield,
    SpeedBoost,
    ScatterShot,
    HomingMissile,
}

/// 技能组件
#[derive(Debug, Clone)]
pub struct Skill {
    pub skill_type: SkillType,
    pub cooldown: std::time::Duration,
    pub remaining_cooldown: std::time::Duration,
    pub duration: std::time::Duration,
    pub active: bool,
}

impl Skill {
    pub fn new(skill_type: SkillType, cooldown: std::time::Duration, duration: std::time::Duration) -> Self {
        Self {
            skill_type,
            cooldown,
            remaining_cooldown: std::time::Duration::ZERO,
            duration,
            active: false,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.remaining_cooldown.is_zero() && !self.active
    }

    pub fn activate(&mut self) {
        if self.is_ready() {
            self.active = true;
            self.remaining_cooldown = self.cooldown;
        }
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.remaining_cooldown = self.remaining_cooldown.saturating_sub(dt);
    }
}
