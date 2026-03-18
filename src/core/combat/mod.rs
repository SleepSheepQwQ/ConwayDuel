use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::{Faction, GameConfig};
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};

/// 武器系统：处理飞船射击逻辑
pub fn weapon_system(world: &mut World, dt: Duration, event_bus: &mut EventBus, config: &GameConfig) {
    // 收集射击信息
    let mut bullets_to_spawn = Vec::new();

    for (entity, (transform, _velocity, mut weapon, ai_state, faction)) in world.query::<(
        &Transform,
        &Velocity,
        &mut Weapon,
        &AiState,
        &FactionComponent,
    )>()
    .iter()
    {
        // 更新武器冷却
        weapon.update(dt);

        // 只有攻击状态且有目标才射击
        if ai_state.current_state != AiBehaviorState::Attacking {
            continue;
        }

        if let Some(target) = ai_state.target {
            if let Ok(target_transform) = world.query_one::<&Transform>(target).get() {
                let direction = (target_transform.position - transform.position).normalize_or_zero();
                
                // 检查是否可以射击
                if weapon.can_fire() {
                    weapon.fire();
                    bullets_to_spawn.push((
                        transform.position,
                        direction,
                        entity,
                        faction.faction,
                        weapon.bullet_speed,
                        weapon.bullet_damage,
                        weapon.bullet_lifetime,
                    ));
                }
            }
        }
    }

    // 生成子弹
    for (position, direction, shooter, faction, speed, damage, lifetime) in bullets_to_spawn {
        spawn_bullet(world, config, position, direction, shooter, faction, speed, damage, lifetime);
    }
}

/// 生成子弹
fn spawn_bullet(
    world: &mut World,
    config: &GameConfig,
    position: Vec2,
    direction: Vec2,
    shooter: hecs::Entity,
    faction: Faction,
    speed: f32,
    damage: f32,
    lifetime: Duration,
) {
    let transform = Transform {
        position,
        rotation: direction.y.atan2(direction.x),
        scale: Vec2::splat(config.bullet_size),
    };

    let velocity = Velocity {
        linear: direction * speed,
        angular: 0.0,
        max_speed: speed,
    };

    let bullet = Bullet {
        shooter,
        lifetime,
        damage,
    };

    let collider = Collider {
        radius: config.bullet_size,
        layer: CollisionLayer::Bullet,
    };

    let renderable = Renderable {
        color: faction.to_color(),
        layer: RenderLayer::Bullet,
        visible: true,
    };

    world.spawn((transform, velocity, bullet, collider, renderable));
}

/// 伤害系统：处理命中伤害
pub fn damage_system(world: &mut World, event_bus: &EventBus) {
    // 收集伤害事件
    let mut damage_events = Vec::new();
    for event in event_bus.events() {
        if let GameEvent::Hit { target, damage, .. } = event {
            damage_events.push((*target, *damage));
        }
    }

    // 应用伤害
    for (target, damage) in damage_events {
        if let Ok(mut health) = world.query_one_mut::<&mut Health>(target) {
            health.take_damage(damage);
        }
    }

    // 处理死亡
    let mut death_events = Vec::new();
    for (entity, (health, transform, faction)) in world.query::<(&Health, &Transform, &FactionComponent)>().iter() {
        if health.is_dead {
            death_events.push((entity, transform.position, faction.faction));
        }
    }

    // 发布死亡事件
    for (entity, position, faction) in death_events {
        event_bus.publish(GameEvent::Death { position, faction });
        let _ = world.despawn(entity);
    }
}

/// 生成爆炸特效
pub fn spawn_explosion(world: &mut World, position: &Vec2, faction: Faction, config: &GameConfig) {
    let transform = Transform {
        position: *position,
        rotation: 0.0,
        scale: Vec2::splat(config.ship_size),
    };

    let effect = Effect {
        lifetime: Duration::from_secs_f32(0.5),
        max_lifetime: Duration::from_secs_f32(0.5),
        start_scale: config.ship_size * 0.5,
        end_scale: config.ship_size * 2.0,
    };

    let renderable = Renderable {
        color: faction.to_color(),
        layer: RenderLayer::Effect,
        visible: true,
    };

    world.spawn((transform, effect, renderable));
}

/// 清理系统：移除死亡实体和过期特效
pub fn cleanup_system(world: &mut World, dt: Duration) {
    // 收集需要移除的实体
    let mut to_remove = Vec::new();

    // 移除过期特效
    for (entity, mut effect) in world.query::<&mut Effect>().iter() {
        effect.lifetime = effect.lifetime.saturating_sub(dt);
        if effect.lifetime.is_zero() {
            to_remove.push(entity);
        }
    }

    // 移除过期子弹
    for (entity, mut bullet) in world.query::<&mut Bullet>().iter() {
        bullet.lifetime = bullet.lifetime.saturating_sub(dt);
        if bullet.lifetime.is_zero() {
            to_remove.push(entity);
        }
    }

    // 移除实体
    for entity in to_remove {
        let _ = world.despawn(entity);
    }
}
