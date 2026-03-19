#!/usr/bin/env python3
"""
ConwayDuel 项目完整修复脚本
修复所有编译错误

问题分析：
1. combat/mod.rs: target_pos - position 类型不匹配（Vec2 - &Vec2）
2. physics/mod.rs: 未使用的变量 new_velocity
"""

import os

# ============== src/core/combat/mod.rs ==============
COMBAT_MOD_RS = '''use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::{Faction, GameConfig};
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};

/// 武器系统：处理飞船射击逻辑
pub fn weapon_system(world: &mut World, dt: Duration, _event_bus: &mut EventBus, config: &GameConfig) {
    // 收集射击信息
    let mut bullets_to_spawn = Vec::new();
    
    // 先收集所有武器状态和AI状态
    let mut weapon_states: Vec<(hecs::Entity, Vec2, Faction, Weapon, AiState)> = Vec::new();
    for (entity, (transform, weapon, ai_state, faction)) in world.query::<(
        &Transform,
        &Weapon,
        &AiState,
        &FactionComponent,
    )>()
    .into_iter()
    {
        weapon_states.push((entity, transform.position, faction.faction, *weapon, ai_state.clone()));
    }

    for (entity, position, faction, weapon, ai_state) in &weapon_states {
        // 只有攻击状态且有目标才射击
        if ai_state.current_state != AiBehaviorState::Attacking {
            continue;
        }

        if let Some(target) = ai_state.target {
            // 获取目标位置
            if let Some(target_pos) = world.query_one::<&Transform>(target).ok().and_then(|mut q| q.get().map(|t| t.position)) {
                // 注意：position 是 &Vec2，target_pos 是 Vec2，需要解引用
                let direction = (target_pos - *position).normalize_or_zero();
                
                // 检查是否可以射击
                if weapon.cooldown_remaining.is_zero() {
                    bullets_to_spawn.push((
                        *position,
                        direction,
                        *entity,
                        *faction,
                        weapon.bullet_speed,
                        weapon.bullet_damage,
                        weapon.bullet_lifetime,
                    ));
                }
            }
        }
    }

    // 更新武器冷却
    for (_, weapon) in world.query::<&mut Weapon>().into_iter() {
        weapon.update(dt);
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
pub fn damage_system(world: &mut World, event_bus: &mut EventBus) {
    // 收集伤害事件
    let mut damage_events = Vec::new();
    for event in event_bus.events() {
        if let GameEvent::Hit { target, damage, .. } = event {
            damage_events.push((*target, *damage));
        }
    }

    // 应用伤害
    for (target, damage) in damage_events {
        if let Ok(health) = world.query_one_mut::<&mut Health>(target) {
            health.take_damage(damage);
        }
    }

    // 处理死亡
    let mut death_events = Vec::new();
    for (entity, (health, transform, faction)) in world.query::<(&Health, &Transform, &FactionComponent)>().into_iter() {
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

    // 更新特效生命周期并收集过期的
    for (entity, effect) in world.query::<&mut Effect>().into_iter() {
        effect.lifetime = effect.lifetime.saturating_sub(dt);
        if effect.lifetime.is_zero() {
            to_remove.push(entity);
        }
    }

    // 更新子弹生命周期并收集过期的
    for (entity, bullet) in world.query::<&mut Bullet>().into_iter() {
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
'''

