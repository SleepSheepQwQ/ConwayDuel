use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::{Faction, GameConfig};
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};

// 武器系统：处理武器冷却、开火逻辑、生成子弹
pub fn weapon_system(world: &mut World, dt: Duration, event_bus: &mut EventBus, config: &GameConfig) {
    let mut bullets_to_spawn = Vec::new();

    // 遍历所有带武器的飞船
    for (shooter, (transform, mut weapon, faction, ai_state)) in world.query::<(&Transform, &mut Weapon, &FactionComponent, &AiState)>()
        .with::<Health>()
        .iter()
    {
        // 更新武器冷却，判断是否可以开火
        let can_fire = weapon.update(dt);
        if !can_fire || !ai_state.should_fire {
            continue;
        }

        // 开火，重置冷却
        weapon.fire();
        let fire_direction = transform.forward();

        // 发布开火事件
        event_bus.publish(GameEvent::Fire {
            shooter,
            position: transform.position,
            direction: fire_direction,
            faction: faction.faction,
        });

        // 收集子弹生成信息，批量生成避免迭代中修改世界
        bullets_to_spawn.push((
            Transform {
                position: transform.position + fire_direction * (config.ship_size * 1.2),
                rotation: transform.rotation,
                scale: Vec2::splat(config.bullet_size),
            },
            Velocity {
                linear: fire_direction * weapon.bullet_speed,
                angular: 0.0,
                max_speed: weapon.bullet_speed,
            },
            Collider {
                radius: config.bullet_size / 2.0,
                layer: CollisionLayer::Bullet,
            },
            Renderable {
                color: faction.faction.to_color(),
                layer: RenderLayer::Bullet,
                visible: true,
            },
            Bullet {
                lifetime: Duration::ZERO,
                max_lifetime: Duration::from_secs_f32(config.bullet_lifetime),
                shooter,
            },
        ));
    }

    // 批量生成子弹，提升性能
    for bullet in bullets_to_spawn {
        world.spawn((bullet.0, bullet.1, bullet.2, bullet.3, bullet.4));
    }
}

// 伤害系统：处理命中事件，计算伤害，触发死亡
pub fn damage_system(world: &mut World, event_bus: &mut EventBus) {
    // 遍历当前帧所有命中事件
    for event in event_bus.iter() {
        let GameEvent::Hit { attacker, target, damage, position } = event else {
            continue;
        };

        // 获取目标的生命组件，扣血
        let mut health = match world.get_mut::<Health>(*target) {
            Ok(h) => h,
            Err(_) => continue,
        };

        // 扣血，判断是否死亡
        let is_dead = health.take_damage(*damage);
        if is_dead {
            // 获取目标阵营
            let faction = world.get::<FactionComponent>(*target)
                .map(|f| f.faction)
                .unwrap_or(Faction::Red);

            // 发布死亡事件
            event_bus.publish(GameEvent::Death {
                entity: *target,
                position: *position,
                faction,
                killer: Some(*attacker),
            });

            // 发布爆炸特效事件
            event_bus.publish(GameEvent::Explosion {
                position: *position,
                radius: 3.0,
                color: faction.to_color(),
            });
        }
    }
}

// 清理系统：销毁死亡实体、过期子弹、过期特效
pub fn cleanup_system(world: &mut World, dt: Duration) {
    let mut entities_to_despawn = Vec::new();

    // 收集死亡的飞船
    for (entity, health) in world.query::<&Health>().iter() {
        if health.is_dead {
            entities_to_despawn.push(entity);
        }
    }

    // 收集过期的子弹
    for (entity, mut bullet) in world.query::<&mut Bullet>().iter() {
        bullet.lifetime += dt;
        if bullet.lifetime >= bullet.max_lifetime {
            entities_to_despawn.push(entity);
        }
    }

    // 收集过期的特效
    for (entity, mut effect) in world.query::<&mut Effect>().iter() {
        effect.lifetime += dt;
        if effect.lifetime >= effect.max_lifetime {
            entities_to_despawn.push(entity);
        }
    }

    // 批量销毁实体，避免内存泄漏
    for entity in entities_to_despawn {
        let _ = world.despawn(entity);
    }
}
