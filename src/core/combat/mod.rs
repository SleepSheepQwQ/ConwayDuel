use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::{Faction, GameConfig};
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};

pub fn weapon_system(
    world: &mut World,
    dt: Duration,
    events: &mut EventBus,
    config: &GameConfig,
) {
    let dt_secs = dt.as_secs_f32();

    for (entity, (transform, ship, weapon, ai)) in
        world.query_mut::<(&Transform, &Ship, &mut Weapon, &AiState)>()
    {
        weapon.remaining_cooldown = weapon.remaining_cooldown.saturating_sub(dt);

        let should_fire = ai.current_state == AiBehaviorState::Attacking
            && weapon.remaining_cooldown.is_zero()
            && ai.target.is_some();

        if should_fire {
            weapon.remaining_cooldown = weapon.cooldown;

            let direction = Vec2::new(transform.rotation.cos(), transform.rotation.sin());
            let bullet_speed = config.ship_max_speed * config.bullet_speed_multiplier;
            let bullet_pos = transform.position + direction * config.ship_size;

            world.spawn((
                Transform {
                    position: bullet_pos,
                    rotation: transform.rotation,
                    scale: Vec2::ONE,
                },
                Velocity {
                    linear: direction * bullet_speed,
                    angular: 0.0,
                },
                Bullet {
                    shooter: entity,
                    lifetime: Duration::from_secs_f32(config.bullet_lifetime),
                    damage: config.bullet_damage,
                },
                Collider {
                    radius: config.bullet_size * 0.5,
                    layer: CollisionLayer::Bullet,
                },
                Renderable {
                    color: ship.faction.to_color(),
                    layer: RenderLayer::Bullet,
                    visible: true,
                },
            ));
        }
    }
}

pub fn damage_system(world: &mut World, events: &EventBus) {
    let mut to_remove: Vec<hecs::Entity> = Vec::new();
    let mut to_despawn: Vec<hecs::Entity> = Vec::new();

    for event in events.events() {
        if let GameEvent::Collision { entity_a, entity_b } = event {
            let (bullet_entity, ship_entity) =
                if world.get::<&Bullet>(*entity_a).is_ok() {
                    (*entity_a, *entity_b)
                } else if world.get::<&Bullet>(*entity_b).is_ok() {
                    (*entity_b, *entity_a)
                } else {
                    continue;
                };

            let bullet_damage = if let Ok(bullet) = world.get::<&Bullet>(bullet_entity) {
                bullet.damage
            } else {
                continue;
            };

            let shooter = if let Ok(bullet) = world.get::<&Bullet>(bullet_entity) {
                bullet.shooter
            } else {
                continue;
            };

            if ship_entity == shooter {
                continue;
            }

            let should_die = if let Ok(mut ship) = world.get::<&mut Ship>(ship_entity) {
                ship.health -= bullet_damage;
                ship.health <= 0.0
            } else {
                continue;
            };

            to_remove.push(bullet_entity);

            if should_die {
                let (faction, position) = if let Ok(ship) = world.get::<&Ship>(ship_entity) {
                    let faction = ship.faction;
                    let position = if let Ok(t) = world.get::<&Transform>(ship_entity) {
                        t.position
                    } else {
                        continue;
                    };
                    (faction, position)
                } else {
                    continue;
                };

                world.despawn(ship_entity).ok();

                world.spawn((
                    RespawnTimer::new(Duration::from_secs_f32(3.0)),
                ));

                let color = faction.to_color();
                events.push(GameEvent::Death { position, faction });

                world.spawn((
                    Transform {
                        position,
                        rotation: 0.0,
                        scale: Vec2::ONE,
                    },
                    Effect {
                        lifetime: Duration::from_secs_f32(0.5),
                        max_lifetime: Duration::from_secs_f32(0.5),
                        start_scale: 1.0,
                        end_scale: 3.0,
                    },
                    Renderable {
                        color: [color[0], color[1], color[2], 0.8],
                        layer: RenderLayer::Effect,
                        visible: true,
                    },
                ));
            }
        }
    }

    for entity in to_remove {
        world.despawn(entity).ok();
    }
    for entity in to_despawn {
        world.despawn(entity).ok();
    }
}

pub fn cleanup_system(world: &mut World, dt: Duration) {
    let mut to_remove: Vec<hecs::Entity> = Vec::new();

    for (entity, bullet) in world.query::<&Bullet>().iter() {
        let mut b = *bullet;
        b.lifetime = b.lifetime.saturating_sub(dt);
        if b.lifetime.is_zero() {
            to_remove.push(entity);
        }
    }

    for (entity, effect) in world.query::<&Effect>().iter() {
        let mut eff = *effect;
        eff.lifetime = eff.lifetime.saturating_sub(dt);
        if eff.lifetime.is_zero() {
            to_remove.push(entity);
        }
    }

    for entity in to_remove {
        world.despawn(entity).ok();
    }
}

pub fn spawn_explosion(world: &mut World, position: Vec2, faction: Faction) {
    let color = faction.to_color();
    world.spawn((
        Transform {
            position,
            rotation: 0.0,
            scale: Vec2::ONE,
        },
        Effect {
            lifetime: Duration::from_secs_f32(0.5),
            max_lifetime: Duration::from_secs_f32(0.5),
            start_scale: 1.0,
            end_scale: 3.0,
        },
        Renderable {
            color: [color[0], color[1], color[2], 0.8],
            layer: RenderLayer::Effect,
            visible: true,
        },
    ));
}