# ============== src/core/physics/mod.rs ==============
PHYSICS_MOD_RS = '''use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::GameConfig;
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};
use crate::config::Faction;

// 移动系统：更新所有实体的位置、旋转，限制最大速度
pub fn movement_system(world: &mut World, dt: Duration) {
    let dt_secs = dt.as_secs_f32();

    for (_, (transform, velocity)) in world.query::<(&mut Transform, &mut Velocity)>().into_iter() {
        // 更新位置
        transform.position += velocity.linear * dt_secs;
        // 更新旋转，归一化角度避免溢出
        transform.rotation += velocity.angular * dt_secs;
        transform.rotation = transform.rotation.rem_euclid(std::f32::consts::TAU);
        // 限制最大速度，防止速度失控
        if velocity.linear.length_squared() > velocity.max_speed.powi(2) {
            velocity.linear = velocity.linear.normalize_or_zero() * velocity.max_speed;
        }
    }
}

// 边界系统：处理实体与战场边界的碰撞反弹，销毁出界子弹
pub fn boundary_system(world: &mut World, event_bus: &mut EventBus, config: &GameConfig) {
    let bounds_min = Vec2::ZERO;
    let bounds_max = Vec2::new(config.world_width, config.world_height);

    // 先收集所有需要处理的数据
    let mut boundary_collisions: Vec<(hecs::Entity, Vec2, Vec2)> = Vec::new();
    
    for (entity, (transform, velocity, collider, _faction)) in world.query::<(
        &Transform,
        &Velocity,
        &Collider,
        &FactionComponent,
    )>().into_iter()
    {
        let mut collision_normal = Vec2::ZERO;
        let radius = collider.radius;
        let mut new_pos = transform.position;

        // 左右边界检测
        if transform.position.x - radius < bounds_min.x {
            collision_normal.x = 1.0;
            new_pos.x = bounds_min.x + radius;
        } else if transform.position.x + radius > bounds_max.x {
            collision_normal.x = -1.0;
            new_pos.x = bounds_max.x - radius;
        }

        // 上下边界检测
        if transform.position.y - radius < bounds_min.y {
            collision_normal.y = 1.0;
            new_pos.y = bounds_min.y + radius;
        } else if transform.position.y + radius > bounds_max.y {
            collision_normal.y = -1.0;
            new_pos.y = bounds_max.y - radius;
        }

        if collision_normal != Vec2::ZERO {
            boundary_collisions.push((entity, collision_normal, new_pos));
            
            // 发布边界碰撞事件
            event_bus.publish(GameEvent::BoundaryCollision {
                entity,
                normal: collision_normal,
            });
        }
    }

    // 应用边界碰撞结果
    for (entity, collision_normal, new_pos) in boundary_collisions {
        if let Ok((transform, velocity)) = world.query_one_mut::<(&mut Transform, &mut Velocity)>(entity) {
            transform.position = new_pos;
            // 速度沿法线反射，实现反弹
            velocity.linear = velocity.linear - 2.0 * velocity.linear.dot(collision_normal) * collision_normal;
            // 应用阻尼
            velocity.linear *= config.ship_bounce_damping;
        }
    }

    // 销毁出界的子弹
    let mut out_of_bounds_bullets = Vec::new();
    for (entity, (transform, _bullet)) in world.query::<(&Transform, &Bullet)>().into_iter() {
        let pos = transform.position;
        if pos.x < bounds_min.x || pos.x > bounds_max.x
            || pos.y < bounds_min.y || pos.y > bounds_max.y
        {
            out_of_bounds_bullets.push(entity);
        }
    }

    for entity in out_of_bounds_bullets {
        let _ = world.despawn(entity);
    }
}

// 碰撞系统：检测实体间的碰撞，处理子弹命中、飞船间碰撞
pub fn collision_system(world: &mut World, event_bus: &mut EventBus, config: &GameConfig) {
    // 收集所有碰撞体信息
    let mut colliders: Vec<(hecs::Entity, Vec2, Collider, Option<Faction>)> = Vec::new();
    for (entity, (transform, collider, faction)) in world.query::<(&Transform, &Collider, Option<&FactionComponent>)>().into_iter() {
        let faction_val = faction.map(|f| f.faction);
        colliders.push((entity, transform.position, *collider, faction_val));
    }

    // 收集需要处理的事件
    let mut hits_to_process: Vec<(hecs::Entity, hecs::Entity, f32, Vec2)> = Vec::new();
    let mut bullets_to_despawn: Vec<hecs::Entity> = Vec::new();
    let mut ship_collisions: Vec<(hecs::Entity, hecs::Entity)> = Vec::new();

    // 两两碰撞检测
    for i in 0..colliders.len() {
        let (entity_a, pos_a, collider_a, faction_a) = colliders[i];
        for j in (i + 1)..colliders.len() {
            let (entity_b, pos_b, collider_b, faction_b) = colliders[j];

            // 判断碰撞层级是否允许碰撞
            if !collider_a.layer.can_collide_with(&collider_b.layer) {
                continue;
            }

            // 飞船间碰撞，检查配置是否开启
            if collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Ship {
                if !config.ship_ship_collision_enabled {
                    continue;
                }
            }

            // 子弹与飞船碰撞，禁止友军伤害
            if (collider_a.layer == CollisionLayer::Bullet && collider_b.layer == CollisionLayer::Ship)
                || (collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Bullet)
            {
                if let (Some(fa), Some(fb)) = (faction_a, faction_b) {
                    if !fa.is_enemy(&fb) {
                        continue;
                    }
                }
            }

            // 圆形碰撞检测
            let distance = pos_a.distance(pos_b);
            let min_collision_distance = collider_a.radius + collider_b.radius + config.collision_margin;

            // 发生碰撞
            if distance < min_collision_distance {
                // 子弹命中飞船
                if collider_a.layer == CollisionLayer::Bullet && collider_b.layer == CollisionLayer::Ship {
                    // 获取子弹信息 - 使用 map 直接获取值
                    if let Some(bullet) = world.query_one::<&Bullet>(entity_a).ok().and_then(|mut q| q.get().copied()) {
                        let damage = world.query_one::<&Weapon>(bullet.shooter).ok()
                            .and_then(|mut q| q.get().map(|w| w.bullet_damage))
                            .unwrap_or(config.bullet_damage);
                        hits_to_process.push((bullet.shooter, entity_b, damage, pos_a));
                        bullets_to_despawn.push(entity_a);
                    }
                }
                // 飞船被子弹命中
                else if collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Bullet {
                    if let Some(bullet) = world.query_one::<&Bullet>(entity_b).ok().and_then(|mut q| q.get().copied()) {
                        let damage = world.query_one::<&Weapon>(bullet.shooter).ok()
                            .and_then(|mut q| q.get().map(|w| w.bullet_damage))
                            .unwrap_or(config.bullet_damage);
                        hits_to_process.push((bullet.shooter, entity_a, damage, pos_b));
                        bullets_to_despawn.push(entity_b);
                    }
                }
                // 飞船间碰撞
                else if collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Ship {
                    ship_collisions.push((entity_a, entity_b));
                }
            }
        }
    }

    // 处理命中事件
    for (attacker, target, damage, position) in hits_to_process {
        event_bus.publish(GameEvent::Hit {
            attacker,
            target,
            damage,
            position,
        });
    }

    // 销毁命中的子弹
    for entity in bullets_to_despawn {
        let _ = world.despawn(entity);
    }

    // 处理飞船间碰撞
    for (entity_a, entity_b) in ship_collisions {
        // 获取位置计算碰撞法线
        let pos_a = world.query_one::<&Transform>(entity_a).ok()
            .and_then(|mut q| q.get().map(|t| t.position))
            .unwrap_or(Vec2::ZERO);
        let pos_b = world.query_one::<&Transform>(entity_b).ok()
            .and_then(|mut q| q.get().map(|t| t.position))
            .unwrap_or(Vec2::ZERO);
        let collision_normal = (pos_a - pos_b).normalize_or_zero();
        
        // 给两个飞船施加反向的速度
        if let Ok(vel_a) = world.query_one_mut::<&mut Velocity>(entity_a) {
            vel_a.linear += collision_normal * 0.5;
        }
        if let Ok(vel_b) = world.query_one_mut::<&mut Velocity>(entity_b) {
            vel_b.linear -= collision_normal * 0.5;
        }
    }
}
'''

def write_file(filepath, content):
    """写入文件"""
    os.makedirs(os.path.dirname(filepath), exist_ok=True)
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"已写入: {filepath}")

def main():
    print("=" * 60)
    print("ConwayDuel 项目修复脚本")
    print("=" * 60)
    print()
    
    # 获取项目根目录
    script_dir = os.path.dirname(os.path.abspath(__file__))
    
    # 写入修复后的文件
    write_file(os.path.join(script_dir, 'src/core/combat/mod.rs'), COMBAT_MOD_RS)
    write_file(os.path.join(script_dir, 'src/core/physics/mod.rs'), PHYSICS_MOD_RS)
    
    print()
    print("=" * 60)
    print("修复完成！")
    print("修复内容：")
    print("1. combat/mod.rs: 修复 target_pos - *position 类型问题")
    print("2. physics/mod.rs: 删除未使用的 new_velocity 变量")
    print("=" * 60)

if __name__ == '__main__':
    main()
