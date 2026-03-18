use glam::Vec2;
use hecs::World;
use std::time::Duration;

use crate::config::GameConfig;
use crate::ecs::components::*;

// AI系统主入口，每一固定帧更新所有飞船的AI行为
pub fn ai_system(world: &mut World, dt: Duration, config: &GameConfig) {
    // 先收集所有存活飞船的信息，用于索敌逻辑
    let mut all_ships = Vec::new();
    for (entity, (transform, health, faction)) in world.query::<(&Transform, &Health, &FactionComponent)>()
        .iter()
    {
        if !health.is_dead {
            all_ships.push((entity, transform.position, faction.faction, health.current));
        }
    }

    // 遍历每个AI飞船，更新状态机和行为
    for (_, (transform, velocity, mut ai_state, health, faction)) in world.query::<(
        &Transform,
        &mut Velocity,
        &mut AiState,
        &Health,
        &FactionComponent,
    )>()
    .iter()
    {
        if health.is_dead {
            continue;
        }

        // 更新目标锁定计时器，防止目标频繁切换
        if ai_state.target_lock_timer > Duration::ZERO {
            ai_state.target_lock_timer = ai_state.target_lock_timer.saturating_sub(dt);
        }

        // ========== 索敌逻辑：找到最优攻击目标 ==========
        let mut best_target = None;
        let mut best_score = f32::INFINITY;
        for (target_entity, target_pos, target_faction, target_health) in &all_ships {
            // 跳过自己和友军
            if *target_entity == entity || !faction.faction.is_enemy(target_faction) {
                continue;
            }
            // 超出视野范围，跳过
            let distance = transform.position.distance(*target_pos);
            if distance > config.ai_view_range {
                continue;
            }
            // 计算目标评分：距离越近、血量越低，评分越高（数值越小）
            let score = distance + (target_health / health.max) * 10.0;
            if score < best_score {
                best_score = score;
                best_target = Some((*target_entity, *target_pos));
            }
        }

        // 更新锁定目标：只有锁定时间结束才会切换，防止目标频繁抖动
        if ai_state.target_lock_timer.is_zero() {
            ai_state.target = best_target.map(|(e, _)| e);
            if ai_state.target.is_some() {
                ai_state.target_lock_timer = Duration::from_secs_f32(config.ai_target_lock_time);
            }
        }

        // 获取当前目标的信息
        let target_info = ai_state.target.and_then(|e| {
            all_ships.iter().find(|(ent, _, _, _)| *ent == e)
        });

        // ========== 状态机切换逻辑 ==========
        let health_percent = health.current / health.max;
        // 血量低于阈值，进入撤退状态
        if health_percent <= config.ai_evade_threshold {
            ai_state.current_state = AiBehaviorState::Retreating;
        }
        // 有目标，进入攻击/追击状态
        else if let Some((_, target_pos, _, _)) = target_info {
            let distance = transform.position.distance(*target_pos);
            ai_state.current_state = if distance <= config.ai_attack_range {
                AiBehaviorState::Attacking
            } else {
                AiBehaviorState::Chasing
            };
        }
        // 无目标，进入索敌状态
        else {
            ai_state.current_state = AiBehaviorState::Seeking;
        }

        // ========== 每个状态的具体行为逻辑 ==========
        let mut desired_velocity = Vec2::ZERO;
        let mut desired_rotation = transform.rotation;
        ai_state.should_fire = false;

        match ai_state.current_state {
            // 索敌状态：向战场中心移动，寻找目标
            AiBehaviorState::Seeking => {
                let world_center = Vec2::new(config.world_width / 2.0, config.world_height / 2.0);
                let to_center = world_center - transform.position;
                desired_velocity = to_center.normalize_or_zero() * config.ship_max_speed * 0.5;
                desired_rotation = desired_velocity.y.atan2(desired_velocity.x);
            }

            // 追击状态：向目标移动，进入攻击范围
            AiBehaviorState::Chasing => {
                if let Some((_, target_pos, _, _)) = target_info {
                    let to_target = *target_pos - transform.position;
                    let distance = to_target.length();
                    let desired_distance = config.ai_attack_range * 0.8;

                    // 距离过远，全速追击
                    desired_velocity = if distance > desired_distance {
                        to_target.normalize_or_zero() * config.ship_max_speed
                    } else {
                        Vec2::ZERO
                    };
                    // 面向目标
                    desired_rotation = to_target.y.atan2(to_target.x);
                }
            }

            // 攻击状态：预判目标位置，开火攻击，灵活机动
            AiBehaviorState::Attacking => {
                if let Some((target_entity, target_pos, _, _)) = target_info {
                    let to_target = *target_pos - transform.position;
                    let distance = to_target.length();

                    // 获取目标速度，预判攻击位置
                    let target_velocity = world.get::<Velocity>(target_entity)
                        .map(|v| v.linear)
                        .unwrap_or(Vec2::ZERO);
                    // 计算子弹飞行时间，预判目标位置
                    let bullet_speed = config.ship_max_speed * config.bullet_speed_multiplier;
                    let time_to_hit = distance / bullet_speed;
                    let predicted_pos = *target_pos + target_velocity * time_to_hit;
                    let to_predicted = predicted_pos - transform.position;

                    // 面向预判位置
                    desired_rotation = to_predicted.y.atan2(to_predicted.x);
                    ai_state.desired_direction = to_predicted.normalize_or_zero();

                    // 机动逻辑：保持最优攻击距离，横向走位
                    let desired_distance = config.ai_attack_range * 0.6;
                    desired_velocity = if distance > desired_distance {
                        // 距离过远，靠近目标
                        to_target.normalize_or_zero() * config.ship_max_speed * 0.7
                    } else if distance < desired_distance * 0.5 {
                        // 距离过近，后退
                        -to_target.normalize_or_zero() * config.ship_max_speed * 0.5
                    } else {
                        // 距离合适，横向走位，规避子弹
                        let perpendicular = Vec2::new(-to_target.y, to_target.x).normalize_or_zero();
                        perpendicular * config.ship_max_speed * 0.3
                    };

                    // 开火判断：角度偏差小，目标存活
                    let angle_diff = (transform.forward().angle_between(to_predicted)).abs();
                    ai_state.should_fire = angle_diff < 0.2;
                }
            }

            // 撤退状态：远离最近的敌人，寻找安全位置
            AiBehaviorState::Retreating => {
                // 找到最近的敌人
                let mut nearest_enemy_pos = None;
                let mut min_distance = f32::INFINITY;
                for (_, pos, target_faction, _) in &all_ships {
                    if faction.faction.is_enemy(target_faction) {
                        let distance = transform.position.distance(*pos);
                        if distance < min_distance {
                            min_distance = distance;
                            nearest_enemy_pos = Some(*pos);
                        }
                    }
                }

                if let Some(enemy_pos) = nearest_enemy_pos {
                    // 远离敌人
                    let away_from_enemy = transform.position - enemy_pos;
                    // 计算安全位置，不超出边界
                    let safe_pos = Vec2::new(
                        transform.position.x + away_from_enemy.x.signum() * 20.0,
                        transform.position.y + away_from_enemy.y.signum() * 20.0,
                    ).clamp(
                        Vec2::splat(config.ship_size * 2.0),
                        Vec2::new(config.world_width, config.world_height) - Vec2::splat(config.ship_size * 2.0),
                    );
                    let to_safe = safe_pos - transform.position;

                    // 全速向安全位置移动
                    desired_velocity = to_safe.normalize_or_zero() * config.ship_max_speed;
                    desired_rotation = desired_velocity.y.atan2(desired_velocity.x);
                }
            }

            _ => {}
        }

        // ========== 应用计算好的速度和旋转 ==========
        velocity.linear = desired_velocity;
        // 平滑转向，避免瞬间掉头
        let rotation_diff = desired_rotation - transform.rotation;
        let rotation_diff = rotation_diff.rem_euclid(std::f32::consts::TAU);
        let rotation_diff = if rotation_diff > std::f32::consts::PI {
            rotation_diff - std::f32::consts::TAU
        } else {
            rotation_diff
        };
        velocity.angular = rotation_diff.clamp(-config.ship_turn_speed, config.ship_turn_speed);
    }
}
