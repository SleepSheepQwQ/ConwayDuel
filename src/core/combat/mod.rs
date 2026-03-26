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
                    scale: Vec2::new(config.bullet_size, config.bullet_size),
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
    for event in events.events() {
        if let GameEvent::Collision { entity_a, entity_b } = event {
            if let Ok(bullet) = world.get::<&Bullet>(*entity_a) {
                let bullet_entity = *entity_a;
                let ship_entity = *entity_b;
                process_bullet_hit(world, bullet_entity, ship_entity, bullet.damage);
            } else if let Ok(bullet) = world.get::<&Bullet>(*entity_b) {
                let bullet_entity = *entity_b;
                let ship_entity = *entity_a;
                process_bullet_hit(world, bullet_entity, ship_entity, bullet.damage);
            }
        }
    }
}

fn process_bullet_hit(world: &mut World, bullet_entity: hecs::Entity, ship_entity: hecs::Entity, damage: f32) {
    let shooter_faction = match world.get::<&Bullet>(bullet_entity) {
        Ok(bullet) => {
            world.get::<&Ship>(bullet.shooter).ok().map(|s| s.faction)
        }
        Err(_) => return,
    };

    let target_faction = match world.get::<&Ship>(ship_entity) {
        Ok(ship) => ship.faction,
        Err(_) => return,
    };

    if let Some(faction) = shooter_faction {
        if faction == target_faction {
            return;
        }
    }

    let should_die = if let Ok(mut ship) = world.query_one_mut::<&mut Ship>(ship_entity) {
        ship.health -= damage;
        ship.health <= 0.0
    } else {
        false
    };

    world.despawn(bullet_entity).ok();

    if should_die {
        let (faction, position) = match world.get::<(&Ship, &Transform)>(ship_entity) {
            Ok((ship, transform)) => (ship.faction, transform.position),
            Err(_) => return,
        };

        world.despawn(ship_entity).ok();

        world.spawn((
            RespawnTimer::new(Duration::from_secs_f32(3.0)),
            Transform { position, rotation: 0.0, scale: Vec2::ONE },
            Ship { health: 0.0, max_health: 100.0, faction },
        ));
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
            start_scale: 0.5,
            end_scale: 3.0,
        },
        Renderable {
            color: [color[0], color[1], color[2], 0.8],
            layer: RenderLayer::Effect,
            visible: true,
        },
    ));
}

pub fn cleanup_system(world: &mut World, dt: Duration) {
    let mut to_remove: Vec<hecs::Entity> = Vec::new();

    for (entity, bullet) in world.query::<&Bullet>().iter() {
        let mut lifetime = *bullet;
        lifetime.lifetime = lifetime.lifetime.saturating_sub(dt);
        if lifetime.lifetime.is_zero() {
            to_remove.push(entity);
        } else {
            if let Ok(mut b) = world.query_one_mut::<&mut Bullet>(entity) {
                b.lifetime = lifetime.lifetime;
            }
        }
    }

    for (entity, effect) in world.query::<&Effect>().iter() {
        let mut eff = *effect;
        eff.lifetime = eff.lifetime.saturating_sub(dt);
        if eff.lifetime.is_zero() {
            to_remove.push(entity);
        } else {
            if let Ok(mut e) = world.query_one_mut::<&mut Effect>(entity) {
                e.lifetime = eff.lifetime;
                let progress = 1.0 - (eff.lifetime.as_secs_f32() / eff.max_lifetime.as_secs_f32());
                let scale = eff.start_scale + (eff.end_scale - eff.start_scale) * progress;
                if let Ok(mut t) = world.query_one_mut::<&mut Transform>(entity) {
                    t.scale = Vec2::splat(scale);
                }
            }
        }
    }

    for entity in to_remove {
        world.despawn(entity).ok();
    }
}
