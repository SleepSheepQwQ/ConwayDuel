use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::GameConfig;
use crate::ecs::components::*;
use crate::ecs::events::{EventBus, GameEvent};

// 移动系统：更新所有实体的位置、旋转，限制最大速度
pub fn movement_system(world: &mut World, dt: Duration) {
    let dt_secs = dt.as_secs_f32();

    for (_, (transform, velocity)) in world.query::<(&mut Transform, &Velocity)>().iter() {
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
pub fn boundary_system(world: &mut World, config: &GameConfig) {
    let bounds_min = Vec2::ZERO;
    let bounds_max = Vec2::new(config.world_width, config.world_height);

    // 处理飞船的边界碰撞与反弹
    for (entity, (transform, velocity, collider)) in world.query::<(&mut Transform, &mut Velocity, &Collider)>()
        .with::<FactionComponent>()
        .iter()
    {
        let mut collision_normal = Vec2::ZERO;
        let radius = collider.radius;

        // 左右边界检测
        if transform.position.x - radius < bounds_min.x {
            collision_normal.x = 1.0;
            transform.position.x = bounds_min.x + radius;
        } else if transform.position.x + radius > bounds_max.x {
            collision_normal.x = -1.0;
            transform.position.x = bounds_max.x - radius;
        }

        // 上下边界检测
        if transform.position.y - radius < bounds_min.y {
            collision_normal.y = 1.0;
            transform.position.y = bounds_min.y + radius;
        } else if transform.position.y + radius > bounds_max.y {
            collision_normal.y = -1.0;
            transform.position.y = bounds_max.y - radius;
        }

        // 发生碰撞时执行反弹逻辑
        if collision_normal != Vec2::ZERO {
            // 速度沿法线反射，实现反弹
            velocity.linear = velocity.linear - 2.0 * velocity.linear.dot(collision_normal) * collision_normal;
            // 应用阻尼，减少能量损耗，避免无限反弹
            velocity.linear *= config.ship_bounce_damping;
        }
    }

    // 销毁出界的子弹，避免内存泄漏
    let mut out_of_bounds_bullets = Vec::new();
    for (entity, transform) in world.query::<&Transform>()
        .with::<Bullet>()
        .iter()
    {
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
    // 收集所有碰撞体信息，避免迭代中修改世界
    let mut colliders = Vec::new();
    for (entity, (transform, collider, faction)) in world.query::<(&Transform, &Collider, Option<&FactionComponent>)>().iter() {
        colliders.push((entity, transform.position, collider, faction));
    }

    // 两两碰撞检测，实体数量少，无性能压力
    for i in 0..colliders.len() {
        let (entity_a, pos_a, collider_a, faction_a) = colliders[i];
        for j in (i + 1)..colliders.len() {
            let (entity_b, pos_b, collider_b, faction_b) = colliders[j];

            // 先判断碰撞层级是否允许碰撞
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
                    if !fa.faction.is_enemy(&fb.faction) {
                        continue;
                    }
                }
            }

            // 圆形碰撞检测，计算距离
            let distance = pos_a.distance(pos_b);
            let min_collision_distance = collider_a.radius + collider_b.radius + config.collision_margin;

            // 发生碰撞
            if distance < min_collision_distance {
                // 子弹命中飞船
                if collider_a.layer == CollisionLayer::Bullet && collider_b.layer == CollisionLayer::Ship {
                    if let Ok(bullet) = world.get::<Bullet>(entity_a) {
                        // 获取子弹伤害
                        let damage = world.get::<Weapon>(bullet.shooter)
                            .map(|w| w.bullet_damage)
                            .unwrap_or(config.bullet_damage);
                        // 发布命中事件
                        event_bus.publish(GameEvent::Hit {
                            attacker: bullet.shooter,
                            target: entity_b,
                            damage,
                            position: pos_a,
                        });
                        // 销毁命中的子弹
                        let _ = world.despawn(entity_a);
                    }
                }
                // 飞船被子弹命中
                else if collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Bullet {
                    if let Ok(bullet) = world.get::<Bullet>(entity_b) {
                        let damage = world.get::<Weapon>(bullet.shooter)
                            .map(|w| w.bullet_damage)
                            .unwrap_or(config.bullet_damage);
                        event_bus.publish(GameEvent::Hit {
                            attacker: bullet.shooter,
                            target: entity_a,
                            damage,
                            position: pos_b,
                        });
                        let _ = world.despawn(entity_b);
                    }
                }
                // 飞船间碰撞，简单反弹
                else if collider_a.layer == CollisionLayer::Ship && collider_b.layer == CollisionLayer::Ship {
                    let collision_normal = (pos_a - pos_b).normalize_or_zero();
                    // 给两个飞船施加反向的速度，实现碰撞反弹
                    if let Ok(mut vel_a) = world.get_mut::<Velocity>(entity_a) {
                        vel_a.linear += collision_normal * 0.5;
                    }
                    if let Ok(mut vel_b) = world.get_mut::<Velocity>(entity_b) {
                        vel_b.linear -= collision_normal * 0.5;
                    }
                }
            }
        }
    }
}
