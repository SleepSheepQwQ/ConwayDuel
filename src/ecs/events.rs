use glam::Vec2;
use hecs::Entity;
use crate::config::Faction;

/// 游戏事件枚举
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// 命中事件：攻击者、目标、伤害、位置
    Hit {
        attacker: Entity,
        target: Entity,
        damage: f32,
        position: Vec2,
    },
    /// 死亡事件：位置、阵营
    Death {
        position: Vec2,
        faction: Faction,
    },
    /// 边界碰撞事件：实体、碰撞法线
    BoundaryCollision {
        entity: Entity,
        normal: Vec2,
    },
    /// 爆炸事件：位置、阵营
    Explosion {
        position: Vec2,
        faction: Faction,
    },
}

/// 事件总线
#[derive(Debug, Default)]
pub struct EventBus {
    events: Vec<GameEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn publish(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[GameEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &GameEvent> {
        self.events.iter()
    }
}
