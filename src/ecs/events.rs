use glam::Vec2;
use hecs::Entity;

#[derive(Debug, Clone)]
pub enum GameEvent {
    Death {
        position: Vec2,
        faction: crate::config::Faction,
    },
    Collision {
        entity_a: Entity,
        entity_b: Entity,
    },
    Hit {
        target: Entity,
        damage: f32,
    },
}

#[derive(Debug, Default)]
pub struct EventBus {
    events: Vec<GameEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[GameEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}
