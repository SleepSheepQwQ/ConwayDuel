use glam::Vec2;
use hecs::World;
use std::time::Duration;
use crate::config::{Faction, GameConfig};
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};
pub fn weapon_system(
    world: &mut World,
    dt: Duration,
    _events: &mut EventBus,
    config: &GameConfig,
) {
    // 预收集发射动作，避免query_mut时同时spawn的借用冲突
    let mut fire_actions: Vec<(hecs::Entity, Transform, Ship)> = Vec::new();
    
    // 先更新冷却，收集发射指令
    for (entity, (transform, ship, weapon, ai)) in
        world.query_mut::<(&Transform, &Ship, &mut Weapon, &AiState)>()
    {
        weapon.remaining_cooldown = weapon.remaining_cooldown.saturating_sub(dt);
        
        let should_fire = ai.current_state == AiBehaviorState::Attacking
            && weapon.remaining_cooldown.is_zero()
            && ai.target.is_some();
        
        if should_fire {
            fire_actions.push((entity, *transform, *ship));
            weapon.remaining_cooldown = weapon.cooldown;
        }
    }
    // 统一生成子弹
    for (shooter_entity, transform, ship) in fire_actions {
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
                shooter: shooter_entity,
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
pub fn damage_system(world: &mut World, events: &mut EventBus) {
    let mut to_remove_bullets: Vec<hecs::Entity> = Vec::new();
    let mut death_events: Vec<(Vec2, Faction)> = Vec::new();
    // 预收集所有命中事件，避免借用冲突
    let mut hit_events: Vec<(hecs::Entity, hecs::Entity, f32)> = Vec::new();
    for event in events.events() {
        if let GameEvent::Collision { entity_a, entity_b } = event {
            // 区分子弹和飞船
            let (bullet_entity, ship_entity) = if world.get::<&Bullet>(*entity_a).is_ok() {
                (*entity_a, *entity_b)
            } else if world.get::<&Bullet>(*entity_b).is_ok() {
                (*entity_b, *entity_a)
            } else {
                continue;
            };
            // 校验子弹有效性
            let Ok(bullet) = world.get::<&Bullet>(bullet_entity) else {
                continue;
            };
            if ship_entity == bullet.shooter {
                continue;
            }
            if world.get::<&Ship>(ship_entity).is_err() {
                continue;
            }
            hit_events.push((bullet_entity, ship_entity, bullet.damage));
        }
    }
    // 预收集所有需要处理的飞船伤害和死亡信息
    let mut ship_damage: Vec<(hecs::Entity, f32)> = Vec::new();
    for (bullet_entity, ship_entity, damage) in hit_events {
        to_remove_bullets.push(bullet_entity);
        ship_damage.push((ship_entity, damage));
    }
    // 统一处理伤害，预收集死亡信息
    let mut to_remove_ships: Vec<hecs::Entity> = Vec::new();
    let mut to_spawn_respawn: Vec<Faction> = Vec::new();
    let mut to_spawn_explosion: Vec<(Vec2, Faction)> = Vec::new();
    for (ship_entity, damage) in ship_damage {
        let Ok(mut ship) = world.get::<&mut Ship>(ship_entity) else {
            continue;
        };
        ship.health -= damage;
        if ship.health <= 0.0 {
            // 提前获取死亡所需的所有信息，避免后续借用冲突
            let faction = ship.faction;
            let Ok(transform) = world.get::<&Transform>(ship_entity) else {
                continue;
            };
            let position = transform.position;
            // 标记要销毁的飞船
            to_remove_ships.push(ship_entity);
            // 标记要生成的重生计时器、爆炸特效
            to_spawn_respawn.push(faction);
            to_spawn_explosion.push((position, faction));
            death_events.push((position, faction));
        }
    }
    // 统一销毁实体（子弹+死亡飞船）
    for entity in to_remove_bullets.into_iter().chain(to_remove_ships) {
        world.despawn(entity).ok();
    }
    // 统一生成重生计时器
    for faction in to_spawn_respawn {
        world.spawn((RespawnTimer::new(Duration::from_secs_f32(3.0), faction),));
    }
    // 统一生成死亡爆炸特效
    for (position, faction) in to_spawn_explosion {
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
    // 推送死亡事件
    for (position, faction) in death_events {
        events.push(GameEvent::Death { position, faction });
    }
}
pub fn cleanup_system(world: &mut World, dt: Duration) {
    // 预收集子弹更新/销毁信息
    let mut bullet_updates: Vec<(hecs::Entity, Duration)> = Vec::new();
    for (entity, bullet) in world.query::<&Bullet>().iter() {
        let remaining = bullet.lifetime.saturating_sub(dt);
        bullet_updates.push((entity, remaining));
    }
    let mut to_remove: Vec<hecs::Entity> = Vec::new();
    for (entity, remaining) in bullet_updates {
        if remaining.is_zero() {
            to_remove.push(entity);
        } else if let Ok(mut bullet) = world.get::<&mut Bullet>(entity) {
            bullet.lifetime = remaining;
        }
    }
    // 预收集特效更新/销毁信息
    let mut effect_updates: Vec<(hecs::Entity, Duration)> = Vec::new();
    for (entity, effect) in world.query::<&Effect>().iter() {
        let remaining = effect.lifetime.saturating_sub(dt);
        effect_updates.push((entity, remaining));
    }
    for (entity, remaining) in effect_updates {
        if remaining.is_zero() {
            to_remove.push(entity);
        } else if let Ok(mut effect) = world.get::<&mut Effect>(entity) {
            effect.lifetime = remaining;
        }
    }
    // 统一销毁实体
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
