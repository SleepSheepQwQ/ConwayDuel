use glam::Vec2;
use hecs::Entity;
use std::collections::VecDeque;

use crate::config::Faction;

// 全量游戏事件枚举，覆盖所有游戏节点，无侵入式扩展
#[derive(Debug, Clone)]
pub enum GameEvent {
    // 开火事件：武器开火时触发
    Fire {
        shooter: Entity,
        position: Vec2,
        direction: Vec2,
        faction: Faction,
    },
    // 命中事件：子弹击中目标时触发
    Hit {
        attacker: Entity,
        target: Entity,
        damage: f32,
        position: Vec2,
    },
    // 死亡事件：实体血量清零时触发
    Death {
        entity: Entity,
        position: Vec2,
        faction: Faction,
        killer: Option<Entity>,
    },
    // 爆炸事件：实体死亡/特效触发时发布
    Explosion {
        position: Vec2,
        radius: f32,
        color: [f32; 4],
    },
    // 边界碰撞事件：实体碰到战场边界时触发
    BoundaryCollision {
        entity: Entity,
        position: Vec2,
        normal: Vec2,
    },
}

// 全局事件总线，单帧内发布的事件统一处理，帧结束后清空
#[derive(Debug, Default)]
pub struct EventBus {
    events: VecDeque<GameEvent>,
}

impl EventBus {
    // 发布事件，所有订阅者都能收到
    pub fn publish(&mut self, event: GameEvent) {
        self.events.push_back(event);
    }

    // 遍历当前帧所有事件
    pub fn iter(&self) -> impl Iterator<Item = &GameEvent> {
        self.events.iter()
    }

    // 清空事件队列，每一帧固定更新结束后调用
    pub fn clear(&mut self) {
        self.events.clear();
    }
}
