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
    // 修复：使用entity变量而不是_忽略
    for (entity, (transform, velocity, mut ai_state, health, faction)) in world.query::<(
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

        // ========== 行为执行逻辑 ==========
        match ai_state.current_state {
            AiBehaviorState::Idle => {
                // 空闲状态：缓慢随机移动
                velocity.linear *= 0.95;
            }
            AiBehaviorState::Seeking => {
                // 索敌状态：随机巡逻
                let wander_angle = rand_random() * std::f32::consts::TAU;
                velocity.linear = Vec2::new(wander_angle.cos(), wander_angle.sin()) * config.ship_max_speed * 0.3;
            }
            AiBehaviorState::Chasing => {
                // 追击状态：朝目标移动
                if let Some((_, target_pos, _, _)) = target_info {
                    let direction = (*target_pos - transform.position).normalize_or_zero();
                    velocity.linear = direction * config.ship_max_speed;
                }
            }
            AiBehaviorState::Attacking => {
                // 攻击状态：保持距离并瞄准
                if let Some((_, target_pos, _, _)) = target_info {
                    let direction = (*target_pos - transform.position).normalize_or_zero();
                    let distance = transform.position.distance(*target_pos);
                    
                    if distance < config.ai_attack_range * 0.5 {
                        // 太近，后退
                        velocity.linear = -direction * config.ship_max_speed * 0.3;
                    } else {
                        // 绕目标旋转
                        let perpendicular = Vec2::new(-direction.y, direction.x);
                        velocity.linear = perpendicular * config.ship_max_speed * 0.5;
                    }
                }
            }
            AiBehaviorState::Retreating => {
                // 撤退状态：远离最近的敌人
                let mut nearest_enemy = None;
                let mut nearest_distance = f32::INFINITY;

                for (target_entity, target_pos, target_faction, _) in &all_ships {
                    if *target_entity == entity || !faction.faction.is_enemy(target_faction) {
                        continue;
                    }
                    let distance = transform.position.distance(*target_pos);
                    if distance < nearest_distance {
                        nearest_distance = distance;
                        nearest_enemy = Some(*target_pos);
                    }
                }

                if let Some(enemy_pos) = nearest_enemy {
                    let away_direction = (transform.position - enemy_pos).normalize_or_zero();
                    velocity.linear = away_direction * config.ship_max_speed;
                }
            }
        }
    }
}

/// 简单的随机数生成
fn rand_random() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f32 / u32::MAX as f32
}
